# Hugging Face-Loadable TCT Sidecar Artifact Contract

This document defines the mid-term TCT-side target for artifacts that downstream
Hugging Face workflows can load as structured-response sidecars. It is not a
full `AutoTokenizer` or `tokenizer.json` compatibility promise, and it is not a
claim that this repository owns model fine-tuning or semantic-quality
evaluation.

## Scope

TCT remains responsible for:

- Generated schema-specific tokenizer wheels and metadata.
- Raw schema-local `encode`, `decode`, optional `decode_prefix`, vocabulary, and
  token-range contracts.
- Sequence-boundary wrappers with explicit PAD, BOS, EOS, schema-decision, and
  payload-token mapping.
- Multi-schema bundle manifests and hybrid prompt/TCT response manifests.

Downstream repositories remain responsible for:

- Adding TCT rows to a pretrained tokenizer and model embedding/output head.
- Hugging Face `ProcessorMixin`, `PreTrainedTokenizerBase`, or training-loop
  integration experiments.
- Fine-tuning, embedding initialization, generation policy, semantic-quality
  evaluation, and checkpoint publication.

## Minimal Schema Wheel API

A schema-specific generated wheel used by a Hugging Face wrapper should expose
the current stable generated-wheel surface:

- `encode(json_str) -> list[int]`: raw schema-local payload tokens.
- `decode(tokens) -> (json_str, consumed, total)`: raw schema-local decode with
  explicit consumed-token accounting.
- `decode_prefix(tokens) -> (json_str, fields_decoded, complete)` when generated
  with streaming support.
- Raw metadata helpers: `vocab_size()` and `pad_token()`.
- Sequence metadata and helpers when model-facing boundaries are needed:
  `encode_sequence`, `decode_sequence`, `bos_token`, `eos_token`,
  `schema_token`, `sequence_pad_token`, `sequence_payload_offset`,
  `sequence_boundary_version`, `sequence_vocab_size`, and
  `sequence_metadata_json`.

The single-schema `sequence_metadata_json()` payload is the source of truth for
sequence metadata. It records the artifact kind/version, sequence-boundary
version, schema id and schema-decision token, raw `BASE_VOCAB_SIZE`,
`VOCAB_SIZE`, and `PAD_TOKEN`, base/BPE/payload ranges, payload offset, sequence
vocabulary size, sequence PAD/BOS/EOS/start/end tokens, reserved wrapper-token
range, whether `decode_prefix` is supported, and BPE merge metadata when a merge
table is embedded.

Raw schema-local APIs are content-only. They do not emit Hugging Face prompt
tokens, BOS, EOS, schema sentinels, or padding. Model-facing wrappers add those
tokens using the sequence and bundle manifests.

## Bundle And Prompt Boundary Manifests

The repo-owned metadata shape is `tct::sequence_bundle`:

- `SequenceBundleManifest` defines the bundle vocabulary, shared sequence
  PAD/BOS/EOS policy, masking policy, schema-decision tokens, optional schema
  sentinels, raw schema vocabulary and PAD values, raw payload ranges, global
  payload ranges, BPE flags, and optional generated artifact references.
- `PromptResponseBoundaryManifest` maps the validated TCT bundle vocabulary into
  a contiguous downstream Hugging Face added-token block and records
  `<|tct_start|>` / `<|tct_end|>` mode sentinels.
- Pure-Python bundle wrappers expose `encode_sequence(schema_id, json_str)`,
  `decode_sequence(tokens) -> (schema_id, json_str, consumed, total)`,
  `sequence_bundle_metadata_json`, `sequence_vocab_size`,
  `sequence_pad_token`, `bos_token`, `eos_token`, `schema_token(schema_id)`,
  `generation_mask_for_schema_token(schema_token)`, and `SequenceBundleError`.
- Prompt tokens keep the pretrained tokenizer's native IDs. TCT bundle IDs map
  to downstream IDs as `hf_added_token_block.start + bundle_token_id`.
- The TCT added-token block must start at or after the prompt tokenizer's first
  available token ID and must have exactly the bundle vocabulary length.

Downstream training wrappers may instead use schema sentinels,
`<|tct_start|>`, `<|tct_end|>`, and deterministic TCT token strings such as
`<|tct:000000|>`. This repository's stable contract is the numeric contiguous
block and manifest mapping; downstream wrappers may choose the concrete token
string template as long as it is bijective and saved with the tokenizer.

Such a wrapper can use a direct local-payload handoff:

```text
prompt ids, schema sentinel, <|tct_start|>, local payload ids, <|tct_end|>
```

That shape does not carry the canonical bundle response
`[BOS, schema token, payload..., EOS]` inside the Hugging Face TCT block; treat
it as a versioned compatibility profile, not as the stable handoff. Even in the direct profile, generated raw
`PAD_TOKEN` is not payload: valid local payload IDs are the raw payload range,
normally `[0, pad_token())`, not the full `[0, vocab_size())`.

Decision: preserve both handoff profiles explicitly for now. The canonical
repo-owned profile is `PromptResponseBoundaryManifest` over bundle sequences,
because it records BOS/EOS, schema-decision tokens, payload ranges, masking
policy, manifest hashes, and save/load metadata in one place. The
downstream direct profile remains a versioned downstream compatibility
profile for prompt-plus-response fine-tuning experiments that already use
schema sentinels, `<|tct_start|>`, `<|tct_end|>`, and a shared payload block.
Do not silently migrate existing downstream artifacts between these profiles:
record the profile name/version, use the matching mask and EOS contract, and
convert only through an explicit compatibility step.

For the current direct profile, TCT exposes
`DirectHfTctCompatibilityProfile` through
`PromptResponseBoundaryManifest::direct_local_payload_profile_for_schema(...)`.
That record is intended to be serialized beside downstream results. It names
the direct profile, schema sentinel, mode-entry and mode-exit tokens, raw
payload bounds, raw PAD rejection, generation mask, label-mask policy, and the
fact that `<|tct_end|>` is a global Hugging Face special token rather than a
schema-local TCT or bundle EOS token. Downstream compatibility gates should
call `DirectHfTctCompatibilityProfile::validate()` on the preserved payload and
compare `profile_sha256()` against result metadata before treating an artifact
as using this exact handoff profile.

For file-based result metadata checks, use the maintained TCT-side gate:

```bash
cargo run --bin tct-validate-direct-hf-profile -- \
  --profile path/to/direct-hf-tct-profile.json \
  --expected-sha256 recorded-profile-sha256
```

The helper rejects stale profile names, unsupported profile versions, raw PAD as
payload, invalid raw-PAD HF mappings, masking-policy drift, generation-mask
drift, and profile hash mismatches.

## Special Tokens And Masking

Do not overload raw schema-local PAD as BOS or EOS in new Hugging Face-oriented
artifacts.

The wrapper-level policy is:

- Prompt tokenizer special tokens remain owned by the pretrained tokenizer.
- TCT response spans use explicit TCT mode start and end sentinels.
- Bundle sequences use explicit sequence PAD, BOS, EOS, and one schema-decision
  token per selected schema.
- TCT PAD is padding-only and label-masked.
- TCT BOS/start and schema-decision tokens are label-masked.
- TCT EOS/end is an unmasked target so generation can learn to stop.
- After schema selection, generation logits are restricted to the selected
  schema's global payload range plus EOS.

This policy intentionally differs from older downstream baseline
checkpoints that reuse PAD as BOS and do not learn a model-emitted EOS.

## Save/Load Artifact Target

The minimal reloadable artifact beside a Hugging Face checkpoint should contain:

```text
checkpoint-or-artifact-dir/
  tct-artifact.json
  sequence_bundle_manifest.json
  prompt_response_boundary_manifest.json      # optional hybrid use
  schemas/<schema_id>/sequence_metadata.json
  schemas/<schema_id>/<generated wheel or import reference>
  schemas/<schema_id>/bpe-merges.json         # optional provenance artifact
```

The artifact envelope should carry load concerns that do not belong inside
`SequenceBundleManifest`: package/contract version, prompt tokenizer directory
or external identifier, generated schema wheel paths or importable module
references, manifest hashes, schema hashes, generated artifact hashes, compiler
configuration, BPE merge artifact provenance, and sequence-boundary version.

Loading should import existing generated wheels, validate metadata versions and
token ranges, and verify that the downstream tokenizer's added-token block is
contiguous and matches `PromptResponseBoundaryManifest`. It should not
regenerate compiler output. Preserve `manifest_sha256()` for serialized
`SequenceBundleManifest` and `PromptResponseBoundaryManifest` payloads when the
artifacts are used as cross-repo experiment evidence.

Do not create a repository-root `tct-bundle/` artifact layout until this
contract stabilizes through downstream use. For now, `tct-bundle/` is a future
artifact layout name. The current repo-supported bundle path is the serializable
manifest plus the pure-Python source-package generator around already generated
schema wheels.

## Processor Decision

This repository should not ship a Hugging Face `ProcessorMixin`-style class yet.
The near-term repo-owned surface is:

- Schema-specific generated wheels.
- Bundle and prompt-boundary manifests.
- Pure-Python bundle wrapper package generation.
- Tests that verify sequence wrapping, unwrapping, range validation, EOS/PAD
  handling, and masking metadata.

A `ProcessorMixin` or tokenizer subclass should remain downstream until at
least one fine-tuning workflow proves the stable save/load surface and failure
modes. If that API stabilizes, this repository can later ship a lightweight
adapter that consumes the same manifests without owning training logic.

## Evidence Policy

Hugging Face compatibility is integration infrastructure. It is not evidence
that pretrained fine-tuning preserves semantic quality. Any such claim needs
downstream experiments with reproducible metadata: base model, prompt tokenizer,
schema set, generated wheels, manifest hashes, embedding initialization,
training data, generation masks, decoding policy, random seeds, checkpoints, and
evaluation reports.
