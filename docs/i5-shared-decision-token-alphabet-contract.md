# I5 Shared Decision-Token Alphabet Contract

> **Experimental-frontend note.** This document was written inside the private
> TCT research repository and is kept as a contract reference for the
> experimental non-JSON frontends, which are compiled only with the
> `experimental` Cargo feature. It may cite roadmap, report, or
> sibling-repository documents that are not part of this repository; treat
> those citations as historical context. The stable JSON Schema core is
> documented in `README.md`, `ARCHITECTURE.md`, and `DEVELOPMENT.md`.

Document path: `docs/i5-shared-decision-token-alphabet-contract.md`.

Status: **CPU checklist complete / implemented on `main`** (tranches 1–12 —
field-relative choice, generic encode inverses, mask admissibility, JSON/C
subset SharedAlphabet round-trips, held-out rust constructed-tree under
SharedAlphabet, multi-grammar same-space witness, layout-diff gate; overnight
tip includes `6ece558c1`). Owner: maintainer-owned shared token-layout / raw
SM path. This status line is not a paper claim and does not register a
production route. Default layout mode remains PerGrammar.

This document is the design contract for a **universal decision-token alphabet**:
every grammar compiles into the **same** raw token space by mapping each branch
point to a shared "choice *k* of *n*" token, plus shared structural brackets and
shared payload/content bands. It is the **compiler** half of the I5 program —
well-formedness, lossless encode/decode, collision-freedom, and a bounded
alphabet independent of how many grammars compile into it.

It builds on:

- `docs/generic-codec-layer-contract.md` (`generic_codec_layer_contract_v0`) —
  language-neutral encode/decode already driven from `StructuralModel` /
  `CompiledSyntaxModel`; a shared alphabet is therefore a **layout** change,
  not a codec rewrite.
- `docs/shared-sm-mixed-payload-contract.md`
  (`shared_sm_mixed_payload_contract_v1`) — discriminated alternatives and
  state-gated choice resolution that this alphabet generalizes across grammars.
- Existing layout machinery: `GrammarStructuralTokenLayout`,
  `build_decision_graph`, `validate_token_layout`
  (`src/grammar_ir_structural_model.rs`); range assignment and the Go
  pinned-table guard (`src/syntax_token_layout.rs`);
  `mixed_field_choice_for_token` / `node_kind_for_alternative_token`
  (`src/syntax_state_machine/raw_replay.rs`);
  `ensure_mixed_field_choice_tokens` (`src/syntax_state_machine.rs`);
  `ModelFacts::alternatives_by_field` (`src/syntax_state_machine/model_facts.rs`).

It does **not** train models, measure transfer or interference, register a
production route, add a per-language subset codec, or edit
downstream repositories.

Claim id for this document:

```text
i5_shared_decision_token_alphabet_contract_v0
```

Claim boundary string (must appear on every compiler-side evidence report or
test assertion that claims this contract):

```text
compiler_shared_alphabet_layout_only
```

## Surfaces and claim boundary

| Surface | Role |
| --- | --- |
| Metadata lower | Pin → `GrammarIR` → `StructuralModel` is already target-neutral for Tier A held-out ingestion (CPU-5; `TreeSitterGrammarPin::rust_fixture_dir`) |
| Generic encode/decode | `CompiledSyntaxModel::encode_complete_tree` / `decode_complete_tree` already model-faithful (`src/syntax_state_machine/encode.rs`, `src/syntax_state_machine.rs`) |
| Layout today | **Per-grammar.** `GrammarStructuralTokenLayout` + `node_start_alternative_token_ids_from_sidecar` assign structural ranges and alternative node_start IDs per `StructuralModel` / grammar id (Go pinned table; alphabetical free-slot fallback otherwise) |
| Mixed choice today | Shared SM admits mixed fields; choice resolution is **state-gated** by active `(rule, field)` (`mixed_field_choice_for_token`) before reverse lookup |
| This contract | Shared alphabet structure + compile map + decodability invariant + selectable layout mode + CPU implementation checklist |
| Out of scope | Model train, transfer/interference, paper claims, production routes, new hand-built subset codecs, GPU work, agentic tool-call work, downstream-repository edits |

**CPU-provable and in scope** (once implemented):

- Multiple grammars compile into one fixed shared layout (JSON subset, C subset,
  held-out rust pin as constructed-tree witnesses).
- Encode/decode round-trip **within** the shared space for those models
  (constructed trees; subset source round-trips where codecs already exist).
- Collision-freedom of role ranges inside the shared layout.
- Alphabet size bounded by compile-time constants, independent of grammar count.
- Layout-diff / token-diff evidence classifying the shared mode relative to
  per-grammar mode (`breaking_change` for absolute streams that switch modes;
  default per-grammar pins remain `exact_reuse` when mode is not selected).

**GPU / NOT claimable here:**

- Whether polysemy of a shared decision token transfers across grammars or
  interferes.
- Semantic loss, compression, model quality, or any paper result.
- Pretrain / finetune / held-out-grammar transfer measurements (that is the
  separate I5 **transfer experiment** in the research roadmap Investigation Queue — not
  this contract).

## Problem

Today every grammar gets its own token layout. `GrammarStructuralTokenLayout`
and `node_start_alternative_token_ids_from_sidecar` assign structural ranges
and alternative node_start tokens **per** `StructuralModel` / grammar id:

- Go resolves multi-alt node kinds through the global pinned table in the
  checked inventory (`go_pinned_alternative_node_start_tokens`).
- Toy, C#, and tree-sitter subsets use alphabetical free-slot assignment inside
  that grammar's `structural_decisions` band.
- `ensure_mixed_field_choice_tokens` extends the same per-grammar map for mixed
  payload alternatives (and hard-errors on Go rather than inventing free slots).

Consequence: two grammars produce **disjoint, incomparable** token spaces. A
model pretrained on grammar A learns nothing reusable about grammar B at the
structural-decision level, and every new grammar needs its own training run.
That is the adoption blocker the I5 program targets.

The generic codec layer closed the encode gap: encode and decode already read
behavior from the model. A shared alphabet is therefore a **layout-mode**
change on top of an already language-neutral codec, not a second codec.

## Current layers (layout column)

| Layer | Shared today? | Notes |
| --- | --- | --- |
| Adapter → GrammarIR | Yes (Tier A) | Held-out rust needs zero language-keyed classification Rust |
| GrammarIR → StructuralModel | Yes | `build_decision_graph`, `validate_token_layout`, finite domains |
| Per-grammar token layout | **No** | Ranges + alternative node_start assignment are grammar-local |
| Raw SM roles | Grammar-local | `SyntaxTokenRoles` derived from that layout |
| Raw SM encode/decode | Model-driven | Generic; walks `ModelFacts` + roles |
| Subset absolute goldens | Per-subset streams | `tree_sitter_json_subset_absolute_token_id_golden_pins`, `tree_sitter_c_subset_absolute_token_id_golden_pins` |
| Go / C# layout pins | Absolute | Layout-diff gate; Go pinned alternative table |

## Contract goals

1. **One alphabet** — a single fixed token space that any supported
   `StructuralModel` can compile into.
2. **Bounded size** — alphabet cardinality is a function of compile-time
   constants only (max branch arity, fixed structural offsets, fixed payload
   bands). It must **not** grow when a new grammar is added.
3. **Lossless round-trip** — for model-conforming trees under the shared layout,
   `encode_complete_tree` and `decode_complete_tree` remain inverses (same
   consumed-token accounting as the generic codec contract).
4. **State-gated decodability** — a shared choice token is unambiguous only
   relative to the SM state at that position (explicit invariant below).
5. **Fail closed** — arity overflow, arity mismatch at decode, and unknown
   ordinals error with rule, field, arity, and ordinal context; never invent
   free-slot tokens that expand the shared pool per grammar.
6. **Pin safety** — default production/pin paths stay on per-grammar layout;
   shared mode is selectable and does not silently rewrite Go/C# pins or
   subset absolute goldens.
7. **No silent layout drift** — any path that selects shared mode and changes
   absolute streams must go through the syntax token-layout diff gate and
   `tct-token-diff` class reporting per `DEVELOPMENT.md` / `DEVELOPMENT.md`.

---

## Decision 1 — Alphabet structure

### Decision (load-bearing)

The shared alphabet has four fixed classes. Choice tokens are **arity-indexed
ordinals**, not a flat undifferentiated pool and not a per-grammar free-slot
scan.

| Class | Contents | Cardinality source |
| --- | --- | --- |
| Structural brackets | Fixed role offsets already used today: `node_start`, `node_end`, `field_start`, `field_end`, `field_missing`, `repeated_empty`, `terminal`, `payload_end`, `boolean_false`, `boolean_true` (offsets `0..RAW_TOKEN_ALTERNATIVE_NODE_START_OFFSET`, i.e. `0..10`) | Compile-time constants (`RAW_*_OFFSET`) |
| Decision / choice | `CHOICE(n, k)` for each arity `n ∈ 2..=MAX_BRANCH_ARITY` and ordinal `k ∈ 0..n` | `sum_{n=2}^{MAX} n = MAX·(MAX+1)/2 − 1` |
| Payload content | Shared `utf8_byte_payload` (256) and `u8_integer_payload` (256) bands | Fixed band widths |
| Pad | One `PAD_TOKEN` at end of raw base | Fixed |

**Packing of `CHOICE(n, k)`** inside the structural decision band, starting at
`RAW_TOKEN_ALTERNATIVE_NODE_START_OFFSET` (10):

```text
choice_base(n) = RAW_TOKEN_ALTERNATIVE_NODE_START_OFFSET
                 + (n * (n - 1) / 2 - 1)     // for n >= 2
token_id(CHOICE(n, k)) = structural_range.start + choice_base(n) + k
// with 0 <= k < n
```

`choice_base(2) = 10`, tokens for arity 2 occupy offsets `10, 11`; arity 3
occupies `12, 13, 14`; and so on. Invert by scanning triangular bases or an
equivalent closed form. Implementation may use a denser packing as long as
`(n, k) ↔ token` is bijective inside the reserved band and stable across runs.

**Default `MAX_BRANCH_ARITY`:** `40` (census-settled; design sketch used 32).

| Quantity | Value at MAX=40 |
| --- | --- |
| Choice tokens | `40·41/2 − 1 = 819` |
| Bracket roles | 10 |
| Structural band minimum | `10 + 819 = 829` slots (before any future reserved roles) |
| utf8 + u8 integer | 256 + 256 |
| Illustrative raw base (brackets+choice+payloads, exclusive of PAD) | order-of-magnitude ~1340 before PAD / BPE reservation |

Exact `vocab_size` / range start addresses are fixed by one shared layout
constructor (checklist tranche 2, after the arity census); the load-bearing
property is that **no per-grammar free-slot allocation** contributes to size.

### Why arity-indexed, not a flat pool?

1. **Decode arity check.** Receiving `CHOICE(3, 1)` when the open field has
   arity 2 is an immediate reject. A flat `CHOICE_k` pool cannot detect
   "wrong arity, right ordinal" without carrying arity elsewhere.
2. **Bound independent of grammars.** Size is `O(MAX²)`, not
   `O(Σ node_kinds across all grammars)`.
3. **Matches the decision semantics.** At a branch point the machine chooses
   among *n* alternatives; the token names that decision, not a global
   node-kind identity.

### Why not grow the pool when a grammar needs more arity?

Growing the shared pool per grammar reintroduces per-grammar vocabulary drift
and breaks the premise that a model pretrained on the fixed alphabet already
"knows" every new grammar's tokens. **Arity overflow hard-errors at compile
time** (Decision 2). Raising `MAX_BRANCH_ARITY` is a deliberate alphabet
revision (layout-diff / breaking for shared-mode consumers), not a silent
per-grammar spill.

### Arity census (checklist tranche 1) — **done**

Settled before any shared layout constant is written. **Evidence:** integration
test `tests/i5_shared_alphabet_arity_census_test.rs` (report under
`target/tct-i5-shared-alphabet-arity-census.json`; claim boundary
`compiler_shared_alphabet_layout_only`). Method:
`max(finite_domain.alternatives.len())` for multi-alt fields after
GrammarIR → StructuralModel lower.

| Surface | Max multi-alt arity | Driver (example) |
| --- | --- | --- |
| Go inventory | 22 | Expr-family fields (e.g. `ArrayType.Elt`) |
| C# layout (all checked metadata fixtures) | 7 | `BlockSyntax.Statements` |
| JSON subset codec | 7 | `_value.subtype` |
| C subset codec | 2 | `_declarator.subtype` |
| tree-sitter-rust held-out pin | **40** | `_expression.subtype` |
| **Overall** | **40** | rust held-out pin |

**Decision:** design sketch `MAX_BRANCH_ARITY = 32` is **insufficient**. Documented
bump to **`MAX_BRANCH_ARITY = 40`** (exact measured overall max). Layout-diff
note: no shared layout constant existed yet; tranche 2 bakes 40. This is **not**
a default per-grammar pin change (Decision 4). Do not invent spill tokens;
further raises remain deliberate alphabet revisions.

---

## Decision 2 — The compile map

### Decision (load-bearing)

Compiling a `StructuralModel` into the shared alphabet is a pure function:

```text
shared_layout_roles(StructuralModel, SharedAlphabetConstants)
  -> SyntaxTokenRoles   // same role struct encode/decode already consume
```

It does **not** invent a second encode/decode walker. Encode continues to emit
whatever `roles.node_start_alternative_tokens` / plain `node_start` the roles
table provides; only the **assignment** of those IDs changes under shared mode.

### Mapping rules

1. **Structural brackets** — always the shared fixed offsets inside the shared
   `structural_decisions` range (same `RAW_*_OFFSET` semantics as today).
2. **Single-alternative Nonterminal fields** — child open uses plain
   `roles.node_start` (no choice token). Matches current single-alt behavior.
3. **Root open** — always plain `roles.node_start`. Root is never an
   alternative choice (same as `docs/generic-codec-layer-contract.md` §B).
4. **Multi-alternative fields** (homogeneous Nonterminal multi-alt **and**
   mixed payload fields listed in `ModelFacts::mixed_payload_fields`):
   - Let `n = alternatives.len()` for that field under
     `ModelFacts::alternatives_by_field` (or the finite-domain alternative
     list for homogeneous multi-alt fields that already drive
     `multi_alternative_nonterminal_fields`).
   - If `n < 2`: not a choice field.
   - If `n > MAX_BRANCH_ARITY`: **compile error** naming owner rule, field,
     arity, and the constant. No grow, no spill, no silent truncation.
   - Order alternatives by a **domain-stable total order** (below).
   - Map alternative at ordinal `k` to `CHOICE(n, k)`.
   - Populate `roles.node_start_alternative_tokens[alt.node_kind] = token_id`
     **for that compilation**. Under shared mode this map is still a
     compile-time table per model, but token IDs are drawn only from the shared
     `CHOICE` band, not from a free-slot scan.
5. **Payload bands** — shared utf8 and u8 integer ranges; no per-grammar
   relocation.
6. **Homogeneous boolean payloads** that still use `boolean_true` /
   `boolean_false` roles keep those fixed shared IDs (subset homogenization
   path unchanged unless a separate layout campaign de-homogenizes).

### Domain-stable ordinal assignment

Ordinals must be stable across runs and independent of `HashMap` iteration.

**Order key (required):** sort each field's alternatives by
`(node_kind, terminal_class_name)` ascending, where `terminal_class_name` is
the same class string `ModelFacts` already uses to distinguish Nonterminal
from each payload class (`model_facts.rs` terminal class name helper).

**Rationale:**

- `children_by_field` already sorts by `node_kind`.
- Non-Go alternative assignment today enumerates a `BTreeSet` of node kinds
  (alphabetical).
- `alternatives_by_field` currently preserves `finite_domains` / GrammarIR
  order; that order is pin-stable but not as easy to re-derive from a
  reconstructed model. Explicit sort makes the compile map a function of the
  alternative **set**, not of accidental pin array order.

**Open question:** whether any retained pin's absolute multi-alt order (under
per-grammar mode) intentionally disagrees with alphabetical
`(node_kind, class)` order in a way that shared-mode experiments care about.
Shared mode does not need to match per-grammar absolute IDs (Decision 4);
ordinal stability within shared mode is the requirement. Evidence if disputed:
diff domain order vs sorted order on Go/JSON/C finite domains.

### What about global node-kind → one token (today's shape)?

Today, within one grammar, the same `node_kind` reuses one alternative
node_start token everywhere it appears as a multi-alt child. Under shared
mode, **the same node_kind may map to different `CHOICE(n, k)` on different
fields** (different arities or different ordinals). That is intentional:

- The token names the **decision at this branch point**, not a global type id.
- Decode never recovers meaning from the token alone (Decision 3).

If a single field lists the same `node_kind` twice, construction already
rejects that identity (`ambiguous_alternative` /
duplicate `(node_kind, class)`). Shared mode inherits that rejection.

### Arity overflow policy (summary)

| Policy | Verdict |
| --- | --- |
| Grow shared pool | **Forbidden** — breaks bounded shared premise |
| Spill to grammar-local free slots | **Forbidden** — reintroduces per-grammar IDs |
| Hard error | **Required** |

---

## Decision 3 — Decodability invariant

### Invariant (load-bearing)

```text
I-DECODE-SHARED:
  A token t in the shared CHOICE band is unambiguously interpretable
  if and only if the raw SM has an active stack frame with an open field
  F = (rule, field) whose compile-time alternative table A_F has arity n,
  t decodes to some (n', k) with n' == n and 0 <= k < n, and A_F[k] is a
  legal alternative of F.

  The interpretation is A_F[k] (node_kind + body kind + target/payload).
  Without F, t has no structural meaning.
```

### What makes it hold (same terms as today's mixed path)

Today `mixed_field_choice_for_token` (`raw_replay.rs`) already:

1. Requires non-empty `mixed_payload_fields` (else `None`).
2. Requires a stack frame with an open field that is not
   missing/repeated-empty and not over-cardinality for a single-valued field.
3. Requires `(frame.rule, field.name)` ∈ `mixed_payload_fields`.
4. Only then maps `token → node_kind` and looks up
   `mixed_field_alternative(rule, field, node_kind)`.

Homogeneous multi-alt resolution similarly requires the open field and only
then accepts an alternative-specific node_start
(`resolve` path around `multi_alternative_nonterminal_fields` in
`raw_replay.rs`).

Shared mode keeps that **state gate** and strengthens the reverse map:

| Step | Today (per-grammar) | Shared mode |
| --- | --- | --- |
| State gate | Active `(rule, field)` | Same |
| Token → identity | Global reverse over `node_start_alternative_tokens` (`node_kind_for_alternative_token`) | Decode `(n, k)` from token; require field arity `n`; take `A_F[k]` |
| Body dispatch | `AlternativeBodyKind` / `apply_node_start` | Same body kinds |

The global reverse map `token → node_kind` is **not** a sound identity under
shared mode, because distinct fields deliberately reuse the same `CHOICE(n, k)`
for unrelated node kinds. Any decode path that reverse-looks up node_kind from
token alone, without the open field, is a defect under this contract.

### Mask / admissibility

At a choice state for field F of arity n, the mask admits exactly the n tokens
`CHOICE(n, 0) .. CHOICE(n, n-1)` that correspond to legal alternatives of F
(not the entire CHOICE band, and not payload bytes). This is the shared-mode
analogue of today's alternative-token mask population.

### Shapes where the invariant does **not** hold

These are the highest-value negative results of this design:

1. **Global reverse-map decode (the primary hazard).**  
   `node_kind_for_alternative_token` as a standalone identity is unsound under
   shared mode: the same token id is shared across fields and grammars. Any
   code path that uses that reverse map without the active `(rule, field)`
   violates `I-DECODE-SHARED`. **Required change:** choice resolution becomes
   field-relative (arity + ordinal into `A_F`), including for homogeneous
   multi-alt, not only mixed fields.

2. **Arity mismatch.**  
   Token decodes to `(n', k)` but open field arity is `n ≠ n'`. Reject; do not
   clamp ordinal into another arity's table.

3. **No open field / wrong phase.**  
   Choice token at root (before any field), inside an open payload substate,
   after `field_missing` / `repeated_empty`, or when a single-valued field is
   already full — same class of errors as today; choice tokens are not
   free-floating.

4. **Elided / mask-forced streams without reconstructed state.**  
   Canonical Go elided streams omit mask-forced tokens. Shared-mode
   decodability for this contract is stated for the **expanded** raw stream
   the SM accepts (same expanded-stream surface as
   `encode_complete_tree` / `decode_complete_tree` v0). Elided-stream
   encode/decode under shared mode is **out of scope for v0** unless a later
   tranche reuses the existing elided replay with the same state gate. Without
   full state reconstruction, a bare shared choice token is not self-describing.

5. **Cross-grammar token dump without a model.**  
   A `Vec<u32>` produced under grammar A's shared compilation cannot be
   decoded with grammar B's `ModelFacts` even though absolute ids live in the
   same alphabet: the **state machine and alternative tables** are still
   grammar-specific. The alphabet is shared; the language is not. This is not
   a bug — it is the boundary between compiler sharing and model transfer
   (GPU).

6. **Duplicate ordinal collision inside one field.**  
   Prevented at compile time by unique `(node_kind, class)` identities and
   bijective `k ↔ alternative` after sort. If construction allowed duplicate
   ordinals, decode would be ambiguous even with state; construction must keep
   rejecting duplicates.

### Encode side of the invariant

Encode under shared mode, for each multi-alt value on field F:

1. Resolve the alternative by node kind / terminal class (same as generic
   codec §D).
2. Find its ordinal `k` in the domain-stable ordered table `A_F`.
3. Emit `CHOICE(n, k)` then the body by `AlternativeBodyKind` (identical body
   accounting to the generic codec and mixed-payload contracts).

No free-slot invention at emit time (same determinism rule as generic codec
§ Consumed-token accounting).

---

## Decision 4 — Relationship to pinned layouts

### Decision (load-bearing): **(a) second, selectable layout mode**

Shared alphabet is a **selectable layout mode** alongside the existing
per-grammar layout. It is **not** a forced replacement of Go, C#, or subset
absolute pins.

| Mode | Who uses it | Pin / golden expectation |
| --- | --- | --- |
| `PerGrammar` (default) | Go inventory pins, C# layout pin, JSON/C subset absolute goldens, production-adjacent gates | Absolute IDs **unchanged**; pins stay green without edits |
| `SharedAlphabet` (opt-in) | Explicit tests, synthetic multi-grammar witnesses, future GPU experiment package construction | Streams use shared IDs; **do not** overwrite per-grammar pin fixtures |

Selection surface (names may adjust slightly in implementation):

```text
// Conceptual — compile-time / construction-time, not a runtime branch in hot encode
enum SyntaxTokenLayoutMode {
    PerGrammar,
    SharedAlphabet,
}
```

Fed into role construction (`SyntaxTokenRoles::from_layout` path /
`node_start_alternative_token_ids_from_sidecar` successor) so encode/decode
remain mode-agnostic walkers over roles + `ModelFacts`.

### Why (a), not (b)

1. **CPU-mergeable.** Checklist tranches can land on an exp branch and merge to
   `main` without a forced `tct-token-diff` `breaking_change` on every retained
   pin. That is the difference between a compiler campaign and a migration
   campaign.
2. **Generic codec already made encode/decode model-driven.** Mode only
   changes role assignment; JSON/C goldens keep exercising `PerGrammar`.
3. **DEVELOPMENT.md layout-diff gate.** Layout-affecting syntax changes must go
   through the syntax token-layout diff gate. Optional mode means the default
   path reports `exact_reuse`; shared-mode fixtures carry their own evidence.
4. **GPU experiment can opt in** by compiling corpora under `SharedAlphabet`
   without rewriting production Go BPE artifacts first.

### Cost of (b) replacement (explicitly rejected for v0)

- Go pinned alternative table and `GO_SYNTAX_LAYOUT_VOCAB_SIZE` / PAD constants
  become wrong for the only layout.
- C# token-layout pin and every absolute golden
  (`tree_sitter_json_subset_absolute_token_id_golden_pins`,
  `tree_sitter_c_subset_absolute_token_id_golden_pins`, I6 absolute vectors)
  break.
- Downstream BPE merge tables, embeddings, and raw-token corpora for Go syntax
  require retrain or explicit migration (`breaking_change`).
- Maintainer investigation required before any `main` merge; campaign is no
  longer a pure CPU feature add.

### Layout-diff obligations when shared mode is exercised

- Switching a **given** artifact from `PerGrammar` to `SharedAlphabet` is a
  stream-level `breaking_change` for consumers of that artifact's absolute
  tokens, even if role *names* are unchanged.
- Adding shared mode **without** flipping default pins is not a pin break;
  still record a range-level description of the shared layout constructor in
  tests.
- Any future proposal to make `SharedAlphabet` the default for a retained pin
  re-opens (b) costs and is out of this contract's implementation checklist.

---

## Decision 5 — Per construct class (shared vs pinned layouts)

| Construct class | Per-grammar mode (today / default) | Shared-alphabet mode | Stream vs pins if mode selected on subsets |
| --- | --- | --- | --- |
| Root open | Plain `node_start` | Same plain shared `node_start` | Absolute ids change with range base; structure same |
| Single-alt Nonterminal child | Plain `node_start` | Same | Same |
| Homogeneous multi-alt NT | Alt-specific node_start from per-grammar map / Go pin table | `CHOICE(n, k)` by domain-stable ordinal | **Diverges** — absolute choice ids differ; body shape same |
| Field-less leaf NT | `node_start` + `node_end` | Same brackets | Structure identical |
| Homogeneous IntegerPayload / Utf8 under NT wrapper | `terminal` + payload bytes + `payload_end` | Same roles in shared payload bands | Payload **byte** ids match if bands use the same widths/order; band **base** may differ if layout packing differs — pin vectors still break if mode flips |
| Optional missing | `field_missing` | Shared `field_missing` | Structure identical |
| Repeated empty | `repeated_empty` | Shared `repeated_empty` | Structure identical |
| Native mixed BooleanPayload | Choice only (`ensure_mixed_field_choice_tokens` + body kind) | `CHOICE(n, k)` only | Diverges in absolute choice ids; accounting identical |
| Native mixed Integer / Utf8 | Choice + payload bytes + `payload_end` (no generic `terminal`) | Same body; shared choice | Same |
| Native mixed Nonterminal sibling | Choice (= child open) + child body | `CHOICE(n, k)` + child body | Same |
| Go pinned alt table kinds | Exact inventory offsets | **Not used** under shared mode | N/A to default pins |
| C# layout pin | Expanded-contract pin | Not rewritten | Default pin stays on per-grammar |

**Default expectation for JSON/C absolute goldens under this campaign:** they
remain on `PerGrammar` and stay bit-identical. Shared-mode proofs use separate
assertions (role tables, round-trips under shared roles, multi-grammar
collision-freedom), not silent golden rewrites.

---

## Decision 6 — CPU / GPU claim boundary

| Claim surface | In this campaign? |
| --- | --- |
| Multiple grammars → one shared layout constructor | Yes (CPU) |
| Encode/decode inverse under shared layout (JSON, C, held-out rust constructed tree) | Yes (CPU) |
| Alphabet bound + arity overflow errors | Yes (CPU) |
| Field-relative choice resolution / arity mismatch rejects | Yes (CPU) |
| Layout-diff evidence for mode selection | Yes (CPU) |
| Cross-grammar transfer, interference, semantic loss | **No** (GPU transfer experiment) |
| Paper headline / claim profile updates | **No** |
| Production-route registration | **No** |

Claim id: `i5_shared_decision_token_alphabet_contract_v0`.  
Boundary: `compiler_shared_alphabet_layout_only`.

The Investigation Queue item **I5 — Shared Token Space + Held-Out-Grammar
Transfer (the decisive experiment)** remains the GPU arm. It consumes this
compiler alphabet when ready; it does not gate this design or its CPU
implementation checklist.

---

## Consumed-token accounting

Unchanged from `docs/generic-codec-layer-contract.md` and
`docs/shared-sm-mixed-payload-contract.md`. Shared mode only renumbers choice
tokens; it does not change how many tokens a construct class consumes.

| Class | Tokens for one field value (body) |
| --- | --- |
| Homogeneous Nonterminal (single-alt) | plain `node_start` + full child record |
| Homogeneous multi-alt Nonterminal | `CHOICE(n,k)` (= child open) + rest of child record |
| Homogeneous IntegerPayload | `terminal` + integer length + `payload_end` |
| Homogeneous Utf8 / IdentifierPayload | `terminal` + byte count + `payload_end` |
| Mixed Nonterminal | `CHOICE(n,k)` + rest of child body |
| Mixed BooleanPayload | `CHOICE(n,k)` only |
| Mixed Integer / Utf8 | `CHOICE(n,k)` + payload length + `payload_end` |
| field_missing / repeated_empty | 1 each |

Plus usual `field_start` / `field_end` / `node_end` brackets.

---

## Held-out and multi-grammar CPU proof shapes

### A. Single-grammar shared-mode inverse (JSON / C / rust)

For each of: tree-sitter JSON subset model, tree-sitter C subset model,
tree-sitter-rust held-out pin (`tests/fixtures/tree_sitter_metadata/rust/`,
`TreeSitterGrammarPin::rust_fixture_dir`):

1. Compile `CompiledSyntaxModel` under `SharedAlphabet`.
2. Construct (or subset-encode via language value → decoded node) a conforming
   tree.
3. Assert stream ↔ tree identities under shared roles.
4. Assert every emitted token `< pad_token` and `< vocab_size`.
5. Claim boundary string `compiler_shared_alphabet_layout_only` on the test.

Rust remains **constructed-tree only** (no parse/render; no tree-sitter
runtime dependency) — same hard boundary as the generic codec held-out proof.

### B. Multi-grammar same-space witness

Compile at least two models (e.g. JSON subset + C subset, or JSON + rust
constructed fragment) under the **same** shared layout constants. Assert:

- Identical `vocab_size`, `pad_token`, and range bases.
- Choice tokens used by both lie in the shared `CHOICE` band.
- No per-grammar free-slot ids outside that band.

Does **not** claim that a stream from model A decodes under model B.

### C. Default pins untouched

With mode defaulting to `PerGrammar`:

- `tree_sitter_json_subset_absolute_token_id_golden_pins`
- `tree_sitter_c_subset_absolute_token_id_golden_pins`
- Go / C# layout pin gates

stay green without fixture edits in every checklist tranche that does not
explicitly opt into shared mode.

---

## Implementation checklist (commit-sized tranches)

Ordered so each step can land independently. Each tranche
must build, pass its validation command, and be reviewable alone. **Design is
this document; implementation starts at tranche 1.**

1. **Arity census (audit) — done**  
   Measured max multi-alt arity on retained pins (Go inventory, C# layout,
   JSON/C subset models, tree-sitter-rust pin). Design default 32 was
   **insufficient** (rust held-out overall max **40**); documented bump to
   `MAX_BRANCH_ARITY = 40` in Decision 1.  
   Validate: `cargo test --features experimental --test i5_shared_alphabet_arity_census_test` (writes
   `target/tct-i5-shared-alphabet-arity-census.json`; not a paper claim).

2. **Shared alphabet constants + layout constructor (no SM wiring yet) — done**  
   Introduced `SHARED_ALPHABET_MAX_BRANCH_ARITY = 40` (census-settled),
   `CHOICE(n,k)` packing helpers, and `shared_alphabet_token_layout()` in
   `src/shared_alphabet_layout.rs` (sibling of `syntax_token_layout`). Unit
   tests: bijective pack/unpack for all `(n,k)`; ranges pass
   `GrammarStructuralTokenLayout::validate`; size independent of any grammar
   fixture. No SM wiring.  
   Validate: `cargo test --features experimental --lib shared_alphabet_layout`

3. **Layout mode enum + role assignment under SharedAlphabet — done**  
   `SyntaxTokenLayoutMode::{PerGrammar, SharedAlphabet}` in
   `src/shared_alphabet_layout.rs`. PerGrammar path bit-identical to today's
   `node_start_alternative_token_ids_from_sidecar` /
   `ensure_mixed_field_choice_tokens` behavior (`from_sidecar` defaults to
   PerGrammar). SharedAlphabet path uses `shared_alphabet_token_layout()` and
   fills `node_start_alternative_tokens` from domain-stable
   `(node_kind, terminal_class)` ordinals via `CHOICE(n,k)`; arity overflow
   hard-errors. Choice-token uniqueness is PerGrammar-only (shared mode reuses
   the CHOICE band across fields).  
   Validate: `cargo test --lib shared_alphabet_role_assignment` (arity 2 and 3,
   two-field CHOICE reuse, overflow at `MAX+1`, PerGrammar bit-identity).
   Boundary e2e: `cargo test --features experimental --test i5_shared_alphabet_arity_boundary_test`
   (arity exactly 40 encode/decode for every ordinal + `MAX+1` overflow
   contrast; `CHOICE(40,39) == STRUCTURAL_BAND_SIZE - 1`).

4. **Failing decode/encode tests for field-relative choice (TDD) — done**  
   Module `shared_alphabet_field_relative_choice_tests` in
   `src/syntax_state_machine.rs`. (a) decode reverse under open field; (b)
   encode field-local `CHOICE(n,k)`; (c) field-keyed table shape. Tranche 5
   turned the red pair green.  
   Validate: `cargo test --lib shared_alphabet_field_relative`.

5. **Field-relative choice resolution (encode + raw_replay) — done**  
   `SyntaxTokenRoles::field_choice_table` holds domain-stable
   `FieldChoiceAlternative` rows per `(rule, field)`. SharedAlphabet encode
   uses `alternative_token_for_field`; decode uses
   `node_kind_for_field_choice_token` (`I-DECODE-SHARED`: unpack `(n',k)`,
   require field arity match, take `A_F[k]`). Mask admission uses field rows
   under SharedAlphabet. Residual global `node_start_alternative_tokens` map
   remains first-wins (not a sound reverse identity). PerGrammar paths and
   default pins unchanged.  
   Validate: `cargo test --lib shared_alphabet_field_relative`;
   `cargo test --features experimental --lib syntax_state_machine`.

6. **Generic encode uses shared roles without special cases — done**  
   Confirmed `encode_complete_tree` is mode-agnostic over roles +
   `ModelFacts` (no language keys); only SharedAlphabet role assignment
   changes absolute choice IDs. Synthetic inverse module
   `shared_alphabet_generic_encode_inverse_tests` in
   `src/syntax_state_machine/encode.rs`: field-less leaf, multi-alt NT
   (`CHOICE(3,k)`), mixed-payload witness all body kinds under domain-stable
   `CHOICE(5,k)`. Claim boundary `compiler_shared_alphabet_layout_only`.  
   Validate: `cargo test --lib shared_alphabet_generic_encode`;
   `cargo test --features experimental --lib syntax_state_machine`.
   Mixed body-kind dispatch regression under SharedAlphabet (encode/decode
   inverse + mask/accept + consumed-token accounting per body kind):
   `cargo test --features experimental --test i5_shared_alphabet_mixed_payload_dispatch_test`
   (`i5_shared_alphabet_mixed_payload_body_kind_dispatch`).

7. **Mask admissibility under shared choice band — done**  
   At a multi-alt field of arity n, admissible set is exactly that field's
   `CHOICE(n, *)` tokens (not the entire CHOICE band, not payload bytes).
   Module `shared_alphabet_mask_admissibility_tests` in
   `src/syntax_state_machine.rs`: arity-3 exact mask, wrong-arity exclusion,
   field-local arity-2 vs arity-3, wrong-arity advance rejection, mixed
   CHOICE(5,*) mask (no generic terminal / no payload ranges), optional
   multi-alt = `field_missing ∪ CHOICE(2,*)`. Mask population was already
   field-row-driven in tranche 5; this tranche locks Decision 3 with focused
   tests.  
   Validate: `cargo test --lib shared_alphabet_mask` (6 pass);
   `cargo test --features experimental --lib syntax_state_machine`.
   Regression: mask/accept set equality on every prefix of valid streams —
   `cargo test --features experimental --test i5_shared_alphabet_mask_accept_consistency_test`
   (`i5_shared_alphabet_mask_accept_consistency_json_and_c`; JSON + C under
   SharedAlphabet and PerGrammar baseline).

8. **JSON subset shared-mode round-trip (opt-in) — done**  
   Compile JSON subset under SharedAlphabet; language value → decoded node →
   encode/decode inverse. **Do not** edit absolute golden pins.  
   `value_to_decoded_node` is public for the opt-in path;
   `tree_sitter_json_subset_shared_alphabet_language_value_round_trip` locks
   stream↔tree inverses, `CHOICE(7,2)` for null, and PerGrammar golden
   bit-identity.  
   Validate: `cargo test --features experimental --test tree_sitter_json_subset_codec_test`
   (includes golden pins on default mode).

9. **C subset shared-mode round-trip (opt-in) — done**  
   Compile C subset under SharedAlphabet; program → decoded node →
   encode/decode inverse. **Do not** edit absolute golden pins.  
   `program_to_decoded_node` is public for the opt-in path;
   `tree_sitter_c_subset_shared_alphabet_program_round_trip` locks
   stream↔tree inverses, `CHOICE(2,0)` / `CHOICE(2,1)` for
   `_declarator.subtype` (function_declarator / identifier), and PerGrammar
   minimal golden bit-identity.  
   Validate: `cargo test --features experimental --test tree_sitter_c_subset_codec_test`
   (includes golden pins on default mode).

10. **Held-out rust constructed-tree under shared mode — done**  
    Reuse rust fixture path + generic encode/decode under
    `SyntaxTokenLayoutMode::SharedAlphabet`. Claim id
    `i5_shared_alphabet_held_out_rust_constructed_tree_v0` with boundary
    `compiler_shared_alphabet_layout_only`. Test
    `tree_sitter_rust_held_out_constructed_tree_shared_alphabet_encode_decode`
    in `tests/tree_sitter_metadata_adapter_test.rs` (beside the generic codec
    held-out proof): field-less root, nested multi-alt, `CHOICE(2,1)` for
    `negative_literal`→`integer_literal`, lifetime utf8 payload in the shared
    band; tokens `< PAD/VOCAB`; default PerGrammar path unchanged. No rust
    codec/corpus; no pin edits.  
    Validate: `cargo test --features experimental --test tree_sitter_metadata_adapter_test` (20 pass).

11. **Multi-grammar same-space witness — done**  
    JSON subset + C subset + held-out rust under
    `SyntaxTokenLayoutMode::SharedAlphabet`; identical vocab/pad/structural/
    payload ranges and fixed bracket roles
    (`shared_alphabet_token_layout()` fingerprint); every
    `field_choice_table` token unpacks as domain-stable `CHOICE(n,k)` in the
    shared band; absolute `CHOICE(2,0)` / `CHOICE(7,2)` ids are grammar-
    independent. Claim id
    `i5_shared_alphabet_multi_grammar_same_space_v0`; boundary
    `compiler_shared_alphabet_layout_only`. PerGrammar defaults keep empty
    field-choice tables and non-shared vocabs (Decision 4; no pin edits).  
    Validate: `cargo test --features experimental --test i5_shared_alphabet_multi_grammar_same_space_test`

12. **Layout-diff / documentation gate — done**  
    Recorded shared vs per-grammar layout summary for the JSON subset fixture
    through `diff_syntax_token_layouts` (syntax token-layout diff machinery).
    Test `i5_shared_alphabet_layout_diff_json_subset_mode_switch_is_breaking_change`
    in `tests/i5_shared_alphabet_layout_diff_test.rs`: PerGrammar self-diff
    and SharedAlphabet self-diff are `exact_reuse`; PerGrammar → SharedAlphabet
    is `breaking_change` with moved `structural_decisions` / payload ranges,
    vocab/pad change to shared constants, unchanged fixed role offsets `0..9`,
    expanded stream contract on both sides, and renumbered alternative absolute
    IDs. SharedAlphabet alternative evidence is derived from the load-bearing
    `field_choice_table` (keys `rule.field.node_kind`); PerGrammar keeps the
    residual global map. Claim id `i5_shared_alphabet_layout_diff_v0`; boundary
    `compiler_shared_alphabet_layout_only`. Default absolute pins unedited
    (Decision 4). This document status line updated to CPU checklist complete;
    still no paper claims.  
    Validate:
    ```bash
    cargo test --features experimental --test i5_shared_alphabet_layout_diff_test
    cargo test --features experimental --test i5_shared_alphabet_mask_accept_consistency_test
    cargo test --features experimental --test i5_shared_alphabet_arity_boundary_test
    cargo test --features experimental --test i5_shared_alphabet_mixed_payload_dispatch_test
    cargo test --features experimental --lib syntax_state_machine
    cargo test --features experimental --test tree_sitter_json_subset_codec_test
    cargo test --features experimental --test tree_sitter_c_subset_codec_test
    cargo test --features experimental --test tree_sitter_metadata_adapter_test
    cargo test --features experimental --test syntax_backend_boundary_test
    cargo test --features experimental --test syntax_generated_codec_gate_test
    ```
    Escalate to `./scripts/testing/test-syntax-backend-gates.sh` if a tranche
    touches shared SM admission or default layout roles.

If a tranche would edit default absolute pins, **stop** — that is a Decision 4
violation unless a separate maintainer migration reopens replacement mode.

---

## Out of scope (hard)

- Model training, finetuning, transfer/interference measurement.
- Paper claims or cross-repo claim profile updates.
- Production-route registration or `tct_compiler_route_allowed` flips.
- New hand-built per-language subset codecs (CPU-4 breadth kill stands).
- Full C / full JS / tree-sitter-go inventory swap.
- Forced replacement of Go/C# pins (Decision 4 option b).
- Elided-stream shared-mode encode (v0).
- GPU work and agentic tool-call work.
- Edits to the downstream training repository or the downstream paper repository.
- De-homogenizing JSON/C projection (separate layout-migration campaign).

## Relationship to prior contracts

| Document | Relationship |
| --- | --- |
| `docs/generic-codec-layer-contract.md` | Prerequisite: model-driven encode/decode; this contract only changes layout/role assignment |
| `docs/shared-sm-mixed-payload-contract.md` | Discriminated alternatives + state-gated choice; shared alphabet generalizes the token **identity** of those choices across grammars |
| `docs/tree-sitter-json-mixed-payload.md` | Full-pin mixed fact; still not a codec or transfer claim |
| `docs/syntax-state-machine-contract.md` | Broader SM planning; shared mode must remain consistent with structural token roles |
| Investigation Queue I5 transfer experiment | GPU consumer of this alphabet; not gated by and not a substitute for this CPU contract |

## Non-goals (recap)

- No claim that shared tokens transfer across grammars.
- No default pin rewrites.
- No language-keyed tables inside encode/decode walkers.
- No alphabet growth per grammar.
- No "decode without a model."

## Validation (when implementing)

```bash
cargo test --features experimental --lib syntax_state_machine
cargo test --features experimental --test tree_sitter_metadata_adapter_test
cargo test --features experimental --test tree_sitter_json_subset_codec_test
cargo test --features experimental --test tree_sitter_c_subset_codec_test
cargo test --features experimental --test syntax_backend_boundary_test
cargo test --features experimental --test syntax_generated_codec_gate_test
```

Optional escalation when default layout roles or shared SM admission move:

```bash
./scripts/testing/test-syntax-backend-gates.sh
```
