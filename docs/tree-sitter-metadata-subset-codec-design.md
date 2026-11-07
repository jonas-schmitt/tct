# Tree-sitter Metadata → Subset Codec Design Note

> **Experimental-frontend note.** This document was written inside the private
> TCT research repository and is kept as a contract reference for the
> experimental non-JSON frontends, which are compiled only with the
> `experimental` Cargo feature. It may cite roadmap, report, or
> sibling-repository documents that are not part of this repository; treat
> those citations as historical context. The stable JSON Schema core is
> documented in `README.md`, `ARCHITECTURE.md`, and `DEVELOPMENT.md`.

Status: **MG-5 design note** (template) plus **MG-6 first instantiation**
pointer for tree-sitter-**C** construct-subset codec v0 (see the research roadmap).

This note records how a **non-JSON** subset codec is cut from a pinned
tree-sitter `node-types` export, using the landed JSON subset path as the
reference pattern. **MG-6** instantiates this template for C only:

| Field | Value |
| --- | --- |
| Subset id | `tree_sitter_c_minimal_function_subset_v0` |
| Claim boundary | `syntax_level_tree_sitter_c_subset_only` |
| Module | `src/tree_sitter_c_subset_codec.rs` (**landed**) |
| Tests | `tests/tree_sitter_c_subset_codec_test.rs` |
| Pin | `tests/fixtures/tree_sitter_metadata/c/` |
| v0 fixture shape | `int main(void) { return 0; }` (+ `int f(void){return 42;}`) |

**MG-6 landed:** pin inventory, subset GrammarIR → StructuralModel → raw SM
encode/decode/render for the closed free-function construct set. Claim remains
`syntax_level_tree_sitter_c_subset_only` — not full C.

| Subset pin rule | Role in `int main(void) { return 0; }` |
| --- | --- |
| `translation_unit` | root; children → one `function_definition` |
| `function_definition` | `type` + `declarator` + `body` (drop attr children) |
| `type_specifier` | subtype → `primitive_type` only |
| `primitive_type` | pin leaf; subset wraps text (`int` / `void`) |
| `_declarator` | subtype → `function_declarator` \| wrapped `identifier` |
| `function_declarator` | name declarator + `parameter_list` |
| `identifier` | pin `IdentifierPayload` leaf; subset wraps name payload |
| `parameter_list` | children → `parameter_declaration` only (`void`) |
| `parameter_declaration` | `type` only (no declarator for `(void)`) |
| `compound_statement` | children → `statement` only |
| `statement` | subtype → `return_statement` only |
| `return_statement` | children → `expression` |
| `expression` | subtype → wrapped `number_literal` only |
| `number_literal` | pin `IntegerPayload` leaf; subset wraps u8 `0..=255` |

Full-pin mixed-payload risks (must not appear on subset choice fields):

- `_declarator.subtype`: Nonterminal composites + bare `identifier:IdentifierPayload`
- `expression.subtype`: Nonterminals + `number_literal:IntegerPayload` +
  `identifier:IdentifierPayload` + `true`/`false` BooleanPayload

Still not a full C or JS codec, not tree-sitter-go, not Go inventory
replacement.

## Claim boundary

| Allowed | Not allowed |
| --- | --- |
| Template steps for pin → subset GrammarIR → StructuralModel → raw SM | Full C or JavaScript codec implementation |
| Documented failure modes from full-pin lowers (e.g. JS SM residual) | tree-sitter-go replacing the inventory Go path |
| Explicit subset identity + claim-boundary strings | Go layout pin changes |
| Reuse of adapter + pin checksum contract | I5 shared decision-token alphabet |
| Pointers to JSON mixed-payload and multi-pin evidence | Model training, GPU experiment matrices |

Working claim shape for any future subset (fill language id when implementing):

```text
syntax_level_tree_sitter_<lang>_subset_only
```

## Reference surfaces (JSON, already landed)

| Step | Surface |
| --- | --- |
| Pin | `tests/fixtures/tree_sitter_metadata/json/` + `pin.json` sha256 |
| Full lower | `TreeSitterMetadataAdapter::lower_pinned_grammar_dir` |
| Mixed-payload fact | `docs/tree-sitter-json-mixed-payload.md` |
| Shared framework | `src/tree_sitter_subset_framework.rs` (CPU-2: multi-alt freeze, reports, inventory, round-trip harness) |
| Subset codec | `src/tree_sitter_json_subset_codec.rs` (language projection + emit) |
| Tests | `tests/tree_sitter_json_subset_codec_test.rs` |

Multi-pin metadata evidence (no codecs): json + c + javascript under
`tests/fixtures/tree_sitter_metadata/{json,c,javascript}/`, smoke in
`tests/tree_sitter_metadata_adapter_test.rs`.

## Template steps (any future non-JSON subset)

Work in this order. Stop at the first step that fails with a precise residual;
do not invent a full grammar codec to paper over a structural gap.

### 1. Pin and checksum

1. Check in `node-types.json`, `pin.json` (package, version, license,
   `node_types_sha256`), and `LICENSE`.
2. Prove `lower_pinned_grammar_dir` accepts the pin and rejects digest
   mismatch (existing multi-pin / failure-mode tests).
3. Do **not** pull a Tree-sitter runtime into the library path.

### 2. Full-pin lower inventory

1. Lower the full pin to `GrammarIR`.
2. Record rule counts, subtype fields, children lists, and
   `GrammarTerminalKind` classification on payload-bearing leaves.
3. Attempt StructuralModel lower with a **sidecar-only** token layout
   (as C/JSON pins do for analysis). Record residuals honestly:
   - **C pin:** StructuralModel succeeds on the full pin with a large sidecar
     layout (metadata evidence only).
   - **JavaScript pin:** full-pin StructuralModel currently residuals
     (`unknown node kind` / `duplicate rule name` class) — a subset **must**
     project away those edges before any codec work.
4. Do not treat a successful full-pin StructuralModel lower as a codec claim.

### 3. Choose a deliberate construct subset

1. Name a stable `SUBSET_ID` (versioned string).
2. Select a **small** closed set of named constructs that still demonstrates
   recursion and at least one payload class (e.g. expression subset, statement
   block subset — not the entire language).
3. Prefer constructs that already lower cleanly through the adapter emission
   filter (named + referenced anonymous). Drop anonymous punctuation that is
   not needed for the subset renderer.
4. Write the subset as an explicit projection from the pin GrammarIR (or a
   hand-built GrammarIR whose rules are a named subset of pin rule names),
   not as a silent rewrite of the pin fixture.

### 4. Homogenize decision fields (mixed-payload rule)

Follow the JSON lesson in `docs/tree-sitter-json-mixed-payload.md`:

1. A single choice field must not mix Nonterminal wrappers with bare
   payload terminals unless a shared-SM mixed-payload design exists
   (it does not today).
2. Prefer **Nonterminal wrappers** for leaves that carry payload: e.g. pin
   bare `number: IntegerPayload` → subset `number` rule with a synthetic
   required `value` IntegerPayload field.
3. Keep boolean/null-like leaves as field-less Nonterminal rules whose
   **rule identity** carries meaning when that is simpler than BooleanPayload
   on a shared subtype field.
4. Optional / repeated fields must use the same raw-SM conventions as other
   syntax subsets (`repeated_empty`, required vs optional presence).

### 5. Sidecar token layout for the subset only

1. Allocate structural decision band, payload bands, and PAD with
   `vocab_size` / `pad_token` consistent with generated hot-path invariants
   (every ID &lt; `VOCAB_SIZE`; PAD last).
2. Keep the layout **sidecar-scoped** to the subset grammar id. Do not
   reuse or mutate Go `tct-go-grammar-artifact-v1` layout pins.
3. Integer ranges (if any) must be explicit (`GrammarIntegerValueRange`) and
   documented in the subset claim (JSON uses u8 `0..=255` only).

### 6. Raw SM encode / decode / render

1. Lower subset GrammarIR → StructuralModel → compiled raw syntax state machine.
2. Implement a tiny value AST + parse/render for the subset source surface
   (JSON used compact text; a C/JS subset would use a deliberately tiny
   concrete syntax, not full language parsing).
3. Round-trip: source → value AST → tokens → value AST → source (semantic
   equality for the subset).
4. Consume-token accounting and strict reject of out-of-subset tokens.

### 7. Tests and docs (same change)

1. Focused unit tests for parse/render edges and one multi-construct instance.
2. Pin provenance assertion (subset still cites the same pin sha256).
3. Explicit claim-boundary constant in code and a short docs section stating
   what is out of subset.
4. Update the research roadmap / handoff only if a Ready item asked for the codec; this
   design note alone does **not** schedule implementation.

## Pin-specific residuals (do not ignore)

| Pin | Metadata lower | Full-pin StructuralModel | Subset codec readiness |
| --- | --- | --- | --- |
| tree-sitter-json | Yes | Yes | **Landed** fixed subset |
| tree-sitter-c | Yes | Yes (sidecar layout) | **Landed** minimal-function subset codec (MG-6) |
| tree-sitter-javascript | Yes | Residual on full pin | Subset must project before SM; not scheduled |

MG-2 census: stream-level forced shares for C/JS still require a subset codec
plus corpus — **not** another metadata pin. MG-4 remains N/A.

## Anti-patterns

- Expanding a subset until it “almost” covers the full pin.
- Treating adapter success as validity-by-construction for source programs.
- Swapping Go inventory grammar for tree-sitter-go to “align” pins.
- Copying the JSON subset module with a find-replace language name without
  re-checking mixed-payload and SM residuals.
- Claiming cross-language entropy or paper results from metadata pins alone.

## Validation (current, no new codec)

```bash
cargo test --features experimental --test tree_sitter_metadata_adapter_test
cargo test --features experimental --test tree_sitter_json_subset_codec_test
cargo test --features experimental --test tree_sitter_c_subset_codec_test
python3 scripts/analysis/i1_multi_language_entropy_census_v2.py --self-test
```

## Next after this note

MG-5 template, MG-6 C subset codec, and CPU-2 shared-framework extract are
complete (`src/tree_sitter_subset_framework.rs`).

**Subset-codec work stays retired.** Do not invent full C, a full JS codec, a
new subset codec for any language, or a tree-sitter-go inventory swap. Every
anti-pattern listed above still holds.

**Generalization work is Ready (CPU-5).** The retirement above applies to
per-language *codecs*, not to the metadata ingestion path. That path does not
generalize today, and the defect is nameable: `classify_terminal`
(`src/tree_sitter_metadata.rs`) is a hardcoded node-kind allowlist, so a leaf
named `identifier` classifies while `property_identifier` does not, and every
new language needs new Rust match arms. The full JavaScript pin consequently
fails `StructuralModel` lowering — a residual pinned as a negative test in
`tests/tree_sitter_metadata_adapter_test.rs`.

The research roadmap item CPU-5 owns removing that per-language special-casing and specifying
the shared-SM mixed-payload contract that gates I5. Adding a subset codec is
breadth and stays retired; deleting per-language branches from shared
infrastructure is depth and is Ready. A held-out language pin is authorized
under CPU-5 T5 as structural-ingestion test evidence only — no codec, no
corpus, no entropy or forced-share claims.
