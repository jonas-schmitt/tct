# Generic Codec Layer Contract

> **Experimental-frontend note.** This document was written inside the private
> TCT research repository and is kept as a contract reference for the
> experimental non-JSON frontends, which are compiled only with the
> `experimental` Cargo feature. It may cite roadmap, report, or
> sibling-repository documents that are not part of this repository; treat
> those citations as historical context. The stable JSON Schema core is
> documented in `README.md`, `ARCHITECTURE.md`, and `DEVELOPMENT.md`.

Document path: `docs/generic-codec-layer-contract.md`.

Status: **implemented** (API, subset migrations,
held-out constructed-tree proof, and regression gates). Owner: maintainer-owned
shared raw SM / generic encode path. Checklist items 1–12 are the landed
implementation order; this status line is not a paper claim and does not
register a production route.

This document is the design contract for a **language-neutral encode** over the
shared raw syntax state machine: `RawSyntaxDecodedNode` → `Vec<u32>` driven by
`CompiledSyntaxModel` (`ModelFacts` + `SyntaxTokenRoles`), plus migration of the
existing tree-sitter JSON and C subset codecs onto that encoder.

It is the inverse surface of decode already provided by
`CompiledSyntaxModel::decode_complete_tree` →
`RawSyntaxDecodedTree::decoded_node_tree`. It builds on
`docs/shared-sm-mixed-payload-contract.md` (`shared_sm_mixed_payload_contract_v1`)
and on the shared subset framework in `src/tree_sitter_subset_framework.rs`. It
does **not** add a tree-sitter runtime, a new per-language subset codec, a
production route, or paper claims.

Claim id for this document:

```text
generic_codec_layer_contract_v0
```

## Surfaces and claim boundary

| Surface | Role |
| --- | --- |
| Metadata lower | Pin → `GrammarIR` → `StructuralModel` is already target-neutral for Tier A held-out ingestion (CPU-5 T5; `tests/fixtures/tree_sitter_metadata/rust/`, `TreeSitterGrammarPin::rust_fixture_dir`) |
| Decode | `CompiledSyntaxModel::decode_complete_tree` / `decoded_node_tree` already yield language-neutral `RawSyntaxDecodedNode` / `RawSyntaxDecodedValue::{Node,Terminal}` for any compiled model (`src/syntax_state_machine.rs`) |
| Mixed fields (decode) | Shared SM admits mixed Nonterminal + payload alternatives (`shared_sm_mixed_payload_contract_v1`; `ModelFacts::alternatives_by_field`, `mixed_payload_fields`; `raw_replay` choice path) |
| Encode today | **Missing generically.** JSON hand-writes `emit_value_node` (`src/tree_sitter_json_subset_codec.rs`); C hand-writes `emit_translation_unit` and per-construct emitters (`src/tree_sitter_c_subset_codec.rs`). No `src/` API takes `RawSyntaxDecodedNode` and returns raw tokens |
| Subset framework | `CompiledSubsetCodecCore`, `FrozenMultiAltNodeStarts`, `run_round_trip_source`, `validate_encode_tokens`, `validate_prefix_and_complete_stream`, `emit_u8_integer_payload`, `emit_utf8_payload`, `observe_mixed_payload_risks` (`src/tree_sitter_subset_framework.rs`) — reuse, do not fork |
| This contract | Generic encode + codec migration + held-out constructed-tree proof shape |
| Out of scope | Source parsing, render/pretty-print, production-route registration, new hand-built subset codecs, Go/C# layout pins, corpus/entropy/forced-share, paper claims, GPU work |

**Hard source-parse boundary.** This repository has no tree-sitter runtime
dependency (`Cargo.toml` does not list one). Source text cannot be parsed
generically. In `CompiledSubsetCodecCore::run_round_trip_source`, the four
closures are `parse`, `encode`, `decode`, and `render`; only **`encode` and
`decode`** become generic under this contract. **`parse` and `render` stay
language-specific** and are explicitly out of scope.

Provable generically (once implemented):

- For any grammar that lowers to a compilable `StructuralModel`, and for any
  complete token stream the shared SM accepts:
  `encode_generic(decode_complete_tree(tokens).decoded_node_tree()) == tokens`.
- For any `RawSyntaxDecodedNode` that conforms to that model:
  `decode_complete_tree(encode_generic(tree)).decoded_node_tree() == tree`
  (under the normalization rules in § Consumed-token accounting).

**Not** provable without a language parser (and therefore not claimed by this
contract or by the held-out proof):

- Source-level validity for a held-out grammar
  (`parse` → encode → decode → `render` → reparse).

## Problem

Decode is already generic. Encode is not.

Each retained subset codec hand-writes an emit walker over a language-specific
value type:

| Codec | Language value | Hand-written emit entry | File |
| --- | --- | --- | --- |
| JSON subset | `JsonSubsetValue` | `TreeSitterJsonSubsetCodec::emit_value_node` | `src/tree_sitter_json_subset_codec.rs` |
| C subset | `CSubsetProgram` | `TreeSitterCSubsetCodec::emit_translation_unit` (+ per-construct helpers) | `src/tree_sitter_c_subset_codec.rs` |

Both reuse framework helpers for multi-alt freezing and payload bytes, but the
**structure walk** (which fields, which choice tokens, which wrappers) is
duplicated per language. That is the opposite of the CPU-5 / mixed-payload depth
direction: metadata lowering and decode no longer need language-keyed SM Rust;
encode still does.

There is no function in `src/` that accepts a `RawSyntaxDecodedNode` (or
equivalent language-neutral tree) and returns `Vec<u32>` raw tokens. Closing
that gap is the entire scope of the generic codec layer.

## Current layers (encode column)

| Layer | Generic today? | Notes |
| --- | --- | --- |
| Adapter → GrammarIR | Yes (Tier A) | CPU-5: default terminal class map + optional pin sidecar; held-out rust needs zero sidecar |
| GrammarIR → StructuralModel | Yes | Mixed alternatives accepted; payload leaves and choice edges recorded separately |
| Raw SM compile | Yes | `ModelFacts` + `SyntaxTokenRoles`; mixed choice tokens via `ensure_mixed_field_choice_tokens` |
| Raw SM decode | Yes | Event stream + `decoded_node_tree` |
| Raw SM encode | **No** | Gap this contract fills |
| Subset projection policy | Language-specific | JSON/C still **homogenize** payload leaves into Nonterminal wrappers (e.g. JSON `number` + synthetic `value`; C `MIXED_PAYLOAD_RISKS` resolutions) |
| Subset parse / render | Language-specific | Stays out of scope |
| Absolute golden pins | Homogeneous streams | `tree_sitter_json_subset_absolute_token_id_golden_pins`, `tree_sitter_c_subset_absolute_token_id_golden_pins` |

## Contract goals

1. **Model-faithful encode** — emit exactly the expanded raw stream the shared
   SM would accept for the given `StructuralModel` / token roles; no second
   encoding dialect.
2. **Language neutrality** — zero language-keyed match arms in the generic
   encoder (no `"json"`, `"c"`, `"rust"`, no rule-name allowlists).
3. **Decode inverse** — for complete valid streams and model-conforming trees,
   encode and decode are inverses (see § Consumed-token accounting).
4. **Fail closed** — non-conforming trees error with rule, field, node kind,
   and reason; never best-effort emission.
5. **Reuse framework** — multi-alt selection, u8/utf8 payload emission,
   vocabulary/PAD validation, and prefix/complete checks stay on
   `CompiledSubsetCodecCore` / shared roles helpers; do not invent a parallel
   core.
6. **No silent layout drift** — migration of JSON/C must either keep absolute
   golden streams bit-identical or ship pin edits with written rationale and
   stream/layout evidence (see § Token-layout implications).

## The generic encode contract

### A. Public interface (intended)

Primary API on the already-generic compiled model (names may be adjusted
slightly in implementation, but the shapes are load-bearing):

```text
CompiledSyntaxModel::encode_complete_tree(
    &self,
    root: &RawSyntaxDecodedNode,
) -> Result<Vec<u32>, RawSyntaxError>
```

Inputs:

| Input | Source of truth |
| --- | --- |
| Tree | `RawSyntaxDecodedNode { rule_name, node_kind, fields: Vec<RawSyntaxDecodedField> }` with `RawSyntaxDecodedValue::{Node, Terminal}` |
| Model facts | Private `ModelFacts` already held by `CompiledSyntaxModel` (`records`, `children_by_field`, `payloads_by_field`, `alternatives_by_field`, `mixed_payload_fields`, `multi_alternative_nonterminal_fields`, `root_rule`) |
| Roles | `SyntaxTokenRoles` (`node_start`, `node_end`, `field_start`, `field_end`, `field_missing`, `repeated_empty`, `terminal`, `payload_end`, `boolean_true` / `boolean_false` only where homogeneous boolean payload still uses them, `node_start_alternative_tokens`, payload ranges, `vocab_size`, `pad_token`) |

Output: expanded raw token stream (not elided). Elided-stream encode is out of
scope for v0 unless a later tranche names it.

Optional thin wrappers (preferred for subset migration, not a second encoder):

```text
CompiledSubsetCodecCore::encode_decoded_tree(&self, root: &RawSyntaxDecodedNode)
  -> Result<Vec<u32>, TreeSitterSubsetError>
```

which calls `model.encode_complete_tree` then `validate_encode_tokens`.

### B. Root and record walk

1. Root `rule_name` / `node_kind` must match the model's root record
   (`ModelFacts::root_rule` and that record's `node_kind`). Mismatch → error.
2. Emit root **plain** `roles.node_start` (root is never an alternative choice).
3. Walk **model field order** (`RecordSpec.fields`), not the order fields happen
   to appear in the decoded node if they differ. Unknown field names on the
   tree → error. Missing required field → error. Extra values after a
   single-valued field is full → error.
4. For each field, emit `field_start`, body, `field_end` — except when the field
   is absent/optional-missing, in which case emit `field_start`,
   `field_missing`, `field_end` (same bracketing the decoder expects today).
5. After all fields, emit `node_end`.

Field-less records emit only `node_start` … `node_end` (no fields).

### C. Per-token decisions (homogeneous fields)

A field is **homogeneous** when it is **not** in
`ModelFacts::mixed_payload_fields`.

| Situation | Tokens |
| --- | --- |
| Required or optional present Nonterminal child | Choice-or-plain child `node_start` (below), full child record encoding, … |
| Multi-alt Nonterminal field (`multi_alternative_nonterminal_fields`) | Child open uses `roles.node_start_alternative_tokens[child.node_kind]` (same assignment `FrozenMultiAltNodeStarts::child_node_start` freezes today). Missing map entry → error. |
| Single-alt Nonterminal field | Child open uses plain `roles.node_start`. |
| Homogeneous IntegerPayload | `roles.terminal` ++ integer payload tokens under the layout integer contract ++ `roles.payload_end`. Prefer reusing the algorithm of `CompiledSubsetCodecCore::emit_u8_integer_payload` when the integer range role is `u8_integer_payload`. |
| Homogeneous StringPayload / IdentifierPayload | `roles.terminal` ++ utf8 byte tokens ++ `roles.payload_end`. Prefer reusing `CompiledSubsetCodecCore::emit_utf8_payload`. Empty content on a repeated-only payload field may use `repeated_empty` instead of an empty payload sequence when that is what decode materializes for the model (JSON `string.children` empty content is the existing witness). |
| Optional field absent / `missing == true` | `field_missing` (no values). |
| Repeated field with zero values / `repeated_empty == true` | `repeated_empty` (no values). |
| Repeated field with N values | N concatenated value encodings (each Nonterminal child or payload terminal as above). |

Generic `terminal` is **only** legal on homogeneous payload fields — matching
decode (`raw_replay` rejects generic `terminal` on mixed fields).

### D. Per-token decisions (mixed fields)

A field is **mixed** when it appears in `ModelFacts::mixed_payload_fields`.
Alternatives and body kinds come from
`ModelFacts::alternatives_by_field` / `DiscriminatedAlternative` /
`AlternativeBodyKind` (same tables decode uses).

For each value on a mixed field:

1. Resolve the alternative by the value's node kind:
   - `RawSyntaxDecodedValue::Node(child)` → Nonterminal alternative whose
     `node_kind` / `target_rule` matches `child`.
   - `RawSyntaxDecodedValue::Terminal(t)` → payload alternative whose
     `node_kind` matches `t.node_kind` (and whose body kind matches the
     terminal class implied by `t.terminal` / payload materialization).
2. Emit the **structural choice token**
   `roles.node_start_alternative_tokens[alt.node_kind]` (must exist after
   `ensure_mixed_field_choice_tokens`; missing → construction or encode error).
3. Body by `AlternativeBodyKind` (must match
   `docs/shared-sm-mixed-payload-contract.md` decode accounting):

| Body kind | After choice token |
| --- | --- |
| `ChildRuleDescent` | Full child record encoding (no extra choice token for the child's root open — the choice token **is** the child `node_start`, same as decode's `apply_node_start(choice_token)`). |
| `BooleanPayload` | **Nothing.** Choice alone completes; zero payload bytes. Explicit identities only: node kind `"true"` / `"false"`. |
| `IntegerPayload` | Integer payload value token(s) ++ `payload_end`. **No** generic `terminal` (choice already opened the payload substate). |
| `Utf8Payload` | Utf8 payload byte tokens ++ `payload_end`. **No** generic `terminal`. |

Choice tokens remain in the structural_decisions band. Payload bytes never
double as choice tokens. Mask/admissible behavior stays a decode concern; encode
must simply emit the unique legal sequence.

### E. Non-conforming trees

Encode **errors** (typed, with rule / field / node kind / expected class) when:

- root rule or node kind mismatches the model;
- a field is unknown, required-but-missing, or over-cardinality;
- a value's node kind is not an alternative of the open field;
- a Nonterminal child rule/node_kind pair is not a StructuralModel reference;
- a Terminal appears where only Nonterminal is legal, or the reverse;
- payload text cannot be re-encoded under the layout contract (e.g. non-u8
  integer when the range is `u8_integer_payload`, invalid UTF-8 already cannot
  appear in `String` payloads);
- mixed BooleanPayload uses a node kind other than `"true"` / `"false"`;
- emitted token would be `>= pad_token` or `>= vocab_size` (same rejection as
  `validate_encode_tokens`).

No silent fixups, no dropping fields, no inventing default alternatives.

### F. Placement and privacy

`ModelFacts` remains crate-private (`pub(super)` today). Encode therefore lives
**inside** `CompiledSyntaxModel` / `syntax_state_machine` (with helpers next to
`raw_replay` if needed), not as an external crate that re-derives facts from
public sidecars alone. Subset codecs call the public encode API; they do not
reimplement choice-token selection.

## Homogenization vs native mixed encoding

### Decision (load-bearing)

**The generic encoder is model-faithful.** It does not homogenize and does not
undo homogenization. Whatever field shapes the compiled `StructuralModel`
carries, encode emits.

| Policy layer | Who owns it | Effect |
| --- | --- | --- |
| Subset projection (JSON/C) | Language modules (`project_*` in subset codecs) | May continue to wrap payload leaves as Nonterminals so choice fields stay homogeneous (valid specialization per mixed-payload contract §D) |
| Generic encode | Shared SM | Emits native mixed sequences **only when** `mixed_payload_fields` is non-empty for that field; otherwise emits homogeneous sequences |
| Held-out / full-pin models | Pin lower as-is | Mixed fields encode natively; no per-language wrapper synthesis in the generic layer |

### Per construct class

| Construct class | Model shape (subset today) | Generic encode form | Expected vs current JSON/C golden streams **if projection unchanged** |
| --- | --- | --- | --- |
| Multi-alt Nonterminal choice (e.g. JSON `_value.subtype` → null/true/false/string/number/array/object) | Homogeneous NT alts | Alt-specific `node_start` + child body | **Identical** |
| Single-alt Nonterminal (e.g. `pair`, array element `_value`) | Homogeneous NT | Plain `node_start` + child | **Identical** |
| Field-less leaf NT (`null` / `true` / `false` wrappers) | Field-less record | `node_start` + `node_end` | **Identical** |
| Wrapped IntegerPayload (JSON `number.value`, C number wrapper `value`) | Homogeneous payload field under NT wrapper | wrapper open + `field_start` + `terminal` + u8 + `payload_end` + `field_end` + wrapper close | **Identical** |
| Wrapped Utf8 / IdentifierPayload | Homogeneous payload under NT wrapper | same pattern with utf8 band (or `repeated_empty` for empty content where the model does today) | **Identical** |
| Optional missing | `field_missing` | `field_missing` | **Identical** |
| Repeated empty | `repeated_empty` | `repeated_empty` | **Identical** |
| Native mixed IntegerPayload on a choice field | Full pin / witness / de-homogenized model only | `choice` + integer value token(s) + `payload_end` (**no** `terminal`, **no** wrapper `node_end`) | **N/A** to current goldens unless projection flips |
| Native mixed BooleanPayload | Same | `choice` only | **N/A** unless flip |
| Native mixed Utf8 / IdentifierPayload | Same | `choice` + utf8 bytes + `payload_end` | **N/A** unless flip |
| Native mixed Nonterminal sibling on same field | Same | `choice` (= child node_start) + child body | **N/A** unless flip |

### Why not flip subsets to native mixed in the same work?

1. Depth goal is **removing hand-written emit walkers**, not forcing a layout
   migration. Homogenization is already declared a valid specialization
   (`docs/shared-sm-mixed-payload-contract.md` §D).
2. Absolute golden pins tripwire downstream BPE-style artifacts; changing them
   without need is churn.
3. Model-faithful encode still **exercises** native mixed paths via the existing
   synthetic mixed-payload witness and via constructed held-out trees on mixed
   fields — so the generic path is not “homogeneous-only.”

**Explicit non-decision for later:** de-homogenizing JSON/C projection to match
full-pin mixed surfaces remains a separate layout-migration campaign. This
contract neither forbids nor requires it; if attempted, § Token-layout
implications applies in full.

## Migration plan for the two existing codecs

### What becomes generic

| Step | Generic | Language-specific |
| --- | --- | --- |
| Pin lower + subset project | Framework + existing projectors | Projector rules stay in language modules |
| StructuralModel + raw SM compile | `CompiledSubsetCodecCore::from_pinned_dir` | Layout constants / subset id / claim boundary strings |
| **Encode structure walk** | `encode_complete_tree` | — |
| Payload byte emission | Shared helpers (u8 / utf8) | — |
| Decode tokens → tree | Already generic | — |
| Tree → language value | — | Existing `value_from_decoded_value_node` / `program_from_decoded_root` |
| Language value → tree | — | **New** lower: `JsonSubsetValue` → `RawSyntaxDecodedNode`, `CSubsetProgram` → `RawSyntaxDecodedNode` (mirror of today's raise helpers) |
| Parse / render | — | Unchanged |
| Round-trip harness | `run_round_trip_source` closures | Closures still supplied by language codec |

### Intended post-migration `encode` shape

```text
// JSON (conceptual)
fn encode(&self, value: &JsonSubsetValue) -> Result<Vec<u32>, _> {
    let root = value_to_decoded_node(value)?;           // language-specific
    self.core.encode_decoded_tree(&root)               // generic + validate_encode_tokens
}

// C (conceptual)
fn encode(&self, program: &CSubsetProgram) -> Result<Vec<u32>, _> {
    let root = program_to_decoded_node(program)?;
    self.core.encode_decoded_tree(&root)
}
```

`run_round_trip_source` continues to receive language `parse` / `encode` /
`decode` / `render`. After migration, the `encode` closure is the thin wrapper
above; the harness itself does not change.

Delete (or reduce to dead-code-free leftovers) the hand-written
`emit_value_node` / `emit_translation_unit` / per-construct emit helpers once
the generic path and golden pins (or authorized pin migration) prove parity.

### What must not happen

- Do not add a third JSON or C “native mixed” codec module.
- Do not register tree-sitter codecs on a production route.
- Do not change Go/C# layout pins as part of this work.
- Do not reintroduce language-keyed branches inside
  `encode_complete_tree`.

## Held-out grammar proof

### Authorization

The CPU-5 tree-sitter-rust pin
(`tests/fixtures/tree_sitter_metadata/rust/`, pin id
`tree_sitter_rust_node_types_v0.24.0`, accessor
`TreeSitterGrammarPin::rust_fixture_dir`) **may** be used to prove the generic
codec end-to-end under this contract. That is an explicit extension beyond the
original T5 “ingestion only” note: it is still **not** a source codec, not a
corpus, and not entropy/forced-share evidence.

### Proof shape

1. Lower the rust pin → `GrammarIR` → `StructuralModel` with a suitable
   oversized or pin-local token layout (reuse the held-out layout helper pattern
   already used in `tests/tree_sitter_metadata_adapter_test.rs` for T5
   StructuralModel lowering — do not invent rust-specific classification Rust).
2. Compile `CompiledSyntaxModel` (zero language-keyed encode/decode Rust).
3. **Construct** a minimal `RawSyntaxDecodedNode` tree that conforms to one or
   more records of that model (fixture data in the test, not parsed source).
   Prefer a small field-less or single-field fragment that is unambiguously
   present in the lowered model; if the only honest witness needs a mixed
   field, construct Terminal/Node values accordingly.
4. Assert:
   - `encode_complete_tree(tree)` succeeds;
   - `decode_complete_tree(tokens).decoded_node_tree() == tree`;
   - `encode_complete_tree(decode(...)) == tokens`.
5. Grep/architecture guard in the same test file (or adjacent unit test): the
   generic encode implementation modules contain no rust-specific rule-name
   tables. The test may name rust **fixture paths** only.

### Claim id for the evidence

```text
generic_codec_layer_held_out_rust_constructed_tree_v0
```

Claim boundary text (must appear in the test or report string the test
asserts):

```text
syntax_level_generic_encode_decode_constructed_tree_only
```

Does **not** claim: parse, render, reparse, source round-trip, production route,
layout pin stability for rust, or any paper metric.

## Token-layout implications

### Default expectation for JSON/C absolute goldens

Under the homogenization decision above, **pinned absolute token vectors are
expected to remain bit-identical** when subset projection stays homogenized and
encode migrates to the generic walker. Implementation must treat unexpected
golden failure as a defect in encode or in the language→tree lower, not as a
reason to edit pins.

Tests:

- `tree_sitter_json_subset_absolute_token_id_golden_pins` in
  `tests/tree_sitter_json_subset_codec_test.rs`
- `tree_sitter_c_subset_absolute_token_id_golden_pins` in
  `tests/tree_sitter_c_subset_codec_test.rs`

### If a layout or absolute stream must move

Maintainer authorization for the implementation branch: pins **may** change when
necessary, but **only** with:

1. **Before/after absolute token vectors** for every affected golden fixture
   (committed in the pin test or an adjacent evidence file under `target/` for
   review, with the pin test update in the same commit as the codec change).
2. **Written rationale** in the commit message stating which construct class
   moved and why (e.g. intentional de-homogenization of `_value.subtype`).
3. **Range-level report** when token ranges / structural assignment change:
   run `tct-token-diff` (or the syntax token-layout pin/diff gate for
   layout-affecting syntax work — see `DEVELOPMENT.md`) and record the class
   (`exact_reuse` / `layout_only_unchanged` / `breaking_change`). Unchanged
   ranges with changed streams are still a **stream breaking change** for any
   consumer of absolute subset streams — do not treat range `exact_reuse` as
   permission to skip absolute vector evidence.
4. Merging such a change to `main` is a **separate maintainer investigation**;
   the lane/branch may land pins only with the evidence above.

A pin edit without evidence is a **defect**, not a decision.

### Per construct class — pin movement summary

| Construct class | Pin movement if subsets keep homogenization | Pin movement if subsets de-homogenize |
| --- | --- | --- |
| Homogeneous multi-alt / single-alt NT, wrappers, missing, repeated_empty | None expected | N/A |
| Homogeneous wrapped payloads | None expected | N/A |
| Native mixed payload on former wrapper fields | N/A (not in subset model) | **Absolute streams break** even if band sizes stay put → `breaking_change` for stream consumers |

## Consumed-token accounting and determinism

Encode must be the exact inverse of decode for the expanded stream.

### Identities tests will assert

```text
// Stream → tree → stream  (any complete stream the SM accepts)
let tree = model.decode_complete_tree(&tokens)?.decoded_node_tree()?;
assert_eq!(model.encode_complete_tree(&tree)?, tokens);

// Tree → stream → tree  (any model-conforming tree, after normalization)
let tokens = model.encode_complete_tree(&tree)?;
let tree2 = model.decode_complete_tree(&tokens)?.decoded_node_tree()?;
assert_eq!(tree2, tree);
```

### Normalization rules for tree equality

- Field order in decoded trees follows model field order after a decode round
  trip; constructed trees should use the same order (encode enforces model
  order regardless).
- Payload strings match decode materialization (`u8` integers as decimal text
  as decode emits today; UTF-8 as the decoded string; boolean mixed identities
  as `"true"` / `"false"`).
- `missing` vs empty `values` vs `repeated_empty` are distinct; encode and
  decode must agree and must not collapse them.

### Accounting table (must match mixed-payload contract)

| Class | Tokens consumed for one field value |
| --- | --- |
| Homogeneous Nonterminal | child `node_start` (plain or alt) + full child record |
| Homogeneous IntegerPayload | `1` (`terminal`) + integer payload length + `1` (`payload_end`) |
| Homogeneous Utf8 / IdentifierPayload | `1` (`terminal`) + byte count + `1` (`payload_end`) |
| Mixed Nonterminal | `1` (choice = child node_start) + rest of child record body |
| Mixed BooleanPayload | `1` (choice only) |
| Mixed IntegerPayload | `1` (choice) + integer payload length + `1` (`payload_end`) |
| Mixed Utf8 / IdentifierPayload | `1` (choice) + byte count + `1` (`payload_end`) |
| field_missing | `1` |
| repeated_empty | `1` |

Plus the usual `field_start` / `field_end` / `node_end` brackets around field
and record structure.

Determinism: given the same model and tree, encode is pure and repeatable.
Choice-token assignment remains a **compile-time** property of
`CompiledSyntaxModel` (including `ensure_mixed_field_choice_tokens`); encode
must not invent free-slot tokens at emit time.

## Implementation checklist (commit-sized tranches)

Ordered so each step can land independently. Each tranche
must build, pass its validation command, and be reviewable alone.

1. **API skeleton + failing inverse test (homogeneous field-less leaf)**  
   Add `CompiledSyntaxModel::encode_complete_tree` (may initially
   `unimplemented!` only for unsupported shapes, but the field-less path should
   be real or the test should fail for the right reason). Unit test: synthetic
   field-less record → encode → decode tree equality.  
   Validate: `cargo test --features experimental --lib syntax_state_machine` (or the focused module
   path once tests land under `src/syntax_state_machine/`).

2. **Homogeneous required single-alt Nonterminal child + field brackets**  
   Record with one required Nonterminal field; assert stream ↔ tree identities.  
   Validate: focused lib test for the new cases.

3. **Multi-alt Nonterminal choice tokens**  
   Use finite-domain multi-alt field; assert alt-specific
   `node_start_alternative_tokens` selection (parity with
   `FrozenMultiAltNodeStarts` behavior).  
   Validate: focused lib test.

4. **Homogeneous IntegerPayload and Utf8 payload emission**  
   Reuse the byte algorithms from `emit_u8_integer_payload` /
   `emit_utf8_payload` (factor shared helpers if needed without breaking subset
   codecs). Cover empty utf8 / `repeated_empty` where the synthetic model
   requires it.  
   Validate: focused lib test + `cargo test --features experimental --lib tree_sitter_subset_framework`
   if helpers move.

5. **Optional `field_missing` and repeated `repeated_empty`**  
   Validate: focused lib test.

6. **Mixed-field encode (witness)**  
   Against `model_facts::mixed_payload_witness` / existing mixed decode tests:
   BooleanPayload choice-only; IntegerPayload choice+bytes+end; Utf8
   choice+bytes+end; Nonterminal descent. Assert inverse identities and that
   generic `terminal` is never emitted on mixed fields.  
   Validate: `cargo test --features experimental --lib syntax_state_machine` (mixed_payload modules).

7. **Framework wrapper**  
   `CompiledSubsetCodecCore::encode_decoded_tree` + `validate_encode_tokens`.  
   Validate: subset framework tests if present; otherwise codec tests in later
   tranches.

8. **JSON migration**  
   Implement `JsonSubsetValue` → `RawSyntaxDecodedNode`; route
   `TreeSitterJsonSubsetCodec::encode` through generic encode; remove hand
   emit walkers when green.  
   Validate:
   `cargo test --features experimental --test tree_sitter_json_subset_codec_test`  
   (must keep `tree_sitter_json_subset_absolute_token_id_golden_pins` green
   without pin edits under the default decision).

9. **C migration**  
   Same pattern for `CSubsetProgram`.  
   Validate:
   `cargo test --features experimental --test tree_sitter_c_subset_codec_test`  
   (absolute golden pins green without pin edits under the default decision).

10. **Held-out rust constructed-tree proof**  
    As in § Held-out grammar proof; claim id
    `generic_codec_layer_held_out_rust_constructed_tree_v0`.  
    Validate: focused test in `tests/tree_sitter_metadata_adapter_test.rs` or a
    dedicated test file that only uses the rust fixture path + generic APIs.

11. **Boundary / syntax gates (regression)**  
    Validate:
    ```bash
    cargo test --features experimental --test syntax_backend_boundary_test
    cargo test --features experimental --test syntax_generated_codec_gate_test
    cargo test --features experimental --test tree_sitter_metadata_adapter_test
    cargo test --features experimental --test tree_sitter_json_subset_codec_test
    cargo test --features experimental --test tree_sitter_c_subset_codec_test
    ```
    Escalate to `./scripts/testing/test-syntax-backend-gates.sh` when a tranche
    touches shared SM admission or layout roles.

12. **Mark this document implemented** (status line only) in a final docs
    tranche when the API and migrations are on the branch — still no paper
    claims, no production-route registration.

If a tranche must break absolute pins, stop and attach the evidence package in
§ Token-layout implications before editing goldens; do not “fix green” by
silent pin rewrite.

## Out of scope

- Source parsing of any language (no tree-sitter runtime dependency).
- Render / pretty-print / reparse evidence for held-out grammars.
- Production-route registration or `tct_compiler_route_allowed` flips.
- New hand-built per-language subset codecs (CPU-4 breadth kill stands; this
  work is **depth**: remove per-language encode special-casing).
- Full C / full JS / tree-sitter-go inventory swap.
- Go / C# syntax layout pin changes.
- Elided-stream generic encode (unless a later contract amends v0).
- BPE training, corpus packaging, entropy / forced-share measurements.
- Paper claims or cross-repo claim profile updates.
- GPU / I5 shared decision-token alphabet transfer experiment (successor after
  this layer exists).
- Editing `docs/shared-sm-mixed-payload-contract.md` except a later one-line
  pointer if maintainers want a cross-link (not required to implement encode).

## Relationship to prior contracts

| Document | Relationship |
| --- | --- |
| `docs/shared-sm-mixed-payload-contract.md` | Decode-side mixed field mechanism; this contract adds the encode inverse and keeps homogenization as a subset specialization |
| `docs/tree-sitter-json-mixed-payload.md` | Full-pin JSON mixed fact; still not a codec claim |
| `docs/tree-sitter-metadata-subset-codec-design.md` | Historical subset design; breadth retired; depth continues here |
| `docs/syntax-state-machine-contract.md` | Broader SM planning contract; encode must remain consistent with structural token roles described there |
| CPU-5 T5 | Held-out ingestion; this contract extends the pin to **constructed-tree** encode/decode only |

## Non-goals (recap)

- No “generic parse.”
- No promotion of I5.
- No golden-pin edits without evidence.
- No language-keyed tables inside the generic encoder.
- No claim that held-out rust source round-trips.

## Validation (when implementing)

```bash
cargo test --features experimental --lib syntax_state_machine
cargo test --features experimental --test tree_sitter_metadata_adapter_test
cargo test --features experimental --test tree_sitter_json_subset_codec_test
cargo test --features experimental --test tree_sitter_c_subset_codec_test
cargo test --features experimental --test syntax_backend_boundary_test
cargo test --features experimental --test syntax_generated_codec_gate_test
```

Optional escalation when shared admission or layout roles move:

```bash
./scripts/testing/test-syntax-backend-gates.sh
```
