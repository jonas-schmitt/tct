# M6 Cross-Target Contract Closure

Date: 2026-07-21  
Branch: `go-dev`  
Audit base still retained: `99cbf6654`  
Pre-closure HEAD: `2f98fa1c7543b98aaf11692243d3cceb5bdafe4b`  
Companion JSON: `reports/m6-cross-target-contract-closure-2026-07-21.json`

## Decision

**M6 is satisfied** for the North Star scope: a target-agnostic raw
syntax-state-machine boundary is stable enough for Go and one structurally
distinct second grammar (wired C# `minimal_class` / shared C# real-source
state-API surface) to share state, transition, admissible-token, decode/error,
token layout, and validation contracts.

No new suite or C# breadth work is required. Residual work after this closure
is outside M6: GPU-blocked M5/M7 training, I1 Go-only metric pilot, external
stress corpus, stable-backend promotion (layer 3), and stage-5 reviewability
extractions.

## Scope And Non-Claims

In scope:

- Shared contracts across Go and C# at the syntax-level boundary
  (`parser-backed metadata -> GrammarIR -> StructuralModel -> token layout ->
  generated codec / raw state machine`).
- Wired C# `minimal_class` as the cheap second grammar, plus the existing
  five-record C# real-source state-API handoff surface that consumes the same
  target-neutral API.
- Matched Go/C# `StructuralModel` category equivalence already retained in
  `tests/syntax_structural_model_cross_target_equivalence_test.rs`.

Out of scope / not claimed:

- C# breadth parity with Go stdlib.
- `TctCompiler` production routing / stable-backend promotion (layer 3).
- Source-program type correctness, paper results, model quality, BPE as
  validity authority, Arena IR / GoIR revival.
- Compile-time benchmarks as correctness gates (volatile host metadata only).

## Contract Map

| Contract category | Shared surface | Go retained gate | C# retained gate | Result |
| --- | --- | --- | --- | --- |
| **State** (prefix → state) | `state_after_raw_prefix` on `syntax-generated-codec-state-api-surface-v1` | `shared_generated_codec_state_api_handoff_consumes_go_and_csharp_state_api_reports`; Go source-instance state-API reports (142 records in handoff fixture) | Same handoff + `csharp_state_machine_parity_consumes_shared_replay_api_contract`; C# scratch empty-prefix seeds (`csharp_minimal_class_*`) | Covered |
| **Transition** | `candidate_raw_token_transition` | `transition_acceptance_matches_admissible_mask_over_checked_corpora`; handoff transition cases | `csharp_minimal_class_scratch_seed_uses_admissible_raw_state_machine_transitions`; C# parity selected-completion traces | Covered |
| **Admissible-token** | `admissible_next_raw_token_ids` / ranges | Shared handoff admissible sets; `matched_go_csharp_presence_masks_use_target_neutral_state_api` | `matched_go_csharp_presence_masks_use_target_neutral_state_api` | Covered |
| **Strict/recovery decode + consumed-token accounting** | `strict_decode`, `recovery_decode`, `consumed_raw_token_accounting` | Handoff Go strict/recovery reports; routing gates `strict-recovery-decode-routing-gate-v1.json`, `consumed-token-accounting-routing-gate-v1.json` | C# parity `consumed_token_accounting_ok` + scratch seed recovery/incomplete-prefix rejection | Covered |
| **Token layout** | Per-target layout + non-collision / `VOCAB_SIZE` / `PAD_TOKEN` roles | `go_syntax_token_layout` / source-file layout (vocab 641, pad 640, 128 structural); `token_roles_validation_rejects_malformed_layouts`; boundary manifest Go counts | C# layout (product vocab 577 / pad 576; conformance pin uses 321/320 structural+UTF-8 slice for `minimal_class`); boundary manifest C# counts; layout pin gate | Covered |
| **Validation** | Shared profile + matrix markers | `syntax_structural_model_conformance_test` (toy+go); `./scripts/testing/test-syntax-backend-gates.sh`; boundary suite | Same conformance (csharp `minimal_class`); validation-matrix and backend-boundary markers include `csharp_minimal_class_syntax_v0` | Covered |
| **StructuralModel cross-target shape** (supporting) | Matched construct categories | Full `syntax_structural_model_cross_target_equivalence_test` (27 tests) | Same (Go↔C# pairs; C# control-flow readiness gate) | Covered |

Primary executable anchors (no new tests added for closure):

```bash
cargo test --test syntax_structural_model_cross_target_equivalence_test
cargo test --test syntax_structural_model_conformance_test
cargo test --test syntax_state_machine_replay_test shared_generated_codec
cargo test --test syntax_state_machine_replay_test csharp_minimal_class_scratch
cargo test --test syntax_state_machine_replay_test csharp_state_machine_parity
cargo test --test syntax_state_machine_replay_test target_neutral_generated
cargo test --test syntax_state_machine_replay_test matched_go_csharp_presence
cargo test --test syntax_state_machine_replay_test transition_acceptance_matches
cargo test --test syntax_state_machine_replay_test token_roles_validation
./scripts/testing/test-syntax-backend-gates.sh
```

## Stable Artifact Facts

Pinned boundary-manifest facts from
`tests/fixtures/syntax_structural_model/boundary-manifest-v1.json` (sidecar
`structural-model-v0` profile; not compile-time benchmarks):

| Target | Grammar id | Rules / records | Decision-graph nodes / edges | Token-range roles (count) | Notes |
| --- | --- | --- | --- | --- | --- |
| toy | `toy_assignment_v1` | 2 / 2 | 10 / 8 | structural + utf8 + u8 int (3) | Control grammar |
| csharp | `csharp_minimal_class_syntax_v0` | 5 / 5 | 23 / 18 | structural + utf8 (2) | M6 second grammar |
| go | `go_minimal_function_syntax_v0` | 12 / 12 | 53 / 41 | structural + utf8 + u8 int (3) | Matched minimal Go slice |

Shared state-API handoff fixture
(`tests/fixtures/syntax_backend/shared-generated-codec-state-api-handoff-v1.json`):

- Status: `shared_go_csharp_generated_codec_state_api_handoff_ready`
- Capabilities: state, transition, EOF, strict/recovery decode, consumed-token
  accounting, admissible IDs/ranges, BPE-as-expansion-only policy
- Go evidence: 142 source records; payload-range and EOF-valid state counts 142
- C# evidence: 5 source records; payload-range and EOF-valid state counts 5
- Claim boundary: target-neutral handoff only; stable backend, paper, type
  correctness, and BPE-as-validity claims remain blocked

Product token layouts (from `src/syntax_token_layout.rs`; distinct from the
minimal conformance pin layouts used only in the StructuralModel boundary
manifest test):

- Go construct / source-file layouts: `vocab_size=641`, `pad_token=640`
- C# product layout: `vocab_size=577`, `pad_token=576`

## Implementation Fix In This Tranche

Validation of the syntax backend gate suite found a **stale pin cascade**, not
a missing contract:

- `tests/fixtures/syntax_structural_model/boundary-manifest-v1.json` still
  recorded old `metadata_sha256` values for C# `minimal_class` and Go
  `minimal_function` after the on-disk metadata files changed.
- Live recompute metadata hashes:
  - csharp: `cef195ae3315ca24cd3a11b2ef3a407c85f08168390ae585861940a18bad8e93`
  - go: `226b7373a59f9d91b30f4700b73a9d31e27dd4e95a1c80859d0eaa86420b42f6`
- Structural counts and grammar shapes were already correct; only provenance
  hashes were refreshed.
- Downstream generated-codec replay pins in
  `tests/fixtures/syntax_backend/generated-artifact-replay-v1.json` track the
  compact-JSON `boundary_manifest_sha256` /
  per-target `target_manifest_sha256` identities and were updated to match:
  - boundary: `815cc872f4998f8cd53ce021d5d922e841561cea233ec43dd04c3d6d6d570aab`
  - csharp target: `62bfd7f74a7402630ed4e61825fa77276be10985d43059abcbdff0683130014d`
  - go target: `582e3cff27d75996b05984e9aa4d41b7dc04b9362eb53c490d421e14f2ee5e89`
  - toy target unchanged

## Validation Recorded

- `cargo test --test syntax_structural_model_cross_target_equivalence_test` —
  27 passed
- `cargo test --test syntax_structural_model_conformance_test` — pass after
  hash pin refresh
- Focused `syntax_state_machine_replay_test` filters listed above — pass
- `./scripts/testing/test-syntax-backend-gates.sh` — re-run after pin fix

## Follow-On

1. **I1** — Go-only entropy-budget metric contract and pilot (CPU research
   ceiling).
2. Go external stress corpus (pinned modules; gap-driven).
3. I2 pause-token protocol freeze (design/fixtures only without GPU).
4. Layer-3 stable promotion remains a separate promotion track, not an M6
   blocker.
