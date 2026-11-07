# Tree-sitter JSON Mixed-Payload Contract

> **Experimental-frontend note.** This document was written inside the private
> TCT research repository and is kept as a contract reference for the
> experimental non-JSON frontends, which are compiled only with the
> `experimental` Cargo feature. It may cite roadmap, report, or
> sibling-repository documents that are not part of this repository; treat
> those citations as historical context. The stable JSON Schema core is
> documented in `README.md`, `ARCHITECTURE.md`, and `DEVELOPMENT.md`.

Status: **contract note for Workstream B′** (CPU residual after B0–B1+).

This document records why the fixed tree-sitter-json **subset** codec wraps
`number` as a Nonterminal with a synthetic IntegerPayload field, instead of
keeping the full pin’s bare `number` IntegerPayload on `_value.subtype`.

Claim boundary for all current codec evidence remains:

```text
syntax_level_tree_sitter_json_subset_only
```

This is **not** a full-pin codec claim, not JSON Schema TCT, not a Go layout
pin, and not a shared multi-grammar decision-token (I5) design.

## Surfaces

| Surface | Role |
| --- | --- |
| Pin fixture | `tests/fixtures/tree_sitter_metadata/json/` (tree-sitter-json **v0.24.8**) |
| Adapter | `src/tree_sitter_metadata.rs` → full pin `GrammarIR` |
| Subset codec | `src/tree_sitter_json_subset_codec.rs` → projected subset + raw SM |
| Focused tests | `tests/tree_sitter_metadata_adapter_test.rs`, `tests/tree_sitter_json_subset_codec_test.rs` |

## Full-pin `_value.subtype` payload kinds

The pin exports `_value` as a tree-sitter **supertype**. The adapter lowers
that to a required single-choice synthetic field `subtype` whose alternatives
are the concrete JSON value kinds. Leaf classification uses
`classify_terminal` (composite named kinds force Nonterminal):

| `_value.subtype` kind | Full-pin `GrammarTerminalKind` | Why |
| --- | --- | --- |
| `array` | `Nonterminal` | Composite (children list) |
| `object` | `Nonterminal` | Composite (pair children) |
| `string` | `Nonterminal` | Composite (`string_content` / escapes) |
| `null` | `Nonterminal` | Named leaf with no dedicated null-payload class in `classify_terminal` |
| `true` | `BooleanPayload` | Boolean leaf mapping |
| `false` | `BooleanPayload` | Boolean leaf mapping |
| `number` | `IntegerPayload` | Number leaf mapping |

So one choice field, `_value.subtype`, mixes **three** terminal classes on the
full pin:

1. `Nonterminal` (structure + `null`)
2. `BooleanPayload` (`true` / `false`)
3. `IntegerPayload` (`number`)

StructuralModel lowering still accepts this shape (payload leaves and choice
edges are recorded separately). The limit is not “adapter cannot parse the
pin”; it is that a **single raw-SM field choice whose alternatives include both
payload terminals and Nonterminal wrappers** is not a design we treat as a
stable, simple codec contract for this frontend spike.

## Why the subset wraps `number`

The B1+ subset projects a deliberate construct subset:

```text
_value → null | true | false | string | number | array | object
```

and **forces every subtype alternative to `Nonterminal`**:

- `null` / `true` / `false` remain field-less Nonterminal wrapper rules
  (boolean meaning is the **rule identity**, not a BooleanPayload leaf on
  `_value.subtype`).
- `string` stays a Nonterminal with optional `string_content` StringPayload.
- `number` becomes a Nonterminal with a **required synthetic field** `value`
  carrying u8 `IntegerPayload` (`0..=255` via the subset integer-value range).
- `array` / `object` stay Nonterminals with recursive children.

That keeps `_value.subtype` **homogeneous Nonterminal**. Integer payload lives
only under `number.value`, so the raw SM opens a payload substate only after
the `number` node is selected—never as a peer of null/string/bool on the same
field.

Without the wrapper, bare pin-style `number: IntegerPayload` on
`_value.subtype` would share that field’s decision with Nonterminal and
BooleanPayload alternatives. Encoding, mask construction, and payload substate
entry would need a **shared-SM mixed-payload design** (one choice edge that
either descends into a child rule or enters a payload kind immediately). That
design is intentionally out of scope for the subset spike.

## Claim boundary

| Allowed | Not allowed |
| --- | --- |
| Pin → full `GrammarIR` payload-kind facts (adapter tests) | Full-pin JSON codec / full JSON value space |
| Subset GrammarIR → StructuralModel → raw SM encode/decode for the fixed construct subset | JSON Schema compiler / generated JSON Schema crates |
| Claim id `syntax_level_tree_sitter_json_subset_only` | Go layout pins, M5 package expansion, tree-sitter-go inventory replacement |
| Documented mixed-payload limit + subset homogenization | I5 shared decision-token alphabet; full-pin mixed-payload SM redesign |

Subset identity:

```text
tree_sitter_json_null_bool_string_number_array_object_subset_v0
```

## Shared-SM follow-up

The shared raw-SM mixed-payload contract is **implemented** in
`docs/shared-sm-mixed-payload-contract.md` (owner of the target-neutral
mechanism). Subset codecs still **homogenize** until an explicit layout
migration: **keep the number Nonterminal wrapper** (and the all-Nonterminal
subtype set) for any tree-sitter-json subset codec path. Do not expand leaf
payload kinds on `_value.subtype` without a layout migration report and
golden-pin / BPE handling.

## Validation

```bash
cargo test --features experimental --test tree_sitter_metadata_adapter_test
cargo test --features experimental --test tree_sitter_json_subset_codec_test
```
