# StructuralModel Boundary Selection

> **Experimental-frontend note.** This document was written inside the private
> TCT research repository and is kept as a contract reference for the
> experimental non-JSON frontends, which are compiled only with the
> `experimental` Cargo feature. It may cite roadmap, report, or
> sibling-repository documents that are not part of this repository; treat
> those citations as historical context. The stable JSON Schema core is
> documented in `README.md`, `ARCHITECTURE.md`, and `DEVELOPMENT.md`.

This memo records the current boundary-selection result for the syntax-level
multi-language backend path. It is planning evidence, not a stable-backend
promotion and not permission to route `GrammarIR` or `StructuralModel` through
the core compiler.

## Decision

Decision: use `StructuralModel` as the candidate target-agnostic backend
boundary for syntax-level source work.

The selected sidecar path remains:

```text
target parser or grammar metadata
  -> GrammarIR and decision graph
  -> StructuralModel
  -> token layout
  -> generated codec
```

`GrammarIR` is upstream grammar input. It normalizes parser or grammar metadata
and should remain separate from the shared backend boundary. External parser
tooling, renderers, formatters, compiler checks, and functional validators stay
sidecar evidence until a later stable gate explicitly promotes one of them.
Adapter selection before `GrammarIR` is defined in
[`docs/grammar-ir-contract.md#adapter-choice`](grammar-ir-contract.md#adapter-choice);
this memo only selects the downstream structural boundary.

GoIR and C# arena IR were controls and migration fixtures; residual operational
surfaces were retired. They preserved parser, renderer, provenance, token-range,
and BPE migration evidence while they existed, but they are target-specific and
are not the stable multi-language backend boundary.

## Evidence Inputs

The selected boundary is based on the current sidecar evidence:

- The toy grammar path lowers checked `GrammarIR` into `StructuralModel` and
  proves records, variants, lists, optionals, finite domains, bounded integers,
  payload leaves, references, renderer metadata, validation hooks, token ranges,
  decision graph edges, and unchanged compiler-boundary flags.
- Tree-sitter-style metadata, Roslyn-backed C# metadata, and Go
  `go/parser` / `go/ast` metadata lower through the same
  `GrammarIR -> StructuralModel` boundary with source and parser provenance.
- The cross-target syntax decision comparison shows toy, Tree-sitter-style,
  C#, and Go metadata expose shared rules, fields, repetitions, references,
  payload leaves, payload-boundary edges, token-range roles, provenance, and
  unchanged compiler-boundary flags.
- The cross-target `StructuralModel` equivalence gate now compares matched
  Go/C# local constructs, the zero-argument invocation categories for the Go
  no-argument call-return and C# zero-argument invocation-return sidecars, and
  the argument-call categories for `go_local_argument_call_return_expression_syntax_v0`
  where `len("world")` carries a `CallExpr.Args -> BasicLit` edge comparable to
  the C# invocation argument literal edge. It also compares the Go/C# if/else
  control-flow categories for condition, then-body, else-body, branch
  assignment, and return payloads, plus the Go/C# condition-only loop
  categories for loop statement, condition, body, branch assignment, and
  return payloads, the Go/C# single-case switch and string-iteration
  categories, and the Go/C# callable-wrapper categories for declaration, name,
  return type, body block, and return literal over the integer-return syntax
  routes. The gate also compares callable parameter-list categories for the
  parameter-list edge, parameter binding, parameter name payload, and parameter
  type payload over the Go minimal-function and C# two-parameter method syntax
  routes. It now compares member-list container categories for the repeated
  member/declaration list, data/type member declaration, callable member
  declaration, member name payloads, and member type payloads over the Go
  package type declaration and C# property-plus-method syntax routes. It also
  compares data-member field/property categories for data-member container,
  data-member binding, member name payload, and member type payload over the Go
  struct type and C# property-plus-method syntax routes. It also compares
  method-member callable categories for method declaration/container, method
  name payload, return type payload, body block, return statement, and return
  expression edge over the Go method declaration and C# property-plus-method
  syntax routes. It also compares package/class data declaration categories for
  data declaration container, data declaration binding, data declaration name
  payload, data declaration type payload, initializer field profile, companion
  callable member, and companion return edge over the Go package var
  declaration and C# property-plus-method syntax routes. It also compares
  collection/composite initializer categories and index/element-access
  categories for callable/member container, parameter array/element type,
  index target identifier, bracketed/index argument list, and integer index
  payload over the Go index expression and C# element-access syntax routes. It
  now compares slice/range categories for callable/member container, parameter
  array/element type, range target identifier, slice/range expression, and
  integer lower/start payload over the Go slice expression and C# range
  element-access syntax routes. It also compares unary-expression categories
  for callable/member container, return body, unary expression edge, unary
  operator payload, and integer operand payload over the Go unary integer
  return expression and C# prefix-unary integer return syntax routes. It now
  compares binary-expression categories for callable/member
  container, return body, binary expression edge, binary operator payload, and
  left/right integer operand payloads over the Go and C# binary integer return
  syntax routes.
  The C# zero-argument empty argument-list wrapper, exact parameter counts, Go
  file/package/function-type wrappers, Go import wrappers, Go type-spec
  wrappers, Go struct-field wrappers, C# folded property binding, C# property
  initializer, Go package var keyword, Go package value-list cardinality, Go
  package initializer literal, C# property initializer string payload, Go
  receiver list/name/type binding, Go method return literal, C# method
  return-identifier spelling, member ordering, C#
  namespace/class/member wrappers, C# element-access argument-list wrappers,
  Go open-ended high-bound absence, C# bounded range right operand, C# range
  argument-list wrappers, Go `UnaryExpr.Op`/`UnaryExpr.X` spelling, C#
  `PrefixUnaryExpressionSyntax.OperatorToken`/`Operand` spelling, Go operator tokens, C#
  else clause wrappers, C# loop header setup fields, and C# type/operator
  tokens remain target-specific syntax evidence.
- The generated syntax codec gate compiles toy, C#, and Go sidecar-generated
  codecs and now covers reproducibility metadata, strict/recovery decode
  behavior, consumed-token accounting, partial-result accounting, token-layout
  invariants, and BPE sidecar evidence without touching `TctCompiler`.
- Historical GoIR StructuralModel v0 sidecars showed arena-route migration
  pressure and token-layout handoff shape, but they were retired and do not
  replace syntax-level lowering.

## Boundary Profile

Before `StructuralModel` can become a stable backend boundary, a maintained
conformance gate must prove this profile across the syntax sidecars:

- Stable `structural-model-v0` identity and unchanged compiler-boundary flags.
- Shared records, variants, lists, optionals, references, payload leaves,
  finite domains, token ranges, renderer metadata, validation hooks, decision
  graph nodes, and decision graph edges.
- Source metadata and parser provenance for parser-backed targets.
- Target-specific renderer and validator facts kept as metadata, not generated
  hot-path logic.
- Token-layout facts sufficient to prove `VOCAB_SIZE`, `PAD_TOKEN`, raw
  structural ranges, payload ranges, and optional BPE boundaries do not collide.
- No dependency on GoIR or C# arena schema shape for stable backend semantics.
- No dependency on generated Rust source substrings as semantic evidence.

## Non-Goals

Do not route `GrammarIR` or `StructuralModel` through the core compiler from
this memo. Do not add production Tree-sitter, Roslyn, Go parser, ANTLR, or
grammar-ingestion paths from this memo. Do not treat a single target's source
fixture or arena route as a stable backend claim.

## Maintained Conformance Gate

`tests/syntax_structural_model_conformance_test.rs` is the maintained syntax
`StructuralModel` boundary conformance gate.

The gate lowers the toy, C#, and Go syntax sidecars and asserts the shared
boundary profile above in one maintained test. It keeps the implementation
sidecar-only, compares only structured `StructuralModel` fields, and leaves
`TctCompiler` and generated JSON Schema output unchanged.

## Maintained Boundary Manifest

`tests/fixtures/syntax_structural_model/boundary-manifest-v1.json` is the
versioned syntax `StructuralModel` boundary fixture manifest. It records the
toy, C#, and Go sidecar targets, source kind, parser provenance where
available, source or metadata hashes, `structural-model-v0` identity,
sidecar-boundary flags, structural category coverage, token-range roles, and
proof checks. The manifest is derived from structured `StructuralModel` fields,
not generated Rust source substrings.

## Stable-Gate Integration Plan

The stable-gate integration plan is
`docs/syntax-structural-model-stable-gate-plan.md`.

The plan maps the boundary manifest onto the JSON-backend-grade gate categories
in `docs/syntax-backend-gate-strategy.md`, names the first concrete
stable-gate implementation hook that can consume it, and keeps `GrammarIR` and
`StructuralModel` outside the production JSON Schema compiler until that gate
exists.

## Next Failing Task

Next failing task: pair the parser-backed Go and C# binary integer return
generated-codec routes in a `StructuralModel` equivalence slice before claiming
any stable `GrammarIR` or `StructuralModel` route through `TctCompiler`.

The generated-code gate now consumes the boundary manifest and records manifest
identity before generated codec evidence is promoted: manifest artifact kind,
version, hash, target ids, and the sidecar-only boundary claim are present in
generated-code reproducibility metadata. The stable-gate helper now runs the
manifest-backed generated-code gate, the `StructuralModel` conformance gate,
and the syntax backend audit inventory as an explicit sidecar validation
bundle. The M6 cross-target contract closure report
`reports/m6-cross-target-contract-closure-2026-07-21.md` records shared
Go/C# contract evidence (layer-3 stable promotion remains a separate track), and
`tests/fixtures/syntax_backend/production-boundary-contract-v1.json` pins the
future production handoff while staying sidecar-only. The production generator
API boundary is now `src/syntax_backend_boundary.rs`, with focused coverage in
`tests/syntax_backend_boundary_test.rs`. The generated artifact envelope fixture
is now `tests/fixtures/syntax_backend/generated-artifact-envelope-v1.json`, with
validation support in `src/syntax_artifact_envelope.rs`, and the generated-code
gate records its identity, fixture hash, target IDs, persistence policy, decode
policy, and token-layout metadata for toy, C#, and Go. The validation-hook
contract fixture is now
`tests/fixtures/syntax_backend/validation-hook-contract-v1.json`, with
validation support in `src/syntax_validation_hooks.rs`, and the generated-code
gate records its identity, fixture hash, target IDs, source checks,
renderer/formatter sidecar status, external-check non-promotion policy,
provenance fields, and non-goals for toy, C#, and Go. The diagnostics contract
fixture is now
`tests/fixtures/syntax_backend/input-validation-diagnostics-v1.json`, with
validation support in `src/syntax_input_diagnostics.rs`, and pins target IDs,
checked input fields, diagnostic fields, unsupported syntax error shape,
source-or-metadata SHA-256, parser-or-grammar identity, diagnostics
provenance, sidecar-only status, and non-goals. The generated-code gate now
records that input diagnostics contract identity, fixture hash, target IDs,
required fields, unsupported syntax error shape, provenance, sidecar status,
and non-goals for toy, C#, and Go. The negative checked-input fixture gate is
now
`tests/fixtures/syntax_backend/input-validation-negative-fixtures-v1.json`, and
it proves malformed and unsupported toy, C#, and Go records with rule/path
diagnostics, source-or-metadata SHA-256, grammar or parser identity, sidecar
boundary SHA-256, and diagnostics contract SHA-256. The validation-hook sidecar
evidence fixture is now
`tests/fixtures/syntax_backend/validation-hook-sidecar-evidence-v1.json`, and it
pins sidecar command identity, renderer/formatter status, validation-hook IDs,
external policy hashes, contract hashes, and generated-code handoff metadata.
The generated artifact replay fixture is now
`tests/fixtures/syntax_backend/generated-artifact-replay-v1.json`, with
validation support in `src/syntax_reproducibility_replay.rs`, and it ties toy,
C#, and Go volatile generated-code report metadata back to the checked syntax
sidecar identities. The production `TctCompiler` routing gate is now
`src/syntax_tct_compiler_routing.rs`, with fixture coverage in
`tests/fixtures/syntax_backend/tct-compiler-routing-gate-v1.json`; it records
how a future route would hand checked `GrammarIR -> StructuralModel` inputs to
the syntax renderer API in `src/syntax_generated_codec.rs` while keeping the
route disabled. The JSON-backend parity gate is now
`src/syntax_json_backend_parity.rs`, with fixture coverage in
`tests/fixtures/syntax_backend/json-backend-parity-routing-gate-v1.json`; it
pins the existing JSON Schema backend surfaces and stable gate commands that
must stay green before a future syntax route can be registered. The
target-agnostic routing boundary gate is now
`src/syntax_target_agnostic_routing_boundary.rs`, with fixture coverage in
`tests/fixtures/syntax_backend/target-agnostic-routing-boundary-gate-v1.json`;
it pins checked `GrammarIR -> StructuralModel` as the route input and rejects
GoIR, C# arena IR, and target-specific schema shapes. The strict/recovery
decode routing gate is now
`src/syntax_strict_recovery_decode_routing.rs`, with fixture coverage in
`tests/fixtures/syntax_backend/strict-recovery-decode-routing-gate-v1.json`;
it pins strict decode authority, trailing `PAD_TOKEN` recovery normalization,
invalid parser-backed payload rejection, and partial-prefix hard errors. The
consumed-token accounting routing gate is now
`src/syntax_consumed_token_accounting_routing.rs`, with fixture coverage in
`tests/fixtures/syntax_backend/consumed-token-accounting-routing-gate-v1.json`;
it pins complete-stream, trailing-`PAD_TOKEN` recovery, hard-error, and
partial-prefix consumed-token and partial-result accounting. The
token/vocabulary/BPE routing gate is now
`src/syntax_token_vocabulary_bpe_routing.rs`, with fixture coverage in
`tests/fixtures/syntax_backend/token-vocabulary-bpe-routing-gate-v1.json`; it
pins `PAD_TOKEN`, `VOCAB_SIZE`, raw base vocabulary, structural ranges, payload
ranges, optional BPE ranges, non-collision checks, BPE artifact fields, and
emitted-token bounds for the route boundary. The generated-artifact
reproducibility routing gate is now `src/syntax_reproducibility_routing.rs`,
with fixture coverage in
`tests/fixtures/syntax_backend/reproducibility-routing-gate-v1.json`; it pins
deterministic hash fields, volatile report fields, source fixture hashes, replay
ownership, and sidecar-only route state for the future syntax route boundary.
The audit coverage routing gate is now
`src/syntax_audit_coverage_routing.rs`, with fixture coverage in
`tests/fixtures/syntax_backend/audit-coverage-routing-gate-v1.json`; it pins
the maintained audit inventory, route-boundary modules and fixtures, helper
commands, readiness report, no-route claims, and fail-on-gaps requirement for
the future syntax route boundary. The stable-promotion review gate is now
`src/syntax_stable_promotion_review.rs`, with fixture coverage in
`tests/fixtures/syntax_backend/stable-promotion-review-gate-v1.json`; it pins
the completed route-boundary evidence and records that the next slice may add
the first production route implementation while preserving the JSON Schema
backend contract. The production route implementation gate is now
`src/syntax_production_route.rs`, with fixture coverage in
`tests/fixtures/syntax_backend/production-route-implementation-gate-v1.json`;
it pins `compile_checked_syntax_production_route`, the target-agnostic
`compile_checked_syntax_compiler_route` entry point, the `syntax-production-route-v0`
handoff, checked `GrammarIR -> StructuralModel` input, and JSON backend parity
without promoting the syntax backend as stable.
The Go local declaration, local assignment, escaped local assignment, short
variable declaration, typed local declaration, zero-value typed local
declaration, integer typed local declaration, untyped integer local
declaration, integer local assignment, integer short variable declaration,
integer return literal, parenthesized integer return expression, string return
literal, boolean return identifier, nil return identifier, unary integer
return expression, binary integer return expression, call return expression,
and argument call return expression route tranches now pass through that route
with parser metadata, generated-code round trips, strict/recovery decode,
canonical rendering, and local `gofmt` / Go sidecar validation. The argument
call route is the `len("world")` parser-backed case. The current workstream is
expanding matched Go/C# `StructuralModel` categories after the checked
five-record Go whole-file generated-codec corpus claim. It now includes
matched if/else, condition-only loop, single-case switch, and string-iteration
control-flow evidence plus callable-wrapper declaration, name, return-type,
body-block, return-literal, exact zero-argument invocation, callable
  parameter-list evidence, member-list container evidence, data-member
  field/property evidence, method-member callable evidence, collection/composite
  initializer evidence, index/element-access evidence, slice/range evidence,
  unary-expression, binary-expression, parenthesized-expression evidence, float
  return literal evidence, boolean return evidence, and nil/null return
  evidence over route-backed Go/C# syntax metadata. The boolean and nil/null
  parity gates compare callable wrapper, return type, return statement, and
  payload leaf evidence while keeping Go `Ident.Name` and C#
  `LiteralExpressionSyntax.Token` spelling target-specific. The C# nullable
  return type route now preserves `string?` as target-specific syntax evidence,
  and that route is pinned in the sidecar-only source-program typecheck control
  envelope. The C# nullable local declaration route
  `csharp_local_nullable_declaration_syntax_v0` now preserves `NullableTypeSyntax`
  on a local declaration with a null initializer as target-specific syntax
  evidence and is pinned in the sidecar-only source-program typecheck control
  envelope. The C# nullable property declaration route
  `csharp_nullable_property_declaration_syntax_v0` now preserves
  `NullableTypeSyntax` on a property with accessors and a null initializer as
  target-specific syntax evidence and is pinned in the sidecar-only
  source-program typecheck control envelope. The C# nullable parameter
  declaration route `csharp_nullable_parameter_declaration_syntax_v0` now
  preserves `NullableTypeSyntax` in callable signature context as
  target-specific syntax evidence and is pinned in the sidecar-only
  source-program typecheck control envelope. The C# nullable context
  `StructuralModel` equivalence gate now compares the return, local, property,
  and parameter nullable contexts by asserting each parser-backed
  `NullableTypeSyntax` owner reference, the shared `NullableTypeSyntax` element
  type reference, and the `QuestionToken` string payload leaf. The C# fixture
  route-corpus toolchain evidence preservation gate now pins the checked
  `tests/fixtures/csharp_syntax_sources/sources.txt` manifest, corpus fingerprint,
  `dotnet` toolchain envelope, and route-decision readiness fields as
  sidecar-only evidence. The next workstream slice is a representative C#
  real-source corpus selection gate, without broadening
  source-program validity, stable backend, paper, model-quality,
  type-correctness, completion, generated hot-path, or arena-IR claims.
