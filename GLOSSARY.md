# TCT

TCT (Type-Constrained Tokenization) compiles a JSON Schema into a schema-specific tokenizer whose tokens take their meaning from their position in the schema. The generated tokenizer encodes instances of the schema as compact token sequences and decodes token sequences back into values with the schema's structure.

Each entry names one concept. Aliases the repository already uses are given in the definition; `_Avoid_` lists only words that would mislead.

## Language

### Compilation

**Generated tokenizer**:
The Rust code (a tokenizer module) the compiler emits for one schema and configuration; it encodes and decodes values of the schema's root type and fixes the token layout. A wheel built from it packages the same tokenizer.
_Avoid_: generated codec (an experimental-frontend term)

**Compiler pass**:
One of the four passes compilation runs in order: discovery, analysis, the optimization phase, and generation. Each consumes the immutable output of earlier passes, and only generation emits code.

**Optimization pass**:
A named, individually selectable optimization run inside the optimization phase, such as `enum_optimization`, `integer_optimization`, or `bit_packing`. An optimization level (O0, O2, O3, Os, OVocab) is a named preset of passes and encoding options.
_Avoid_: optimization pass, for the optimization phase itself

**Encoding decision**:
The per-type choice of token encoding (one-token index, multi-token codec, zero-token constant, bounded-integer offset, format-aware string, bit-packed presence) that the optimization phase records as data for generation to consume.

**Multi-token codec**:
A lossless fixed-width encoding, also called a structural codec, that spreads an enum index, discriminator, or bounded-integer value over several digit tokens; used only when the value count exceeds the structural vocabulary cap.

**Structural vocabulary cap**:
The optional limit set by `--max-vocab` on how many distinct values one structural token may carry; values beyond it get a multi-token codec in that base. It bounds neither the base vocabulary nor the total vocabulary.

**Encoding base**:
The power-of-two radix, set by `max_base_vocab`, of the fixed-width integer encodings for collection lengths and bounded-integer offsets. It lower-bounds the base vocabulary and never causes a multi-token codec.
_Avoid_: codec base

**Experimental frontend**:
A non-JSON compilation route (SQL schema linking, Go and C# syntax, tree-sitter subset codecs), built only with the `experimental` feature and without stability guarantees.

### Instances and guarantees

**Schema-valid**:
Passes a JSON Schema validator for the schema. Schema-valid does not imply encodable, and a decoded value is not guaranteed schema-valid because value constraints are not enforced.

**Encodable**:
Deserializes into the generated types and can therefore be encoded. A schema-valid instance can be unencodable, most often because a closed object rejects an undeclared key.
_Avoid_: valid instance

**Catch-all map**:
The generated member that holds an object's additional properties when the schema allows them. A closed object (`additionalProperties: false`) has none and rejects undeclared keys.

**Value constraint**:
A keyword that restricts values rather than types or structure: string patterns, length and item-count bounds, uniqueness, multiples, and numeric bounds on floats or one-sided integers. Schemas may use them, but no encode or decode mode enforces them.

**Validity by construction**:
The guarantee that every successfully decoded token sequence yields a value with the schema's types, structure, and finite choices, whatever tokens a model produced. It excludes value constraints.
_Avoid_: schema validity

**Round trip**:
Encoding an instance and decoding it back to a semantically equivalent JSON value, with explicit null kept distinct from an omitted field. Every optimization level must preserve it.

### Token space

**Schema-local token stream**:
The sequence of untyped token IDs one generated tokenizer produces for one instance, each ID's meaning given by its position. It includes BPE merge IDs when a merge table is embedded, and never sequence-wrapper tokens or padding.
_Avoid_: raw stream (see the ambiguities below)

**Token layout**:
The compile-time assignment of ID ranges and positional meanings (base tokens, merge IDs, `PAD_TOKEN`, field order) for one schema and compiler configuration. Generated constants, merge tables, and token diff reports all refer to it.

**Base vocabulary**:
The ID range `[0, BASE_VOCAB_SIZE)`, also called the base layer, holding every ID the encoder emits before BPE: byte tokens, the end marker, and structural tokens, which share IDs and are told apart by position.

**Structural token**:
A base-vocabulary ID that carries structure rather than content at its position: an enum index, a discriminator, a presence flag, an optional-field state, or a collection length. Structural tokens reuse the IDs of byte tokens; they have no separate range.
_Avoid_: type tag (TCT drops JSON type tags)

**End marker**:
The base token (`END_STRING_MARKER`) that ends the byte tokens of a string or decimal-encoded number, so no length is needed up front.

**Finite choice**:
A position with a bounded set of alternatives: a boolean, an enum value, a union branch, or an optional member's presence or state.

**Enum index**:
The structural token (or multi-token codec digits) giving the position of an `enum` or `const` value in its value list; a single-value enum can take zero tokens.
_Avoid_: enum discriminator

**Discriminator**:
The structural token before a `oneOf`/`anyOf` variant's payload that selects the branch. Not to be confused with a discriminator key, the JSON property (such as `kind`) that identifies a branch during deserialization.

**Presence flag**:
The structural token, or one bit of a bit-packed presence token, recording whether a two-state optional member is present.

**Optional-field state**:
The structural token recording which of three states a nullable optional member is in (absent, explicit null, present); it keeps null distinct from omission.

**PAD token**:
`PAD_TOKEN`, the last ID of the generated vocabulary, after the base vocabulary and merge IDs; it is never valid as payload. Distinct from the sequence wrapper's PAD.

### Decoding

**Canonical token sequence**:
The one token sequence encoding emits for a value, with no normalization and only in-vocabulary IDs.

**Recovery decode**:
The default decode behavior: out-of-range finite choices and BPE IDs are normalized into range with modulo arithmetic so model-generated streams still decode, while truncation, malformed strings, and over-cap lengths still fail. Bounded-integer offsets are not normalized and can decode outside the declared bounds.

**Strict decode**:
The additive decode entry points that reject out-of-range finite choices, bounded-integer digits, and unknown BPE IDs instead of normalizing them. It is not a schema validator: value constraints stay unenforced.
_Avoid_: validating decode

**Whole-stream decode**:
A recovery or strict decode of one value that also rejects unconsumed trailing tokens (the `_full` entry points).
_Avoid_: full decode (see the ambiguities below)

**Prefix decode**:
Truncation-tolerant decoding of a left prefix of a canonical token sequence into a partial value with defaults, available in tokenizers generated with streaming; the README calls it streaming decode. It is not suffix-constrained infill.

### Compression

**Merge table**:
BPE merges, also called a BPE table or merge file, trained on schema-local token streams (not JSON text) for one schema and compiler configuration and embedded with merge IDs after the base vocabulary. It ships with a `*.provenance.json` sidecar.

**Target vocabulary**:
The total budget a merge table is trained to: the base vocabulary, the merges, PAD, and the reserved BOS, EOS, and schema slots.
_Avoid_: merge count (that is only the merges)

### Sequences and bundles

**Sequence wrapper**:
The model-facing framing BOS, schema token, payload, EOS around a schema-local token stream, with payload IDs offset so wrapper tokens never collide with them.

**Schema token**:
The wrapper token after BOS that names which schema's payload follows.

**Bundle**:
Several generated tokenizers combined under one sequence vocabulary by a bundle manifest that fixes the shared PAD, BOS, and EOS, one schema token per schema, and each schema's payload range.

**Prompt-response boundary**:
The contract for a pretrained prompt tokenizer followed by a TCT response span: prompt tokens keep their IDs, the bundle maps into one added-token block, and sentinels mark the span. TCT's guarantees hold only inside the span.

### Schema evolution

**Token diff report**:
The `tct-token-diff` comparison of two schema versions' token layouts (and optionally merge tables), which assigns a decision class. It is compiler-artifact evidence, not proof that a model can reuse anything.

**Decision class**:
The conservative reuse verdict of a token diff report: `exact_reuse`, `layout_only_unchanged`, `additive_compatible`, or `breaking_change`. Only `exact_reuse` allows reuse of tokenizer, merge-table, and embedding artifacts without review.
_Avoid_: compatibility level

## Flagged ambiguities

- **"raw"**: means unwrapped (BPE included) in the sequence-wrapper and wheel APIs, but pre-BPE in BPE and measurement contexts. Say which.
- **"discriminator"**: also used loosely for enum indices and for discriminator keys; this glossary reserves it for the union branch token.
- **"optimization pass"**: older text uses it for the optimization phase; here it means only the named passes inside that phase.
- **"full decode"**: means whole-stream decode in the API (`decode_full`) and plain non-prefix decode in prose.
- **"payload"**: the sequence-wrapper body, a union variant's data, or value-content tokens such as string bytes, depending on context.
