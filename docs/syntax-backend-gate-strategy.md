# Syntax Backend Gate Strategy

> **Experimental-frontend note.** This document was written inside the private
> TCT research repository and is kept as a contract reference for the
> experimental non-JSON frontends, which are compiled only with the
> `experimental` Cargo feature. It may cite roadmap, report, or
> sibling-repository documents that are not part of this repository; treat
> those citations as historical context. The stable JSON Schema core is
> documented in `README.md`, `ARCHITECTURE.md`, and `DEVELOPMENT.md`.

This document defines the gate strategy for a future syntax-level backend. It
is a planning contract, not a stable-backend promotion and not permission to
route `StructuralModel` through the core JSON Schema compiler path.

The current conclusion is mandatory: multi-language source work must enter TCT
at the syntax level. Target-specific arena IRs such as historical GoIR and C#
arena fixtures were controls and migration evidence only; residual operational
surfaces are retired, and they are not the stable backend boundary. The shared
path is:

```text
target parser or grammar metadata
  -> GrammarIR and decision graph
  -> StructuralModel
  -> token layout
  -> generated codec
```

`GrammarIR -> StructuralModel` is still sidecar evidence only. The checked
lowerings must keep generated JSON Schema output unchanged until the syntax
backend has JSON-backend-grade gates for its own boundary.
Adapter choice before `GrammarIR` is not redefined here; see
[`docs/grammar-ir-contract.md#adapter-choice`](grammar-ir-contract.md#adapter-choice)
for the current language-specific, Tree-sitter, and ANTLR policy.
`docs/structural-model-boundary-selection.md` records the current boundary
selection result, the maintained conformance gate, and the manifest-backed
stable-gate handoff plan before any core compiler routing.
`reports/m6-cross-target-contract-closure-2026-07-21.md` records M6
cross-target contract closure evidence for Go and C# `minimal_class`
(layer-3 stable promotion remains a separate track).
The completed `syntax backend cross-target StructuralModel equivalence gate` remains the retained executable
StructuralModel parity surface for that cross-target map.
`tests/fixtures/syntax_backend/production-boundary-contract-v1.json` pins the
future production handoff shape while keeping that handoff sidecar-only.
`src/syntax_backend_boundary.rs` consumes that contract fixture as the
sidecar-only production generator API boundary without routing syntax data
through `TctCompiler`.
`src/syntax_production_route.rs` owns the feature-limited stable production
route metadata:
`compile_checked_syntax_production_route` accepts checked
`GrammarIR -> StructuralModel` sidecars and delegates to the
`src/syntax_generated_codec.rs` renderer API without changing JSON Schema
compiler passes or generated JSON Schema output. The target-agnostic compiler
route entry point, `compile_checked_syntax_compiler_route`, infers C# and Go
targets from the checked route construct table before delegating to that
renderer-backed route.
`src/syntax_validation_matrix.rs` and
`tests/fixtures/syntax_backend/validation-matrix-gate-v1.json` pin the
route-owned JSON-backend-grade validation matrix for
`target_agnostic_compiler_route` metadata. The matrix covers input validation,
generated-code compilation, semantic round trips, strict/recovery decode,
consumed-token accounting, token/vocabulary/BPE invariants,
generated-artifact reproducibility, maintained documentation, audit coverage,
and JSON-backend parity while keeping arena IR evidence as control-only.
`docs/syntax-state-machine-contract.md` pins the raw syntax-token state,
transition, payload, EOF, consumed-token, and admissible-token semantics that
the report-only replay gate must prove before generated-codec state API
promotion.

## Current Evidence

The current evidence is enough to plan gates, not enough to promote the
backend:

- `tests/toy_grammar_frontend_test.rs` proves a tiny hand-written grammar can
  parse, encode/decode, strictly reject malformed token streams, render, and
  preserve token-layout invariants without touching `TctCompiler`.
- `tests/tree_sitter_metadata_adapter_test.rs` proves Tree-sitter-style
  `node-types.json` metadata (toy assignment fixture and pinned
  tree-sitter-json + tree-sitter-c exports) can lower into `GrammarIR` with
  field requiredness, multiplicity, subtypes/children, referenced anonymous
  leaves, payload classification, pin provenance, and unsupported
  empty-alternative errors.
- `tests/tree_sitter_json_subset_codec_test.rs` proves a fixed
  null/bool/string/number construct subset of that pin can encode/decode
  through the raw syntax state machine and render/reparse with a syntax-level
  claim boundary (no Go layout pin, no JSON Schema compiler path).
- `tests/csharp_syntax_metadata_adapter_test.rs` and
  `scripts/analysis/csharp_syntax_metadata_smoke.py --self-test` prove a
  Roslyn-backed C# syntax fixture can reproduce checked metadata from source
  and lower through the same sidecar with `.NET` / Roslyn provenance. The same
  Rust test also proves the minimal-class C# syntax payload can round trip
  through sidecar syntax tokens without routing `StructuralModel` through
  `TctCompiler`, and that the C# syntax token layout can reserve a
  non-colliding BPE range by moving `PAD_TOKEN` and `VOCAB_SIZE`.
  `./scripts/testing/test-csharp-real-source-corpus.sh` now preserves the
  non-fixture `csharp-dotnet-samples-mini-v1` corpus selection under
  `corpora/csharp/dotnet-samples-mini/`, including the marked source manifest,
  MIT license/provenance, source-list hashes, and `route_source_manifest`
  fingerprint checks as sidecar-only corpus-selection evidence. Isolated
  `dotnet` toolchain and syntax-metadata smokes remain separate retained
  helpers.
  `./scripts/testing/test-csharp-real-source-metadata-gaps.sh` now parses all
  five selected files with Roslyn, records supported syntax route fragments,
  and, after the checked generic-name, qualified-name, constructor-declaration,
  while/postfix loop, interpolated-string, member-access expression, field
  declaration, base-list, conditional-access, array-rank/object-creation,
  `this` expression, pattern-expression, and arrow-expression-clause sidecar
  routes plus completed assignment-expression route support,
  now requires zero remaining unsupported C# syntax-metadata gap classes before
  broader corpus claims.
  `tests/fixtures/syntax_backend/source-corpus-evidence-v1.json` now consumes
  that result as the checked `csharp_no_gap_metadata_report` handoff contract,
  and the source-program handoff report summary preserves it while keeping
  source-program validity, type-correctness, and stable-backend claims blocked.
  `tests/fixtures/syntax_backend/csharp-real-source-generated-codec-feasibility-v1.json`
  consumes that no-gap report, the checked C# corpus manifest, and the
  production route manifest, then records `pattern-matching/Shapes.cs`,
  `delegates-and-events/Logger.cs`, `events/VersionOne.cs`,
  `iterators/IteratorMethods.cs`, and `serialization/Loan.cs` as supported
  `csharp_compilation_unit_source_file_syntax_v0` whole-source
  generated-codec routes. The follow-on is the C# generated-codec corpus
  completion handoff, not a source-program validity or stable-backend claim.
  `tests/fixtures/syntax_backend/csharp-real-source-generated-codec-corpus-v1.json`
  preserves the supported Shapes generated-codec report under
  `target/tct-syntax-real-source-demo/csharp-shapes-generated-codec-demo.json`
  plus the supported Logger report under
  `target/tct-syntax-real-source-demo/csharp-logger-generated-codec-demo.json`;
  the supported VersionOne report under
  `target/tct-syntax-real-source-demo/csharp-version-one-generated-codec-demo.json`;
  the supported IteratorMethods report under
  `target/tct-syntax-real-source-demo/csharp-iterator-methods-generated-codec-demo.json`;
  and the supported Loan report under
  `target/tct-syntax-real-source-demo/csharp-loan-generated-codec-demo.json`,
  without promoting source-program validity, type correctness, arena IR,
  generated hot paths, or stable backend status.
- `tests/go_syntax_metadata_adapter_test.rs` and
  `go run scripts/analysis/go_syntax_metadata_smoke.go --self-test` prove a Go
  `go/parser` / `go/ast` syntax fixture can reproduce checked metadata from
  source and lower through the same sidecar with Go parser provenance.
  The same projector now has checked
  `sort-interface-source-file-declaration-prefix` and
  `sort-interface-source-file` fixtures over the real-source
  `sort/example_interface_test.go` corpus record. The complete source-file
  fixture preserves header comments, package/import shape, `Person` struct
  declaration, `ByAge` slice type declaration, method declaration set, and the
  supported executable `Example` body through `people := []Person{...}`, the
  string/integer person literal payloads, `fmt.Println(people)`,
  `sort.Sort(ByAge(people))`, `sort.Slice(people, func(i, j int) bool {
  return people[i].Age > people[j].Age })`, and the parser-backed `// Output:`
  comment group with its three output lines. The sidecar generated-codec gate
  also compiles that fixture through a generated Rust codec with strict decode,
  trailing-PAD recovery, canonical full-source rendering, integer payload
  checks, closure payload checks, output-comment payload checks, and a Go
  parser/gofmt sidecar, while recording generated-codec corpus promotion for
  this record as sidecar-only gate evidence rather than source-program validity
  evidence.
  The same path also has a checked `sort-search-source-file` fixture over the
  real-source `sort/example_search_test.go` corpus record. That fixture
  preserves the parser-backed header comments, import block, six supported
  example declarations, all `// Output:` comment groups, and the full
  `ExampleSearch` body with the `[]int` composite literal, `sort.Search(len(a),
  func(i int) bool { return a[i] >= x })` predicate, indexed `if/else`
  condition, and `fmt.Printf` branches. Its generated codec records
  strict/recovery round trips, consumed-token accounting, integer payload
  checks, canonical complete-source rendering, and volatile token-count
  evidence under `target/`, while keeping source-program validity,
  functional-correctness, and stable backend promotion claims blocked.
  The same path also has a checked `strings-search-source-file` fixture over
  the real-source `strings/search.go` corpus record. That fixture preserves the
  parser-backed header comments, package clause, `stringFinder` declaration
  with type and field comments, the complete `makeStringFinder` body,
  `longestCommonSuffix`, and the `(*stringFinder).next` method, including
  pointer receivers, named results, branch statements, range/counting loops,
  slice/index expressions, selector assignments, unary integer return, and
  parser-backed comments. Its generated codec records strict/recovery round
  trips, consumed-token accounting, canonical complete-source rendering,
  parser/gofmt sidecar validation, and volatile token-count evidence under
  `target/`, while keeping source-program validity, functional-correctness,
  and stable backend promotion claims blocked.
  The same path also has a checked `path-match-source-file` fixture over the
  real-source `path/match.go` corpus record. That fixture preserves the
  parser-backed header comments, import block, `ErrBadPattern`, `Match`,
  `scanChunk`, `matchChunk`, and `getEsc` declarations, including switch/case
  flow, branch statements, `fallthrough`, UTF-8 decoding calls, error-result
  flow, slice/index expressions, and parser-backed comments. Its generated
  codec records strict/recovery round trips, consumed-token accounting,
  canonical complete-source rendering, parser/gofmt sidecar validation, and
  volatile token-count evidence under `target/`, while keeping source-program
  validity, functional-correctness, type-correctness, arena IR, generated hot
  paths, and stable backend promotion claims blocked.
- `tests/fixtures/go_syntax_corpus/manifest-v1.json` and
  `go run scripts/analysis/go_syntax_corpus_check.go --manifest tests/fixtures/go_syntax_corpus/manifest-v1.json --json-output target/tct-analysis/go-syntax-corpus-check/report.json`
  pin a small BSD-licensed Go standard-library real-source corpus, validate
  source-list and file hashes, parse each file with `go/parser`, and report
  unsupported syntax constructs. The current report is sidecar-only evidence,
  records the checked local float return literal route fixture as coverage for
  the pinned `basic_lit_other` gap, and does not claim stable backend
  promotion. The checked generated-codec corpus gate now preserves all five
  pinned real-source records as sidecar generated-codec round trips with zero
  blocked generated-codec corpus records.
- `tests/syntax_decision_comparison_test.rs` proves the Tree-sitter-style, C#,
  and Go metadata fixtures share target-agnostic categories: rules, fields,
  repetitions, references, payload leaves, payload-boundary edges,
  token-range roles, input provenance, and unchanged compiler boundary flags.
- `tests/syntax_structural_model_conformance_test.rs` is the sidecar-only
  syntax `StructuralModel` boundary conformance gate. It lowers toy, C#, and
  Go syntax sidecars and asserts the shared `structural-model-v0` profile:
  records, finite domains, variants, lists, optionals where present,
  references, payload leaves, token ranges, renderer metadata, validation
  hooks, decision graph nodes and edges, source provenance, parser provenance
  where available, and unchanged compiler-boundary flags.
- `tests/syntax_structural_model_cross_target_equivalence_test.rs` is the
  sidecar-only cross-target equivalence gate. It lowers the matched Go and C#
  local string return, integer return, float return, boolean return, typed
  declaration, short-var/implicit-local declaration, integer short-var/integer
  implicit-local declaration, assignment, escaped-assignment, and shared
  call-expression fixtures, compares canonical local-construct
  `StructuralModel` category sequences, preserves target field cardinality
  profiles, proves the exact Go/C# zero-argument invocation categories through
  `go_local_call_return_expression_syntax_v0` and
  `csharp_method_invocation_return_syntax_v0`, proves the Go argument-call return route
  `go_local_argument_call_return_expression_syntax_v0` carries `len("world")`
  through a `CallExpr.Args -> BasicLit` edge comparable to the C# invocation
  argument literal edge, compares the matched Go/C# if/else control-flow
  categories for condition, then-body, else-body, branch assignment, and
  return payloads, compares the matched Go/C# condition-only loop categories
  for loop statement, condition, body, branch assignment, and return payloads,
  compares the matched Go/C# callable-wrapper categories for declaration,
  name, return type, body block, and return literal over the integer-return
  syntax routes, compares callable parameter-list categories for parameter-list
  edge, parameter binding, parameter name payload, and parameter type payload
  over the Go minimal-function and C# two-parameter method syntax routes,
  compares member-list container categories for repeated member/declaration
  list, data/type member declaration, callable member declaration, member name
  payloads, and member type payloads over the Go package type declaration and
  C# property-plus-method syntax routes, compares data-member field/property
  categories for data-member container, data-member binding, member name
  payload, and member type payload over the Go struct type and C#
  property-plus-method syntax routes, compares method-member callable
  categories for method declaration/container, method name payload, return
  type payload, body block, return statement, and return expression edge over
  the Go method declaration and C# property-plus-method syntax routes, compares
  package/class data declaration categories for data declaration container, data
  declaration binding, data declaration name payload, data declaration type
  payload, initializer field profile, companion callable member, and companion
  return edge over the Go package var declaration and C# property-plus-method
  syntax routes, compares collection/composite initializer categories for
  callable/member container, collection creation node, element type reference
  and payload, repeated initializer element list, and integer element payloads
  over the Go array composite literal and C# collection initializer syntax
  routes, compares index/element-access categories for callable/member
  container, parameter array/element type, index target identifier,
  bracketed/index argument list, and integer index payload over the Go index
  expression and C# element-access syntax routes, compares slice/range
  categories for callable/member container, parameter array/element type,
  range target identifier, slice/range expression, and integer lower/start
  payload over the Go slice expression and C# range element-access syntax
  routes, compares unary-expression categories for callable/member container,
  return body, unary expression edge, unary operator payload, and integer
  operand payload over `go_local_unary_integer_return_expression_syntax_v0`
  and `csharp_local_unary_integer_return_syntax_v0`, compares
  binary-expression categories for callable/member container, return body,
  binary expression edge, binary operator payload, and left/right integer
  operand payloads over `go_local_binary_integer_return_expression_syntax_v0`
  and `csharp_local_binary_integer_return_syntax_v0`, compares
  parenthesized-expression categories for callable/member container, return
  body, parenthesized expression edge, and integer operand payload over
  `go_local_parenthesized_integer_return_expression_syntax_v0` and
  `csharp_local_parenthesized_integer_return_syntax_v0`, compares boolean
  return categories for callable wrapper, return type, return statement, and
  boolean payload leaf over `go_local_boolean_return_identifier_syntax_v0` and
  `csharp_local_boolean_return_literal_syntax_v0`, keeps target-specific
  wrappers, exact parameter counts, Go
  struct-field wrappers, C# folded property binding, Go package var keyword, Go
  package value-list cardinality, Go package initializer literal, C# property
  initializer string payload, Go receiver list/name/type binding, Go method
  return literal, C# method return-identifier spelling, member ordering, Go
  composite literal spelling, C# array-creation initializer wrapper, keyed
  composite values, C# element-access argument-list wrappers, Go open-ended
  high-bound absence, C# bounded range right operand, C# range argument-list
  wrappers, loop setup/header fields, Go `UnaryExpr.Op`/`UnaryExpr.X`
  spelling, C# `PrefixUnaryExpressionSyntax.OperatorToken`/`Operand`
  spelling, Go `BinaryExpr.Op`/`X`/`Y` spelling, C#
  `BinaryExpressionSyntax.OperatorToken`/`Left`/`Right` spelling, Go
  `ParenExpr.X` and C# `ParenthesizedExpressionSyntax.Expression` spelling,
  Go boolean identifier spelling, and C# boolean literal spelling,
  and operator/type tokens out of the shared boundary claim, and
  proves the
  comparison rejects a deliberate semantic payload divergence.
- `tests/fixtures/syntax_structural_model/boundary-manifest-v1.json` is the
  versioned syntax `StructuralModel` boundary manifest. The conformance test
  regenerates it from structured sidecar data, and the audit validates target
  coverage, source or metadata hashes, parser provenance where available,
  model version, proof checks, token-range roles, and the sidecar-only
  compiler-boundary claim.
- `tests/syntax_generated_codec_gate_test.rs` proves the first generated syntax
  codec fixture can be rendered deterministically from the toy grammar
  `GrammarIR -> StructuralModel` sidecar, compiled in the maintained
  generated-crate environment, and checked for strict decode rejection of EOF,
  trailing tokens, malformed structural tokens, and malformed payload tokens
  with consumed-token accounting for success, failure, and partial-prefix
  errors plus a recovery policy that normalizes only trailing `PAD_TOKEN`s
  after a complete program prefix. It also proves the toy syntax layout can
  reserve a non-colliding BPE range inside its current token gap without
  touching `TctCompiler`, and records deterministic reproducibility metadata
  for the rendered toy codec, grammar input, structural model, token layout,
  and sidecar generation options under `target/`. The same gate also carries a
  concrete sidecar BPE merge artifact over deterministic toy syntax token
  sequences and proves BPE encode/decode round trips through the runtime BPE
  implementation without colliding with `PAD_TOKEN` or `VOCAB_SIZE`.
- The same generated-code gate proves the first parser-backed C# syntax codec
  fixture can be rendered from Roslyn-backed `GrammarIR -> StructuralModel`
  metadata, compiled in the maintained generated-crate environment, and checked
  for strict C# minimal-class round trip, EOF rejection, trailing-token
  rejection, malformed payload rejection, consumed-token plus partial-field
  accounting, and the parser-backed recovery policy that normalizes only
  trailing `PAD_TOKEN`s after a complete syntax prefix without touching
  `TctCompiler`. The gate also records deterministic reproducibility metadata
  for the rendered C# codec, parser-backed metadata input, structural model,
  token layout, source metadata provenance, generated-crate identity, and
  sidecar generation options under `target/`.
- The same generated-code gate proves the first parser-backed Go syntax codec
  fixture can be rendered from `go/parser` / `go/ast`-backed
  `GrammarIR -> StructuralModel` metadata, compiled in the maintained
  generated-crate environment, and checked for strict Go minimal-function round
  trip, EOF rejection, trailing-token rejection, malformed-payload rejection,
  invalid identifier rejection, consumed-token plus partial-field accounting,
  and the parser-backed recovery policy that normalizes only trailing
  `PAD_TOKEN`s after a complete syntax prefix without touching `TctCompiler`.
  The gate also records deterministic reproducibility metadata for the rendered
  Go codec, parser-backed metadata input, structural model, token layout,
  source metadata provenance, generated-crate identity, and sidecar generation
  options under `target/`; representative C# and Go parser-backed
  reproducibility artifacts now carry `target_agnostic_compiler_route`
  metadata from `compile_checked_syntax_compiler_route`.
- `src/syntax_backend_boundary.rs` and
  `tests/syntax_backend_boundary_test.rs` define the sidecar-only production
  generator API boundary from the production-boundary contract fixture. The
  boundary names generator inputs, outputs, metadata envelope, error surface,
  and non-goals, and keeps `GrammarIR`, `StructuralModel`, and generated syntax
  codecs outside `TctCompiler`.
- `src/syntax_production_route.rs` and
  `tests/fixtures/syntax_backend/production-route-implementation-gate-v1.json`
  define the feature-limited stable route metadata gate. The gate registers
  the `syntax-production-route-v0` handoff through
  `compile_checked_syntax_production_route`, accepts only checked
  `GrammarIR -> StructuralModel` inputs for toy, C#, and Go, delegates to the
  library-owned renderer API, preserves JSON Schema backend parity, and records
  route construct identity through the
  `SYNTAX_PRODUCTION_ROUTE_CONSTRUCTS` table. The completed route setup class
  is `syntax backend route setup normalization tranche`, the completed C#
  construct classes are `syntax backend C# architecture-parity route tranche`,
  `syntax backend C# local string return literal route tranche`,
  `syntax backend C# integer return literal route tranche`,
  `syntax backend C# float return literal route tranche`,
  `syntax backend C# boolean return literal route tranche`,
  `syntax backend C# null return literal route tranche`,
  `syntax backend C# nullable return type route tranche`,
  `syntax backend C# nullable local declaration route tranche`,
  `syntax backend C# nullable property declaration route tranche`,
  `syntax backend C# nullable parameter declaration route tranche`,
  `syntax backend C# local typed declaration route tranche`,
  `syntax backend C# implicit local declaration route tranche`,
  `syntax backend C# integer implicit local declaration route tranche`,
  `syntax backend C# local assignment route tranche`,
  `syntax backend C# local if/else route tranche`,
  `syntax backend C# condition-only for route tranche`,
  `syntax backend C# single-case switch route tranche`,
  `syntax backend C# local foreach string route tranche`,
  `syntax backend C# two-parameter method route tranche`,
  `syntax backend C# property plus method route tranche`, and
  `syntax backend C# escaped local assignment route tranche`, the source-corpus
  report producer class is `syntax backend
  parser-backed source corpus report producer tranche`, the source-corpus
  report validator class is `syntax backend parser-backed source corpus report
  validator tranche`, the checked route manifest class is `syntax backend
  declarative route manifest tranche`, the route manifest loader class is
  `syntax backend route manifest loader tranche`, the projector metadata setup
  class is `syntax backend projector-driven metadata route setup tranche`, the
  completed Go float literal class is `syntax backend Go float return literal
  route tranche`, the completed Go real-source generated-codec demo is the
  sidecar-only `strings.Compare` route report under
  `target/tct-syntax-real-source-demo/`, the current compiler route entry point
  is `compile_checked_syntax_compiler_route` over representative C#
  `minimal_class` and `local_string_return_literal` plus Go
  `minimal_function`, `local_declaration`, and real-source `strings.Compare`
  generated-codec evidence, parser-backed reproducibility evidence for those
  C# and Go fixture representatives carries deterministic
  `target_agnostic_compiler_route` metadata, the completed compiler route slice class is
  `syntax backend target-agnostic syntax compiler routing slice`, the completed
  generated-code ownership class is `syntax backend target-agnostic
  generated-code gate ownership tranche`, and representative strict/recovery,
  consumed-token, and token/vocabulary/BPE evidence now carries
  `target_agnostic_compiler_route` metadata. The token/vocabulary/BPE ownership
  class is `syntax backend target-agnostic token/vocabulary/BPE route ownership
  tranche`, the route-owned validation matrix class is `syntax backend
  JSON-backend-grade validation matrix route ownership gate`, the validation
  matrix execution preservation class is `syntax backend validation matrix
  execution evidence preservation gate`, and the next missing class is `syntax
  backend final stable-promotion review gate` without
  claiming stable backend promotion.
- `src/syntax_route_manifest.rs` and
  `tests/fixtures/syntax_backend/production-route-manifest-v1.json` define the
  checked declarative route manifest for the current syntax production route.
  The manifest mirrors `SYNTAX_PRODUCTION_ROUTE_CONSTRUCTS`, preserves target
  IDs, grammar IDs, completed route classes, checked metadata paths, generated
  test markers, route entrypoint, route gate, and no-stable claims, and rejects
  stale route/source-corpus drift. Its checked loader is the source-corpus
  record path before more similar construct variants are added, without a build
  script or generated route source.
- `src/syntax_source_corpus_evidence.rs` and
  `tests/fixtures/syntax_backend/source-corpus-evidence-v1.json` define the
  parser-backed source-corpus evidence gate for the current production route.
  The gate derives records from the checked C# minimal-class metadata and the
  checked Go syntax fixtures registered in
  `SYNTAX_PRODUCTION_ROUTE_CONSTRUCTS`, preserving source paths and hashes,
  checked metadata hashes, parser/tool versions, generated-code hashes,
  token-layout metadata, generated-test markers, and explicit BPE-boundary
  state while leaving generated reports under `target/`.
- `src/syntax_artifact_envelope.rs` and
  `tests/fixtures/syntax_backend/generated-artifact-envelope-v1.json` define
  the generated syntax artifact envelope fixture. The fixture names generated
  codec metadata fields, provenance hashes, volatile persistence policy,
  decode policy, token-layout metadata, and non-goals while keeping artifacts
  under `target/`.
- `src/syntax_validation_hooks.rs` and
  `tests/fixtures/syntax_backend/validation-hook-contract-v1.json` define the
  validation-hook contract fixture. The fixture names source reparse or
  equivalent checks, renderer/formatter sidecar status, external-check
  non-promotion policy, validation hook IDs, provenance fields, and non-goals
  without promoting external checks into generated hot paths.
- `src/syntax_input_diagnostics.rs` and
  `tests/fixtures/syntax_backend/input-validation-diagnostics-v1.json` define
  the checked input-validation diagnostics contract fixture. The fixture names
  target IDs, required checked-input fields, diagnostic fields, unsupported
  syntax error shape, rule/path context, source or metadata SHA-256,
  parser-or-grammar identity, diagnostics provenance, sidecar-only status, and
  non-goals without routing through `TctCompiler`.
- `tests/fixtures/syntax_backend/input-validation-negative-fixtures-v1.json`
  defines the first sidecar-only negative checked-input fixture gate. It proves
  malformed toy syntax diagnostics and unsupported toy syntax errors carry
  rule/path context, source SHA-256, grammar identity, sidecar boundary SHA-256,
  and diagnostics contract SHA-256 without requiring a live parser or generated
  hot-path checks.

External parsers, renderers, formatters, type checkers, and functional
validators remain sidecar evidence unless a later stable gate explicitly
promotes one of them.

## Gate Matrix

Every future syntax backend promotion must cover the same maturity categories
as the stable JSON Schema backend, at the syntax backend boundary:

| Category | Syntax Backend Requirement | Current Status |
| --- | --- | --- |
| Input validation | Validate parser metadata or grammar input with rule/path context, source provenance, parser/tool provenance, and explicit unsupported-syntax errors. | Partly covered by toy, Tree-sitter, C#, and Go fixture tests. The checked input-validation diagnostics contract is pinned in `src/syntax_input_diagnostics.rs` and `tests/fixtures/syntax_backend/input-validation-diagnostics-v1.json`, generated-code metadata carries input diagnostics contract identity for toy, C#, and Go, and `tests/fixtures/syntax_backend/input-validation-negative-fixtures-v1.json` proves malformed and unsupported checked-input diagnostics for toy plus direct parser-backed C# and Go metadata records. The validation-hook sidecar evidence fixture in `tests/fixtures/syntax_backend/validation-hook-sidecar-evidence-v1.json` pins toy grammar and C#/Go parser identity, source-or-metadata hashes, sidecar command identity, renderer/formatter status, validation-hook IDs, and external-check non-promotion policy. |
| Generated-code compilation | If the syntax backend emits generated code, compile that generated crate or package under the maintained generated-artifact environment. | Toy grammar plus first parser-backed C# and Go generated-code compilation gates compile through `tests/syntax_generated_codec_gate_test.rs`, while the sidecar renderer API and artifact builders live in `src/syntax_generated_codec.rs`. Representative C# `minimal_class` and `local_string_return_literal` plus Go `minimal_function`, `local_declaration`, and real-source `strings.Compare` generated-codec evidence now crosses `compile_checked_syntax_compiler_route`; remaining parser-backed artifact builders still preserve the lower-level sidecar renderer API as controls. The production generator API boundary is separated in `src/syntax_backend_boundary.rs`, and generated-code metadata now consumes the generated artifact envelope fixture from `src/syntax_artifact_envelope.rs`, the validation-hook contract and sidecar evidence fixtures from `src/syntax_validation_hooks.rs`, the input diagnostics contract fixture from `src/syntax_input_diagnostics.rs`, the input diagnostics negative fixture gate, and the generated artifact replay fixture from `src/syntax_reproducibility_replay.rs` without routing through stable `TctCompiler`. |
| Semantic round trips | Prove source or syntax IR -> tokens -> syntax IR preserves semantic data, then render and reparse where a target renderer exists. | Covered by the toy grammar, the C# minimal-class sidecar token round trip, the generated C# minimal-class, block-scoped namespace, using directive, struct declaration, enum declaration, event declaration, yield statement, attribute syntax, generic name, qualified name, constructor declaration, while/postfix loop, local string return literal, integer return literal, float return literal, boolean return literal, null return literal, nullable return type, nullable local declaration, nullable property declaration, prefix unary integer return, parenthesized integer return, binary integer return, local typed declaration, implicit local declaration, integer implicit local declaration, local assignment, local if/else, condition-only for, single-case switch, local foreach string, two-parameter method, property plus method, zero-argument method invocation return, invocation-with-argument return, collection initializer, element access, range element access, and escaped local assignment codec round trips, and the generated Go minimal-function, local declaration, local assignment, escaped local assignment, short variable declaration, integer short variable declaration, integer return literal, parenthesized integer return expression, unary integer return expression, binary integer return expression, call return expression, argument call return expression, string return literal, float return literal, boolean return identifier, nil return identifier, typed local declaration, zero-value typed local declaration, integer typed local declaration, untyped integer local declaration, integer local assignment, local compound assignment, local increment/decrement, local branch statement, if/else, condition-only for, single-case switch, string range, package type declaration, method declaration, struct type, composite literal, function literal, key/value composite literal, array composite literal, index expression, slice expression, three-index slice expression, star expression, package-level var declaration, ValueSpec.Comment, ImportSpec.Name, TypeSpec.Assign, TypeSpec.Comment, and TypeSpec.Doc codec round trips. The Go `strings.Compare` real-source demo also flows through parser-backed corpus evidence, `GrammarIR -> StructuralModel`, generated codec encode/decode, canonical Go rendering, `gofmt`, and `go/parser`, and records a volatile token-count comparison under `target/tct-syntax-real-source-demo/`; not yet a shared syntax backend. |
| Strict/recovery decode behavior | Define strict rejection, recovery normalization, EOF behavior, trailing-token handling, and malformed payload behavior for syntax token streams. | Strict generated-code rejection and recovery policy evidence are wired for the toy grammar and parser-backed C# and Go fixtures: only trailing `PAD_TOKEN`s after a complete syntax prefix normalize, while EOF, malformed structural tokens, malformed payloads, invalid identifiers, and partial-prefix failures remain hard errors. Representative C# `minimal_class`, C# `local_string_return_literal`, Go `minimal_function`, Go `local_declaration`, and Go real-source `strings.Compare` evidence now carries `target_agnostic_compiler_route.strict_recovery_decode_route_evidence` metadata from `compile_checked_syntax_compiler_route`; remaining parser-backed artifacts stay as sidecar controls. |
| Consumed-token accounting | Report complete versus truncated consumption, surplus tokens, and partial-result behavior for syntax decode paths. | Toy generated-code strict decode reports consumed-token counts for complete streams, truncated streams, surplus tokens, malformed structural tokens, malformed payloads, and partial-prefix failures. Parser-backed C# and Go generated-code strict decode reports consumed-token and partial-field counts for complete streams, EOF, surplus tokens, malformed payloads, and recovery hard errors. Representative C# `minimal_class`, C# `local_string_return_literal`, Go `minimal_function`, Go `local_declaration`, and Go real-source `strings.Compare` evidence now carries `target_agnostic_compiler_route.consumed_token_accounting_route_evidence` plus the accounting cases through the compiler route. The route-boundary accounting contract is pinned in `src/syntax_consumed_token_accounting_routing.rs` and `tests/fixtures/syntax_backend/consumed-token-accounting-routing-gate-v1.json`. |
| Token/vocabulary/BPE invariants | Prove all emitted tokens are below `VOCAB_SIZE`, `PAD_TOKEN` does not collide, structural and payload ranges are declared, and optional BPE ranges are compatible. | Toy generated-code plus C# and Go sidecar token layouts have explicit non-colliding BPE boundary evidence, and the toy generated-code gate has a concrete sidecar BPE merge artifact with runtime encode/decode round-trip evidence. Representative C# `minimal_class`, C# `local_string_return_literal`, Go `minimal_function`, Go `local_declaration`, and Go real-source `strings.Compare` evidence now carries `target_agnostic_compiler_route.token_vocabulary_bpe_route_evidence` plus the invariant-case list through `compile_checked_syntax_compiler_route`; remaining parser-backed artifacts stay as sidecar controls. The route-boundary token/vocabulary/BPE contract is pinned in `src/syntax_token_vocabulary_bpe_routing.rs` and `tests/fixtures/syntax_backend/token-vocabulary-bpe-routing-gate-v1.json`. |
| Generated-artifact reproducibility | When artifacts are produced, record generation inputs and prove deterministic regeneration or a deliberate diff. | The toy, C#, and Go generated-code gates record stable hashes for rendered source, grammar input, structural model, token layout, source metadata provenance, boundary manifest identity, generated-crate identity, generation options, generated artifact envelope identity, envelope fixture hash, target ids, persistence policy, decode policy, token-layout metadata, validation-hook contract identity, validation-hook sidecar evidence identity, generated artifact replay identity, source checks, renderer/formatter sidecar status, external-check non-promotion policy, input diagnostics contract identity, input diagnostics negative fixture identity, rule/path evidence, unsupported syntax error shape, sidecar status, provenance fields, and non-goals, then write volatile reports under `target/tct-syntax-generated-codec-reproducibility/`. The representative C# `minimal_class`, C# `local_string_return_literal`, Go `minimal_function`, and Go `local_declaration` reproducibility artifacts now come from route-owned helpers that call `compile_checked_syntax_compiler_route` and prove deterministic `target_agnostic_compiler_route` metadata; remaining parser-backed artifacts stay as sidecar controls. The replay fixture is `tests/fixtures/syntax_backend/generated-artifact-replay-v1.json`, and the route-boundary reproducibility contract is pinned in `src/syntax_reproducibility_routing.rs` and `tests/fixtures/syntax_backend/reproducibility-routing-gate-v1.json`. |
| Maintained documentation | Keep `docs/grammar-ir-contract.md`, `docs/structural-model-contract.md`, this strategy, `DEVELOPMENT.md`, and `scripts/README.md` aligned with actual commands and gates. | This document, `reports/m6-cross-target-contract-closure-2026-07-21.md`, `reports/README.md`, the research roadmap, and the audit inventory now name the completed Go real-source source-program validity corpus gate, the completed `syntax backend Go real-source generated-codec corpus round-trip gate`, the completed `syntax backend Go real-source generated-codec corpus completion handoff gate`, the completed stable metadata promotion input refinement, the completed `syntax backend C# collection initializer route tranche`, the completed `syntax backend C# element access route tranche`, the completed `syntax backend C# range element access route tranche`, the completed `syntax backend C# prefix unary integer return route tranche`, the completed `syntax backend C# parenthesized integer return route tranche`, the completed `syntax backend C# binary integer return route tranche`, the completed `syntax backend C# boolean return literal route tranche`, the completed `syntax backend C# null return literal route tranche`, the completed `syntax backend C# nullable return type route tranche`, the completed `syntax backend C# nullable local declaration route tranche`, the completed `syntax backend C# nullable property declaration route tranche`, the completed `syntax backend C# nullable parameter declaration route tranche`, the completed C# nullable context `StructuralModel` equivalence gate, the completed `syntax backend Go/C# boolean return StructuralModel parity tranche`, the completed `syntax backend Go/C# nil/null return StructuralModel parity tranche`, and the maintained Go real-source generated-codec corpus runner. |
| Audit coverage | Inventory the syntax backend strategy, fixtures, helpers, tests, docs, reports, and missing gate classes in the research repository's test-quality audit. | Implemented as a report-only inventory over the production-route, validation-matrix, source-corpus, source-program handoff, external-validation, the retired type-checker feasibility/execution/preservation evidence family (historical fixture metadata only), Go real-source corpus eligibility, Go real-source generated-codec corpus, Go real-source generated-codec completion handoff, stable metadata promotion, source-program handoff input-refinement, external-validation input-refinement, retired type-checker input-refinement historical pins, source-program validity corpus input-refinement, generated-codec corpus input-refinement, generated-codec completion handoff input-refinement, stable metadata promotion source-input refinement, the maintained Go real-source generated-codec corpus runner, the route-pinned generic local-construct equivalence gate, the cross-target exact zero-argument invocation, argument-call, if/else, condition-only loop, single-case switch, string-iteration, callable-wrapper, callable parameter-list, member-list container, data-member field/property, method-member callable, package/class data declaration, collection/composite initializer, index/element-access, slice/range, unary-expression, binary-expression, parenthesized-expression, float return literal, boolean return, and nil/null return structural-model equivalence gates, the C# nullable context StructuralModel equivalence gate, and the C# local if/else, condition-only for, single-case switch, local foreach string, two-parameter method, property plus method, zero-argument method invocation return, invocation-with-argument return, collection initializer, element access, range element access, prefix-unary integer return, parenthesized integer return, binary integer return, float return literal, boolean return literal, null return literal, nullable return type, nullable local declaration, nullable property declaration, nullable parameter declaration, block-scoped namespace, using directive, struct declaration, enum declaration, event declaration, yield statement, attribute syntax, generic name, qualified name, constructor declaration, while/postfix loop, interpolated string, member-access expression, field declaration, base-list, conditional-access, array-rank/object-creation, this-expression, pattern-expression, arrow-expression-clause, and Go argument-call return syntax generated-codec parity slices. The completed corpus and handoff classes include the Go real-source source-program validity corpus gate, Go real-source generated-codec corpus round-trip gate, Go real-source generated-codec corpus completion handoff gate, stable metadata promotion implementation gate, source-program claim handoff gate, external-validation controls, retired type-checker control gates as historical metadata, C# representative real-source corpus selection gate, parser-backed C# real-source syntax-metadata gap evidence gate, all C# sidecar routes through arrow-expression-clause, the checked `csharp_no_gap_metadata_report` source-corpus/source-program handoff contract, the C# real-source generated-codec feasibility gate, the C# real-source generated-codec corpus gate, the C# real-source generated-codec corpus completion handoff gate, the source-program external-validation controls C# completion handoff input, the external-validation execution-preservation C# completion handoff input, and the C# completion handoff inputs retained on the external-validation chain for `pattern-matching/Shapes.cs`, `delegates-and-events/Logger.cs`, `events/VersionOne.cs`, `iterators/IteratorMethods.cs`, and `serialization/Loan.cs`, while keeping source-program validity, type-correctness, paper, model-quality, arena IR, generated hot-path, and stable backend claims blocked until explicitly proven. |

The audit inventory also includes the completed Go string range, package type
declaration, method declaration, struct type, composite literal, function
literal, key/value composite literal, array composite literal, index
expression classes: `syntax backend Go string range route tranche`, `syntax
backend Go package type declaration route tranche`, `syntax backend Go method
declaration route tranche`, `syntax backend Go struct type route tranche`,
`syntax backend Go composite literal route tranche`, and `syntax backend Go
function literal route tranche`, and `syntax backend Go key/value composite
literal route tranche`, and `syntax backend Go array composite literal route
tranche`, `syntax backend Go index expression route tranche`, `syntax backend
Go slice expression route tranche`, `syntax backend Go three-index slice
expression route tranche`, `syntax backend Go star expression route
tranche`, `syntax backend Go package var declaration route tranche`,
`syntax backend Go local compound assignment route tranche`, and
`syntax backend Go local increment/decrement route tranche`, and
`syntax backend Go local branch statement route tranche`. It also records the
Go real-source generated-codec demo marker
`generated_go_minimal_function_real_source_strings_compare_demo` as
sidecar-only evidence, not stable backend promotion.

## Promotion Rule

Do not promote a syntax-level backend until all gate categories above have an
owned validation command or a precise blocker. The first stable promotion
candidate must be target-agnostic enough to serve at least the C# and Go
parser-backed syntax fixtures through the same `GrammarIR -> StructuralModel`
boundary. It must not rely on GoIR or C# arena schema shape as the backend
contract.

Stable promotion must also preserve the JSON Schema backend contract: schema
compilation, generated code compilation, token encoding, token decoding, data
validation, vocabulary accounting, BPE integration, and maintained docs must
continue to agree.

## Audit And Generated-Code Inventory

The current route-hardening slices include the completed Go minimal-function
production route entry after the completed projector-driven metadata route
setup, completed manifest-backed route setup loader, completed checked
declarative route manifest, and completed parser-backed source-corpus report
producer and validator plus the
completed Go local declaration, local assignment, escaped local assignment,
short variable declaration, integer short variable
declaration, integer return literal, parenthesized integer return expression,
unary integer return expression, binary integer return expression, call return
expression, string return literal, boolean return identifier, nil return
identifier, typed local declaration, zero-value typed local declaration,
  integer typed local declaration, untyped integer local declaration, integer
  local assignment, local compound assignment, local increment/decrement,
  local branch statement, if/else, condition-only for, single-case switch, string range, package type
  declaration, method declaration, star expression, package-level var
  declaration, route setup
normalization, C# architecture-parity, C# local
string return literal, C# integer return literal, C# local assignment, C#
local if/else, condition-only for, single-case switch, local foreach string, callable-wrapper parity, exact zero-argument invocation parity, two-parameter method, property plus method, zero-argument method invocation return, invocation-with-argument return, escaped local assignment, parser-backed source-corpus
evidence tranches, not stable backend promotion.

the research repository's test-quality audit includes a syntax backend gate inventory
that checks the current strategy without running external parsers. The
inventory reports:

- the presence of this strategy document and required gate-category markers;
- the fixture, adapter, helper, and focused-test files for toy grammar,
  Tree-sitter metadata, C# syntax metadata, and Go syntax metadata;
- command markers for the C# and Go parser-backed metadata smokes in
  `DEVELOPMENT.md` and `scripts/README.md`;
- the cross-fixture comparison test as the current target-agnostic sidecar
  signal;
- the shared generated syntax codec gate command,
  `cargo test --features experimental --test syntax_generated_codec_gate_test`, plus toy, C#, and Go
  generated-code targets with their source kind, generated crate name, sidecar
  boundary, and missing target markers;
- the syntax `StructuralModel` conformance gate,
  `tests/syntax_structural_model_conformance_test.rs`, with required profile
  markers for the sidecar boundary;
- the versioned syntax `StructuralModel` boundary manifest,
  `tests/fixtures/syntax_structural_model/boundary-manifest-v1.json`, with toy,
  C#, and Go targets plus sidecar-boundary validation;
- the syntax `StructuralModel` stable-gate plan in
  `docs/syntax-structural-model-stable-gate-plan.md`, including its mapping
  from the manifest to JSON-backend-grade gate categories;
- the manifest-backed syntax generated-code handoff in
  `tests/syntax_generated_codec_gate_test.rs`, where toy, C#, and Go
  generated-code reproducibility metadata records manifest path, artifact kind,
  version, hash, target ids, target source kind, parser identity when present,
  and sidecar-only boundary claim;
- the maintained syntax backend sidecar gate helper,
  `scripts/testing/test-syntax-backend-gates.sh`, which runs the generated-code
  gate, `StructuralModel` conformance gate, production generator API boundary
  gate, and syntax backend audit inventory;
- the M6 cross-target contract closure report,
  `reports/m6-cross-target-contract-closure-2026-07-21.md`, which maps Go and
  C# `minimal_class` shared state/transition/admissible/decode/layout/validation
  contracts to retained executable gates (layer-3 stable promotion remains a
  separate track);
- the production-boundary contract fixture,
  `tests/fixtures/syntax_backend/production-boundary-contract-v1.json`, which
  pins the future checked-syntax-input to generated-codec handoff while keeping
  `TctCompiler` routing blocked;
- the production generator API boundary module and test,
  `src/syntax_backend_boundary.rs` and
  `tests/syntax_backend_boundary_test.rs`, which name the future generator API
  inputs, outputs, metadata envelope, error surface, and non-goals without
  promoting production compiler routing;
- the production `TctCompiler` routing gate module and fixture,
  `src/syntax_tct_compiler_routing.rs` and
  `tests/fixtures/syntax_backend/tct-compiler-routing-gate-v1.json`, which name
  the future checked `GrammarIR -> StructuralModel` routing handoff, the
  renderer API dependency, the still-disabled route state, JSON-backend parity
  requirements, and routing blockers without registering a syntax route;
- the JSON-backend parity routing gate module and fixture,
  `src/syntax_json_backend_parity.rs` and
  `tests/fixtures/syntax_backend/json-backend-parity-routing-gate-v1.json`,
  which name the stable JSON Schema backend surfaces, stable gate commands,
  disabled route state, and remaining routing blockers before any syntax route
  may call the renderer API;
- the target-agnostic routing boundary gate module and fixture,
  `src/syntax_target_agnostic_routing_boundary.rs` and
  `tests/fixtures/syntax_backend/target-agnostic-routing-boundary-gate-v1.json`,
  which pin checked `GrammarIR -> StructuralModel` as the only acceptable
  future route input while rejecting GoIR, C# arena IR, and target-specific
  schema shapes;
- the strict/recovery decode routing gate module and fixture,
  `src/syntax_strict_recovery_decode_routing.rs` and
  `tests/fixtures/syntax_backend/strict-recovery-decode-routing-gate-v1.json`,
  which pin strict decode authority, trailing `PAD_TOKEN` recovery
  normalization, invalid parser-backed payload rejection, and partial-prefix
  failures at the future route boundary, and now require representative
  parser-backed evidence to carry `target_agnostic_compiler_route` metadata;
- the consumed-token accounting routing gate module and fixture,
  `src/syntax_consumed_token_accounting_routing.rs` and
  `tests/fixtures/syntax_backend/consumed-token-accounting-routing-gate-v1.json`,
  which pin complete-stream, trailing-`PAD_TOKEN` recovery, hard-error, and
  partial-prefix consumed-token and partial-result accounting at the future
  route boundary, and now require representative parser-backed evidence to
  carry `target_agnostic_compiler_route` metadata;
- the token/vocabulary/BPE routing gate module and fixture,
  `src/syntax_token_vocabulary_bpe_routing.rs` and
  `tests/fixtures/syntax_backend/token-vocabulary-bpe-routing-gate-v1.json`,
  which pin `PAD_TOKEN`, `VOCAB_SIZE`, raw base vocabulary, structural ranges,
  payload ranges, optional BPE ranges, non-collision checks, BPE artifact
  fields, and emitted-token bounds at the future route boundary;
- the reproducibility routing gate module and fixture,
  `src/syntax_reproducibility_routing.rs` and
  `tests/fixtures/syntax_backend/reproducibility-routing-gate-v1.json`, which
  pin deterministic hash fields, volatile report fields, source fixture hashes,
  replay ownership, generated-code evidence markers, and sidecar-only route
  state at the future route boundary;
- the audit coverage routing gate module and fixture,
  `src/syntax_audit_coverage_routing.rs` and
  `tests/fixtures/syntax_backend/audit-coverage-routing-gate-v1.json`, which
  pin the maintained audit inventory, route-boundary modules and fixtures,
  helper commands, readiness report, no-route claims, fail-on-gaps requirement,
  and sidecar-only route state at the future route boundary;
- the stable-promotion review module and fixture,
  `src/syntax_stable_promotion_review.rs` and
  `tests/fixtures/syntax_backend/stable-promotion-review-gate-v1.json`, which
  pin the completed route-boundary classes, JSON-backend-grade gate
  categories, no-route claims, and the decision that the next tranche may add a
  first production route implementation slice without promoting a stable
  backend;
- the production route implementation module and fixture,
  `src/syntax_production_route.rs` and
  `tests/fixtures/syntax_backend/production-route-implementation-gate-v1.json`,
  which pin `tct_syntax_backend_production_route_implementation_gate`, the
  `syntax-production-route-v0` route gate,
  `compile_checked_syntax_production_route`, checked
  `GrammarIR -> StructuralModel` input, the toy, C#, and Go renderer functions,
  JSON-backend parity requirements, the
  `SYNTAX_PRODUCTION_ROUTE_CONSTRUCTS` route setup source of truth, and the
  target-agnostic compiler route plus parser-backed reproducibility evidence
  for representative C#/Go fixtures, with validation-matrix route metadata and
  feature-limited stable metadata claims;
- the validation matrix module and fixture,
  `src/syntax_validation_matrix.rs` and
  `tests/fixtures/syntax_backend/validation-matrix-gate-v1.json`, which pin
  `tct_syntax_backend_validation_matrix_gate`, the
  `target_agnostic_compiler_route` metadata field, the JSON-backend-grade
  category set, required route-owned evidence, required sidecar controls,
  required validation commands, stable metadata claims, and the next
  execution-depth blocker
  `syntax backend validation matrix execution evidence preservation gate`;
- the validation matrix execution runner,
  `scripts/testing/run-syntax-validation-matrix.py`, which writes
  `tct_syntax_backend_validation_matrix_execution_report` artifacts to
  `target/tct-syntax-validation-matrix/execution-report-v1.json` with
  per-command and per-category pass/fail records via
  `python3 scripts/testing/run-syntax-validation-matrix.py --output target/tct-syntax-validation-matrix/execution-report-v1.json`,
  and validates them with
  `python3 scripts/testing/run-syntax-validation-matrix.py --validate-report target/tct-syntax-validation-matrix/execution-report-v1.json`;
  its helper class is `syntax backend validation matrix execution report
  helper gate`;
- the validation matrix execution preservation gate,
  `src/syntax_validation_matrix_preservation.rs` and
  `tests/fixtures/syntax_backend/validation-matrix-execution-preservation-v1.json`,
  which pin
  `tct_syntax_backend_validation_matrix_execution_preservation_gate`,
  `release_depth_execution_evidence_preservation_policy`, the report refresh
  and validation commands, the stable execution report fields, the volatile
  fields `command_results.duration_ms` and `command_results.log_path`, no
  checked-in report hash, no arena IR proof, JSON Schema backend preservation,
  the completed class `syntax backend validation matrix execution evidence
  preservation gate`, and the next missing class `syntax backend final
  stable-promotion review gate`;
- the final stable-promotion decision gate,
  `src/syntax_final_promotion_decision.rs` and
  `tests/fixtures/syntax_backend/final-stable-promotion-decision-v1.json`,
  which pins `tct_syntax_backend_final_stable_promotion_decision_gate`,
  `stable_promotion_ready_decision`,
  `ready_for_stable_metadata_promotion_implementation`, the preserved
  release-depth execution report policy, the route-owned validation-matrix
  evidence, JSON Schema backend preservation, no arena IR proof, no external
  sidecar promotion, the completed class `syntax backend final
  stable-promotion review gate`, the completed stable metadata class `syntax
  backend stable metadata promotion implementation gate`, and the completed
  stable metadata decision input before the source-program handoff sequence;
- the checked route manifest module and fixture,
  `src/syntax_route_manifest.rs` and
  `tests/fixtures/syntax_backend/production-route-manifest-v1.json`, which pin
  `tct_syntax_backend_declarative_route_manifest`, the route setup source of
  truth, construct identity fields, the registered C#/Go construct records,
  the checked route manifest loader, no-stable claims, the completed loader
  class, and the next projector-driven metadata route setup class;
- the source-corpus evidence module and fixture,
  `src/syntax_source_corpus_evidence.rs` and
  `tests/fixtures/syntax_backend/source-corpus-evidence-v1.json`, which pin the
  currently supported parser-backed source corpus, preserved record fields,
  generated report target path, producer and validator commands, missing
  evidence, no-stable-claim policy, and next missing class before downstream
  source-program handoff;
- the source-program claim handoff profile,
  `src/syntax_source_program_handoff.rs` and
  `tests/fixtures/syntax_backend/source-program-claim-handoff-v1.json`, which
  pins `tct_syntax_backend_source_program_claim_handoff`,
  `syntax-source-program-claim-handoff-v1`, the syntax-level boundary
  `parser-backed metadata -> GrammarIR -> StructuralModel -> token layout ->
  generated codec`, the allowed generated-codec validity claim for checked
  parser-backed source-corpus records and the checked Go real-source
  generated-codec corpus records, the consumed stable metadata promotion input
  summary, including completion-handoff source-program stable report fields,
  type-checker preservation report contract stable fields, source-program
  blocked claims, type-checker execution and feasibility report identities,
  pass-envelope and sidecar-only policy, corpus manifest/report identity,
  generated-codec report commands, source-program and generated-codec
  no-stable claims, supported counts, supported manifest paths,
  generated-codec routes, generated-codec markers, and parser-backed
  generated-codec round-trip records, plus the Go generated-codec completion
  handoff inputs, blocked whole-source-language, whole-file generated-codec,
  cross-repo paper, model-quality,
  functional-correctness, completion, type-correctness, arena IR,
  external-sidecar stable-promotion, generated-hot-path, and generated JSON
  Schema output claims, required report summary and record fields, generated
  report target path under `target/`, no checked-in report policy, producer
  and validator commands in `src/bin/tct-syntax-source-program-handoff.rs`, no
  arena IR proof, JSON Schema backend preservation, the completed class
  `syntax backend source-program claim handoff gate`, the completed report
  producer class `syntax backend source-program handoff report producer gate`,
  and the next missing control-evidence class `syntax backend source-program
  external validation control evidence gate`;
  the producer command is
  `cargo run --features experimental --bin tct-syntax-source-program-handoff -- --source-corpus-report target/tct-syntax-source-corpus-evidence/parser-backed-source-corpus-evidence-v1.json --output target/tct-syntax-source-program-handoff/source-program-claim-handoff-v1.json`;
  the validator command is
  `cargo run --features experimental --bin tct-syntax-source-program-handoff -- --validate-report target/tct-syntax-source-program-handoff/source-program-claim-handoff-v1.json`;
- the source-program external validation controls gate,
  `src/syntax_source_program_external_validation.rs` and
  `tests/fixtures/syntax_backend/source-program-external-validation-controls-v1.json`,
  which consumes the generated source-program handoff report plus its stable
  metadata promotion and Go generated-codec completion handoff inputs,
  classifies parser metadata reproduction and renderer/formatter status as
  sidecar-only controls, records type-checker and functional-validator
  evidence as missing downstream controls, blocks source-program,
  whole-language, whole-file generated-codec, type-correctness, paper,
  model-quality, functional-correctness, completion, arena IR,
  external-sidecar stable-promotion, generated-hot-path, and generated JSON
  Schema output claims, preserves the no-arena-IR and no-external-sidecar
  promotion boundary, and sets the next missing class to `syntax backend
  source-program external validation execution report gate`;
- the source-program external validation execution report helper,
  `src/bin/tct-syntax-source-program-external-validation.rs` and
  `src/syntax_source_program_external_validation.rs`, which writes and
  validates
  `tct_syntax_backend_source_program_external_validation_execution_report`
  artifacts under
  `target/tct-syntax-source-program-external-validation/execution-report-v1.json`,
  records observed tool versions, preserves source and metadata hashes,
  parser/tool identities, generated artifact hashes, token-layout hashes,
  route metadata, validation-hook status, sidecar-control status, source
  handoff promotion-source inputs, blocked claims, audit coverage, and
  validation commands, keeps parser metadata and renderer/formatter controls
  sidecar-only, keeps type-checker and functional-validator controls
  unavailable, completes `syntax backend source-program external validation
  execution report gate`, and sets the next missing class to `syntax backend
  source-program external validation execution preservation gate`;
  the producer command is
  `cargo run --features experimental --bin tct-syntax-source-program-external-validation -- --source-program-handoff-report target/tct-syntax-source-program-handoff/source-program-claim-handoff-v1.json --output target/tct-syntax-source-program-external-validation/execution-report-v1.json`;
  the validator command is
  `cargo run --features experimental --bin tct-syntax-source-program-external-validation -- --validate-report target/tct-syntax-source-program-external-validation/execution-report-v1.json`;
- the source-program external validation execution preservation gate,
  `src/syntax_source_program_external_validation_preservation.rs` and
  `tests/fixtures/syntax_backend/source-program-external-validation-execution-preservation-v1.json`,
  which keeps the volatile execution report under `target/`, preserves the
  report artifact kind, producer and validator commands, target path, source
  handoff report hash, source handoff promotion-source fields, the C#
  generated-codec completion handoff report fields and five-record identity
  summary, controls fixture hash, tool envelope, control summary, record
  hashes, blocked claims, audit coverage, and refresh policy, keeps parser
  metadata and
  renderer/formatter controls sidecar-only, keeps type-checker and
  functional-validator controls unavailable, keeps whole-file generated-codec,
  arena IR, external-sidecar stable-promotion, generated-hot-path, and
  generated JSON Schema output claims blocked, completes `syntax backend
  source-program external validation execution preservation gate`, and records
  the historical next-class pin as `syntax backend source-program type-checker
  control feasibility gate` (that typecheck family is retired; the live retained
  chain continues at the Go real-source source-program validity corpus gate);
- the source-program type-checker feasibility/execution evidence family was
  retired (2026-06) rather than regenerated; no live
  tct-syntax-source-program-typecheck-* binaries remain. Sidecar type-checker
  evidence stays a documented concept only. Retained handoff, Go real-source
  validity corpus, generated-codec corpus/completion, and stable-metadata
  promotion gates pin the retired family's identity pins and stable field
  names as historical fixture metadata only (see the
  source_typecheck_preservation block on the Go validity corpus fixture).
  Retired inventory markers kept for audit compatibility (not live modules or
  commands): src/syntax_source_program_typecheck_feasibility.rs,
  src/bin/tct-syntax-source-program-typecheck-feasibility.rs,
  tct_syntax_backend_source_program_typecheck_feasibility_report,
  syntax backend source-program type-checker control feasibility gate,
  syntax backend source-program type-checker execution report gate,
  src/syntax_source_program_typecheck_execution.rs,
  src/bin/tct-syntax-source-program-typecheck-execution.rs,
  tct_syntax_backend_source_program_typecheck_execution_report,
  src/syntax_source_program_typecheck_execution_preservation.rs,
  tests/fixtures/syntax_backend/source-program-typecheck-execution-preservation-v1.json,
  tct_syntax_backend_source_program_typecheck_execution_preservation_gate,
  source_program_typecheck_execution_report_preservation_policy, and
  syntax backend source-program type-checker execution preservation gate.
  Historical next-class pins after external-validation preservation still name
  the type-checker control feasibility gate; the live retained chain continues
  at the Go real-source source-program validity corpus gate below;
- the Go real-source source-program validity corpus gate,
  `src/syntax_go_real_source_program_validity_corpus.rs` and
  `tests/fixtures/syntax_backend/go-real-source-program-validity-corpus-v1.json`,
  which pins the BSD-licensed Go stdlib corpus manifest
  `tests/fixtures/go_syntax_corpus/manifest-v1.json`, records the volatile
  `go_syntax_corpus_check` report contract under
  `target/tct-analysis/go-syntax-corpus-check/report.json`, classifies the
  five current corpus files as parser-backed source-program sidecar-eligible,
  consumes the retired type-checker execution preservation family's source
  handoff promotion-source stable fields and expanded blocked-claim contract as
  historical fixture metadata only,
  records zero unsupported corpus records for the current helper report, keeps
  source-program validity, whole-language, paper, model-quality, type
  correctness, completion, arena IR, external-sidecar stable-promotion,
  generated hot-path, and JSON Schema output claims blocked, completes
  `syntax backend Go real-source source-program validity corpus gate`, and sets
  the next missing class to `syntax backend Go real-source generated-codec
  corpus round-trip gate`;
- the Go real-source generated-codec corpus gate,
  `src/syntax_go_real_source_generated_codec_corpus.rs` and
  `tests/fixtures/syntax_backend/go-real-source-generated-codec-corpus-v1.json`,
  which consumes the source-program corpus gate's preserved type-checker
  handoff fields (historical metadata from the retired typecheck family),
  expanded blocked claims, and sidecar-only no-stable boundary while preserving
  generated-codec round-trip evidence for all five checked Go real-source corpus
  records (stdlib-relative identities: `strings/compare.go`,
  `sort/example_interface_test.go`, `sort/example_search_test.go`, `strings/search.go`,
  and `path/match.go`), the volatile generated-codec
  reports under `target/tct-syntax-real-source-demo/`, parser-backed `gofmt`
  plus `go/parser` validation, exact token-count evidence for the canonical
  rendered sources, zero blocked generated-codec corpus records, the
  sidecar-only
  `whole_file_generated_codec_claim_for_checked_go_real_source_corpus` claim,
  and the completed
  `syntax backend Go real-source generated-codec corpus round-trip gate`;
- the Go real-source generated-codec corpus completion handoff gate,
  `src/syntax_go_real_source_generated_codec_completion_handoff.rs` and
  `tests/fixtures/syntax_backend/go-real-source-generated-codec-completion-handoff-v1.json`,
  which consumes the completed five-record generated-codec corpus gate, its
  preserved source-program corpus handoff fields, retired type-checker
  preservation report contract stable fields, retired type-checker execution and
  feasibility report identities, pass-envelope and sidecar-only policy,
  corpus manifest/report identity, generated-codec report commands,
  generated-codec no-stable claims, supported record counts and identities,
  parser-backed generated-codec round-trip records, and the final
  stable-promotion decision gate, records
  `tct_syntax_backend_go_real_source_generated_codec_completion_handoff_gate`,
  keeps parser/formatter and type-checker sidecars unpromoted, blocks
  source-program validity, whole-file generated-codec, paper, model-quality,
  functional-correctness, completion or infill, type-correctness, arena IR,
  generated hot-path, and generated JSON Schema output claims, completes
  `syntax backend Go real-source generated-codec corpus completion handoff
  gate`, and names `syntax backend stable metadata promotion implementation
  gate` as the next source-input preservation prerequisite;
- the stable metadata promotion implementation gate,
  `src/syntax_stable_metadata_promotion.rs` and
  `tests/fixtures/syntax_backend/stable-metadata-promotion-implementation-v1.json`,
  which consumes the Go real-source generated-codec corpus completion handoff,
  the final stable-promotion decision, and the feature-limited production route
  metadata, records
  `tct_syntax_backend_stable_metadata_promotion_implementation_gate`, promotes
  only `stable_route_metadata_promoted` for
  `feature_limited_parser_backed_syntax_backend`, preserves the handoff's
  source-program stable report fields, retired type-checker preservation report
  contract stable fields, source-program blocked claims, retired type-checker
  execution and feasibility report identities, pass-envelope and sidecar-only
  policy,
  corpus manifest/report identity, generated-codec report
  commands, source-program and generated-codec no-stable claims, supported
  record counts, record identities, and parser-backed generated-codec
  round-trip records, preserves the JSON Schema backend contract, keeps
  source-program, whole-file generated-codec, paper, model-quality,
  functional-correctness, completion or infill,
  type-correctness, arena IR, generated hot-path, and generated JSON Schema
  output claims blocked, completes `syntax backend stable metadata promotion
  implementation gate`, and names `Build Go Source-Record Raw-Token Generator
  Core` as the next route-owned generator-core prerequisite;
- the generated artifact envelope module and fixture,
  `src/syntax_artifact_envelope.rs` and
  `tests/fixtures/syntax_backend/generated-artifact-envelope-v1.json`, which
  name generated codec artifact metadata fields, provenance hashes, volatile
  persistence policy, decode policy, token-layout metadata, and non-goals;
- the generated artifact envelope handoff in
  `tests/syntax_generated_codec_gate_test.rs`, where toy, C#, and Go
  generated-code reproducibility metadata records envelope identity, fixture
  hash, target ids, persistence policy, decode policy, token-layout metadata,
  and the sidecar-only compiler boundary before writing volatile reports under
  `target/`;
- the validation-hook contract module and fixture,
  `src/syntax_validation_hooks.rs` and
  `tests/fixtures/syntax_backend/validation-hook-contract-v1.json`, which name
  source reparse or equivalent checks, renderer/formatter sidecar status,
  external-check non-promotion policy, validation hook IDs, provenance fields,
  and non-goals;
- the validation-hook contract generated-code handoff in
  `tests/syntax_generated_codec_gate_test.rs`, where toy, C#, and Go metadata
  records contract identity, fixture hash, target IDs, source checks,
  renderer/formatter sidecar status, external-check non-promotion policy,
  provenance fields, and non-goals;
- the validation-hook sidecar evidence fixture,
  `tests/fixtures/syntax_backend/validation-hook-sidecar-evidence-v1.json`,
  where toy, C#, and Go records pin grammar or parser identity,
  source-or-metadata SHA-256, sidecar command identity, renderer metadata ID,
  renderer/formatter sidecar status, validation-hook IDs, policy hashes,
  contract hash, sidecar boundary SHA-256, and sidecar-only status;
- the validation-hook sidecar evidence generated-code handoff in
  `tests/syntax_generated_codec_gate_test.rs`, where toy, C#, and Go metadata
  records fixture identity and target record identity alongside the validation
  hook contract hash;
- the input-validation diagnostics contract module and fixture,
  `src/syntax_input_diagnostics.rs` and
  `tests/fixtures/syntax_backend/input-validation-diagnostics-v1.json`, which
  name checked syntax input fields, diagnostic fields, unsupported syntax
  error shape, rule/path context, source-or-metadata SHA-256,
  parser-or-grammar identity, diagnostics provenance, sidecar-only status, and
  non-goals;
- the input diagnostics contract generated-code handoff in
  `tests/syntax_generated_codec_gate_test.rs`, where toy, C#, and Go metadata
  records contract identity, fixture hash, target IDs, required input fields,
  diagnostic fields, unsupported syntax error shape, diagnostics provenance,
  sidecar status, and non-goals;
- the input-validation negative fixture gate in
  `tests/fixtures/syntax_backend/input-validation-negative-fixtures-v1.json`,
  where synthetic toy, C#, and Go malformed and unsupported checked-input
  records prove rule/path diagnostics, unsupported syntax errors,
  source-or-metadata SHA-256, grammar or parser identity, sidecar boundary
  SHA-256, diagnostics contract SHA-256, and sidecar-only status;
- the input diagnostics negative fixture generated-code handoff in
  `tests/syntax_generated_codec_gate_test.rs`, where toy, C#, and Go metadata
  records fixture identity, fixture hash, fixture IDs, fixture kinds, severity
  coverage, rule/path evidence, unsupported syntax errors, provenance hashes,
  sidecar status, and non-goals;
- current route/source-corpus missing stable-promotion class: syntax backend
  source-program external validation execution preservation gate. The checked
  source-program claim handoff profile and source-program external validation
  controls now record the consumed stable metadata promotion implementation
  gate and Go real-source generated-codec completion handoff as explicit
  promotion-source inputs, record the precise generated-codec-only claim
  boundary, the handoff
  producer/validator writes `tct_syntax_backend_source_program_claim_handoff_report`
  artifacts under `target/`, the external validation controls gate classifies
  parser/formatter evidence as sidecar-only controls, the external validation
  execution helper writes
  `tct_syntax_backend_source_program_external_validation_execution_report`
  artifacts under `target/`, the execution preservation gate keeps that
  refresh contract checked without checking volatile reports into git, the
  type-checker feasibility helper maps current Go and C# handoff records to
  direct versus wrapper-based type-check command shapes, the type-checker
  execution helper records 19 passing C# wrapper builds and 39 passing Go direct
  checks, the type-checker execution preservation gate keeps that all-current
  pass envelope, source handoff promotion inputs, and expanded blocked-claim
  set checked without checking volatile reports into git, the source-program
  external validation controls gate now consumes the source-program handoff
  source inputs directly without promoting parser, formatter, type-checker, or
  functional-validator controls into syntax-level proof, the Go
  real-source corpus gate keeps parser-backed corpus eligibility sidecar-only
  while consuming the type-checker preservation source handoff inputs,
  the Go real-source generated-codec corpus gate preserves all five checked Go
  corpus records as sidecar generated-codec round trips without whole-file
  source validity, functional, type-correctness, paper, model-quality, arena IR,
  generated hot-path, or JSON Schema output claims, the Go real-source
  generated-codec corpus completion handoff gate consumes that completed corpus
  plus its source-program handoff inputs,
  and the stable metadata promotion implementation gate consumes the handoff's
  source-program stable report fields, source-program blocked claims,
  type-checker report identities, pass-envelope policy, generated-codec report
  commands, source-program and generated-codec no-stable claims, supported
  record counts, and record identities. The source-program handoff profile now
  consumes those stable metadata source inputs, and the external validation
  controls gate now consumes that source-program handoff source input summary,
  the external validation execution preservation gate now preserves the
  execution report's source handoff source-input fields plus the C#
  generated-codec completion handoff, and the type-checker feasibility report
  now pins the existing external-validation source inputs while
  the type-checker execution report now pins the feasibility source inputs,
  and the type-checker execution preservation gate now pins the execution
  source inputs. The Go real-source source-program validity corpus gate now
  pins the type-checker preservation source input summary, the Go real-source
  generated-codec corpus gate now pins the source-program validity corpus
  source-input summary, the Go real-source generated-codec completion handoff
  gate now pins the generated-codec corpus source-input summary, and the stable
  metadata promotion gate now pins the completion handoff source-input summary.

That audit inventory remains report-only unless `--fail-on-gaps` is requested,
matching the existing audit behavior for other route-readiness and quality
inventories. The generated-code compilation gate remains syntax-route scoped
and does not route `StructuralModel` or `GrammarIR` through the production JSON
Schema compiler. The maintained local runner
`./scripts/testing/test-go-real-source-generated-codec-corpus.sh` now executes
the five Go real-source generated-codec corpus exact tests while keeping parser,
formatter, type-checker, corpus eligibility, and generated-codec sidecar results
under the checked claim boundary. The cross-target invocation equivalence gate
now proves exact zero-argument return/call/callee categories for the matched Go
`recover()` call-return and C# `Format()` invocation-return sidecars, and string
argument categories for the matched Go `len("world")` call-return and C#
invocation-with-argument return sidecars, plus the matched Go/C# if/else
control-flow categories for condition, branch bodies, branch assignment, and
return payloads, while keeping the C# zero-argument empty argument-list wrapper,
C# parameter payloads, C# else wrappers, and target-specific operator/type
tokens outside the shared boundary claim. It also proves the matched Go/C#
condition-only loop categories for loop statement, condition, body, branch
assignment, and return payloads while keeping C# loop header/setup fields out
of the shared claim, and the matched Go/C# callable-wrapper categories for
declaration, name, return type, body block, and return literal over the
integer-return syntax routes while keeping Go file/package/function-type
wrappers and C# namespace/class/member wrappers target-specific. It now also
proves callable parameter-list categories for parameter-list edge, parameter
binding, parameter name payload, and parameter type payload over the Go
minimal-function and C# two-parameter method syntax routes while keeping exact
parameter counts and Go import wrappers target-specific. The gate now also
proves member-list container categories for repeated member/declaration list,
data/type member declaration, callable member declaration, member name payloads,
and member type payloads over the Go package type declaration and C#
property-plus-method syntax routes while keeping Go type-spec wrappers, C#
property initializer, C# method return-identifier spelling, and member ordering
target-specific. It also proves data-member field/property categories for
data-member container, data-member binding, member name payload, and member
type payload over the Go struct type and C# property-plus-method syntax routes
while keeping Go struct-field wrappers and C# folded property binding
target-specific. It now proves method-member callable categories for method
declaration/container, method name payload, return type payload, body block,
return statement, and return expression edge over the Go method declaration and
C# property-plus-method syntax routes while keeping Go receiver list/name/type
binding, Go method return literal, and C# method return-identifier spelling
target-specific. It now proves package/class data declaration categories for
data declaration container, data declaration binding, data declaration name
payload, data declaration type payload, initializer field profile, companion
callable member, and companion return edge over the Go package var declaration
and C# property-plus-method syntax routes while keeping Go package var keyword,
Go package value-list cardinality, Go package initializer literal, C# property
initializer string payload, member ordering, and source-program validity
target-specific. It also proves collection/composite initializer categories for
callable/member container, collection creation node, element type reference and
payload, repeated initializer element list, and integer element payloads over
the Go array composite literal and C# collection initializer syntax routes
while keeping Go composite literal spelling, C# array-creation initializer
wrapper, keyed composite values, and source-program validity target-specific.
It now proves index/element-access categories for callable/member container,
parameter array/element type, index target identifier, bracketed/index argument
list, and integer index payload over the Go index expression and C# element
access syntax routes while keeping Go function/file wrappers, C# namespace and
class wrappers, C# element-access argument-list wrappers, slice/range,
nullable/index-from-end syntax, and source-program validity target-specific.
It now proves slice/range categories for callable/member container, parameter
array/element type, range target identifier, slice/range expression, and
integer lower/start payload over the Go slice expression and C# range
element-access syntax routes while keeping Go open-ended high-bound absence,
C# bounded range right operand, C# range argument-list wrappers,
nullable/index-from-end syntax, and source-program validity target-specific.
The Go/C# boolean return and nil/null return `StructuralModel` parity gates now
compare the route-backed callable wrapper, return type, return statement, and
payload leaf while preserving Go boolean identifier spelling, C# boolean
literal spelling, Go nil identifier spelling, and C# null literal spelling as
target-specific evidence. The C# nullable return type route preserves `string?`
as target-specific syntax evidence and is pinned in the sidecar-only
source-program typecheck control envelope. The C# nullable local declaration
route `csharp_local_nullable_declaration_syntax_v0` preserves `string? value =
null;` as target-specific syntax evidence and is pinned in the sidecar-only
source-program typecheck control envelope. The C# nullable property declaration
route `csharp_nullable_property_declaration_syntax_v0` preserves `public
string? Name { get; init; } = null;` as parser-backed member syntax evidence
and is pinned in the sidecar-only source-program typecheck control envelope.
The C# nullable parameter declaration route
`csharp_nullable_parameter_declaration_syntax_v0` preserves `string? name` in
callable signature context as parser-backed syntax evidence and is pinned in
the sidecar-only source-program typecheck control envelope. The C# nullable
context `StructuralModel` equivalence gate now compares the route-backed
return, local declaration, property declaration, and parameter declaration
contexts by asserting the parser-backed owner references into
`NullableTypeSyntax`, the shared `NullableTypeSyntax.ElementType` reference,
and the `QuestionToken` string payload leaf. The C# fixture route-corpus
toolchain evidence preservation residual was retired with the arena frontend
evidence consumers; current C# real-source selection uses
`scripts/testing/test-csharp-real-source-corpus.sh` and
`corpora/csharp/dotnet-samples-mini/manifest-v1.json` only. The C#
representative real-source corpus selection gate now preserves the
`csharp-dotnet-samples-mini-v1` source-list identity, non-fixture corpus
fingerprint, selected file hashes, MIT provenance, `dotnet` availability
requirement, and route-readiness summary while preserving the JSON Schema
backend. The C# generic-name sidecar route now lowers `GenericNameSyntax`,
`TypeArgumentListSyntax`, and a generic type-argument payload through checked
Roslyn metadata into `GrammarIR`, `StructuralModel`, and generated-codec
evidence. The C# qualified-name sidecar route now lowers `QualifiedNameSyntax`
and identifier-name edges through checked Roslyn metadata into `GrammarIR`,
`StructuralModel`, and generated-codec evidence. The C# constructor-declaration
sidecar route now lowers `ConstructorDeclarationSyntax`, parameter-list,
block-statement, and identifier assignment edges through the same checked
syntax boundary. The C# while/postfix loop sidecar route now lowers
`WhileStatementSyntax`, `PostfixUnaryExpressionSyntax`, loop condition, integer
literal, and identifier edges through checked Roslyn metadata into `GrammarIR`,
`StructuralModel`, and generated-codec evidence. The C# interpolated-string
sidecar route now lowers `InterpolatedStringExpressionSyntax`,
`InterpolatedStringTextSyntax`, `InterpolationSyntax`, parameter, and
identifier edges through the same checked Roslyn metadata path. The C#
member-access expression sidecar route now lowers
`MemberAccessExpressionSyntax` target/name edges through checked Roslyn
metadata into the same sidecar path. The C# field declaration sidecar route
now lowers `FieldDeclarationSyntax`, variable declaration, predefined type,
and variable declarator edges through checked Roslyn metadata into the same
sidecar path. The C# base-list sidecar route now lowers `BaseListSyntax`,
`SimpleBaseTypeSyntax`, and identifier-name base type edges through checked
Roslyn metadata into the same sidecar path. The C# conditional-access sidecar
route now lowers `ConditionalAccessExpressionSyntax`,
`MemberBindingExpressionSyntax`, invocation, and argument edges through checked
Roslyn metadata into the same sidecar path. The C# array-rank/object-creation
sidecar route now lowers `ObjectCreationExpressionSyntax`,
`ArrayRankSpecifierSyntax`, `OmittedArraySizeExpressionSyntax`, array creation,
and initializer edges through checked Roslyn metadata into the same sidecar
path. The C# `this` expression sidecar route now lowers `ThisExpressionSyntax`
argument edges through checked Roslyn metadata into the same sidecar path. The
C# pattern-expression sidecar route now lowers `IsPatternExpressionSyntax` and
`ConstantPatternSyntax` edges through checked Roslyn metadata into the same
sidecar path. The C# arrow-expression-clause sidecar route now lowers
`ArrowExpressionClauseSyntax` expression-body edges through checked Roslyn
metadata into the same sidecar path. The real-source gap gate now counts
completed assignment/expression-statement, qualified-name, constructor,
while/postfix, interpolated-string, member-access expression, field
declaration/base-list, conditional-access, array-rank/object-creation, and
`this` expression/pattern-expression/arrow-expression-clause routes as
supported and reports zero remaining C# syntax-metadata gap classes, leaving
broad source-language, paper, model-quality, type-correctness, completion,
generated hot-path, stable backend, generated-codec real-source, and arena-IR
claims blocked. The checked source-corpus gate now carries that no-gap result
as `csharp_no_gap_metadata_report`, and the checked C# generated-codec
feasibility gate records `csharp_compilation_unit_source_file_syntax_v0` as the
supported whole-source generated-codec route for
`pattern-matching/Shapes.cs`, `delegates-and-events/Logger.cs`,
`events/VersionOne.cs`, `iterators/IteratorMethods.cs`, and
`serialization/Loan.cs`. The checked C# generated-codec corpus gate preserves
those round-trip reports under `target/tct-syntax-real-source-demo/`, the
checked C# generated-codec completion handoff consumes that completed corpus,
and the source-program external-validation controls plus execution-preservation
gate now carry the C# completion handoff as sidecar evidence. The
type-checker feasibility, execution, and execution-preservation gates now
preserve the same C# completion handoff beside the Go handoff in the checked
stable report-field contract; the remaining downstream consumer-contract
blocker stays tracked in the research roadmap.
