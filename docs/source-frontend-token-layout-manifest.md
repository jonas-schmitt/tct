# Source Frontend Token Layout Manifest

> **Experimental-frontend note.** This document was written inside the private
> TCT research repository and is kept as a contract reference for the
> experimental non-JSON frontends, which are compiled only with the
> `experimental` Cargo feature. It may cite roadmap, report, or
> sibling-repository documents that are not part of this repository; treat
> those citations as historical context. The stable JSON Schema core is
> documented in `README.md`, `ARCHITECTURE.md`, and `DEVELOPMENT.md`.

This document defines the report-only layout contract that source-language
frontends must preserve before TCT shares frontend runtime code or moves
`StructuralModel` into the compiler path.

The contract records layout facts. It does not parse source, render source,
change JSON Schema compilation, or allocate new generated tokenizer ranges.

The legacy JSON-Schema-backed GoIR control route and its checked layout
manifest fixtures were retired with the GoIR residual investigation family.
Do not regenerate GoIR-era token-layout manifests, `tct-goir-evidence`
reports, or `source_frontend_token_layout_manifest.py` workflows. Forward
syntax/grammar routes preserve the same layout *facts* at the parser-backed
syntax metadata boundary (inventory-derived Go artifact, C# `minimal_class`,
and shared raw-state gates).

## Contract

Each layout record must preserve:

- Frontend identity: frontend id, source language, and source kind.
- Source provenance: corpus id, corpus source, license label, source file hash,
  syntax metadata or target-control IR file hash, and corpus fingerprint kind.
- Model identities: source IR id, `StructuralModel` id and version, renderer
  id, and token-layout id.
- Token layout: optimization profile, `BASE_VOCAB_SIZE`, `VOCAB_SIZE`,
  `PAD_TOKEN`, max observed emitted token, structural ranges, payload ranges,
  optional BPE ranges, wrapper-token offsets, and compatibility class.
- Collision checks: pad outside the base range, pad below `VOCAB_SIZE`, max
  observed token below `VOCAB_SIZE`, observed generated tokens excluding
  `PAD_TOKEN`, BPE absence or non-collision, unique wrapper offsets, and the
  current contextual structural/payload range policy.
- Runtime boundary: parser, renderer, formatter, validator, and wrapper work
  stay outside generated tokenizer hot paths.

For schema-local generated tokenizers, structural and payload entries may share
the base token interval when token meaning is contextual. Model-facing BOS,
EOS, schema-decision, and bundle tokens are wrapper offsets; raw generated
tokenizers do not contain them.

## Compatibility Class

The durable compatibility class for content-only generated tokenizers is
`raw_generated_tokenizer_layout_v1`. That class means:

- The raw generated tokenizer owns schema-local content tokens only.
- `PAD_TOKEN` is outside the base token interval and below `VOCAB_SIZE`.
- Observed generated tokens exclude `PAD_TOKEN`.
- BPE is absent unless an explicit BPE range and artifact are supplied.
- Wrapper offsets are model-facing metadata and are not raw generated tokens.
- Parser, renderer, formatter, and functional validators remain sidecar
  evidence outside the generated tokenizer hot path.

Any future layout change must either preserve this class or introduce a new
class with a layout-diff gate, focused tests, and updated handoff docs before
shared frontend runtime code depends on it. Treat non-`exact_reuse` layout or
stream-contract changes like a `tct-token-diff` breaking change for downstream
raw-token corpora and BPE artifacts.
