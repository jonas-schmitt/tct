# Non-JSON Frontend Target Selection

> **Experimental-frontend note.** This document was written inside the private
> TCT research repository and is kept as a contract reference for the
> experimental non-JSON frontends, which are compiled only with the
> `experimental` Cargo feature. It may cite roadmap, report, or
> sibling-repository documents that are not part of this repository; treat
> those citations as historical context. The stable JSON Schema core is
> documented in `README.md`, `ARCHITECTURE.md`, and `DEVELOPMENT.md`.

This memo records the current target-selection rationale for non-JSON TCT
frontends. It is planning evidence, not paper evidence. Claims about model
quality, functional usefulness, or generalization beyond JSON Schema still need
the cross-repo provenance envelope in
the cross-repository provenance notes.

Status: the current recommendation is syntax/grammar-level Go first, followed
by C# parity on the same boundary. Historical GoIR route-decision material in
this memo is historical context, not active roadmap direction. Treat GoIR,
ArenaIR, and other non-main-track material as cleanup candidates unless a
current retained test proves direct utility for JSON Schema TCT, Go syntax TCT,
C# syntax TCT, or shared raw syntax-state-machine infrastructure. This memo
must not override the research strategy notes or the active queue in the research roadmap.

## Selection Criteria

Compare candidate frontends on:

- Parser and toolchain availability: stable parsers, formatters, validators,
  type checkers, and licenses.
- Corpus quality: accessible real-world files with meaningful structure and
  enough diversity to measure tokenization behavior.
- Functional validator availability: a tool that can check decoded artifacts
  beyond syntax where practical.
- Compiler and tokenizer complexity: how much new lowering, renderer,
  recursion, payload handling, and metadata is needed before round trips work.
- Reviewer value: how directly the target addresses feedback about domain
  scope, grammar-integrated baselines, functional usefulness, pretrained
  fine-tuning, and retraining cost.

## Recommendation

Use Go as the first syntax/grammar-level programming-language target.

Go has a stable standard-library parser, AST, formatter, and optional type
checker. The forward TCT route is parser-backed syntax metadata lowered through
`GrammarIR -> StructuralModel -> token layout -> generated codec`, with
rendered output checked by `gofmt` and `go/parser` as sidecar validation. This
is the source-language analog of the JSON Schema validity-by-construction
claim.

Historical GoIR evidence was a baseline/control route only; residual GoIR
fixtures and operational helpers were retired once the syntax-level path owned
forward evidence. GoIR is not the forward backend boundary and does not satisfy
source-language validity by construction. New Go work should be driven by
syntax-level corpus gaps and generated-codec evidence, not by restoring
arena-indexed GoIR coverage.

The next broad target is already selected by the research strategy notes: C#
is the architecture-first second target and the syntax-level cross-target
check. The retired target-selection, route-strategy, and frontend-readiness
report ladder is no longer maintained and should not be refreshed or used to
select new work. The current C# signal is the parser-backed C# syntax metadata
and generated-codec route evidence preserved by the syntax backend gates and
the active queue in the research roadmap.
The first syntax-level C#
signal is now the checked minimal C# syntax metadata fixture that lowers through
`GrammarIR -> StructuralModel` with source hash and `.NET`/Roslyn provenance.
The second live parser-backed syntax signal is the checked minimal Go syntax
metadata fixture, reproduced from source with `go/parser` / `go/ast` and local
`go1.24.3` provenance. The cross-target syntax decision comparison now confirms
that the C# fixture, Go fixture, and existing toy/Tree-sitter metadata path
expose the same required target-agnostic sidecar categories. A scoped
Roslyn-backed metadata generator smoke reproduces the checked C# metadata
projection from the source file with `Microsoft.CodeAnalysis.CSharp 5.3.0`
provenance. The target-agnostic syntax backend gate strategy is now maintained
in `docs/syntax-backend-gate-strategy.md`. The generated-code gate now compiles
sidecar-scoped toy, C#, and Go syntax codec fixtures, and the audit reports
them as one shared target set. Go syntax now has explicit BPE boundary evidence
for its syntax token layout. Parser-backed C# and Go generated codec
reproducibility metadata now records rendered source, parser-backed grammar
input, structural model, token layout, source metadata provenance, generated
crate identity, and sidecar boundary flags. Parser-backed C# and Go generated
codecs now have matching strict/recovery decode policy evidence with
consumed-token plus partial-field accounting. The syntax `StructuralModel`
boundary conformance gate and versioned boundary manifest now prove the toy,
C#, and Go sidecars share the selected boundary profile. The syntax
`StructuralModel` stable-gate plan now maps that manifest to JSON-backend-grade
gate categories, and generated-code metadata records the manifest identity for
toy, C#, and Go sidecar-generated codecs. The stable-gate helper now runs that
sidecar gate bundle explicitly. The M6 cross-target contract closure report
`reports/m6-cross-target-contract-closure-2026-07-21.md` records shared
Go/C# contract evidence (layer-3 stable promotion remains a separate track), and
`tests/fixtures/syntax_backend/production-boundary-contract-v1.json` pins the
future production handoff while preserving the sidecar boundary.
`src/syntax_backend_boundary.rs` now defines the production generator API
boundary with focused coverage in `tests/syntax_backend_boundary_test.rs`. The
generated artifact envelope fixture now lives at
`tests/fixtures/syntax_backend/generated-artifact-envelope-v1.json` with
validation support in `src/syntax_artifact_envelope.rs`, and generated-code
metadata now records that envelope for toy, C#, and Go. The validation-hook
contract fixture now lives at
`tests/fixtures/syntax_backend/validation-hook-contract-v1.json` with
validation support in `src/syntax_validation_hooks.rs`, and generated-code
metadata now records that contract for toy, C#, and Go. The input-validation
diagnostics contract fixture now lives at
`tests/fixtures/syntax_backend/input-validation-diagnostics-v1.json` with
validation support in `src/syntax_input_diagnostics.rs`, and generated-code
metadata now records that diagnostics contract for toy, C#, and Go. The
negative fixture gate now lives at
`tests/fixtures/syntax_backend/input-validation-negative-fixtures-v1.json`.
Generated-code metadata now carries that negative fixture identity for toy,
C#, and Go, including direct parser-backed C# and Go negative records. The
validation-hook sidecar evidence fixture now lives at
`tests/fixtures/syntax_backend/validation-hook-sidecar-evidence-v1.json`.
Current execution priority belongs in the research roadmap, not in this target-selection
memo.
The arena-control route also has a mixed property plus block-identifier
member-list fixture to pin arena indexing, and a two-parameter
expression-bodied method fixture plus a mixed two-parameter member-list fixture
to prove the next parameter-list slice. A two-parameter block-bodied literal
fixture covers the matching block-return path, and a mixed two-parameter
block-bodied literal fixture pins member ordering for that block-return path.
The two-parameter identifier-return expression fixture confirms declared
parameter references in the expression-bodied path, and the two-parameter
block-bodied identifier-return fixture confirms the same reference check under
the single-return block shape. The mixed two-parameter block-bodied
identifier-return fixture composes that block shape with a property member and
pins member ordering for the identifier-return path. An escaped regular string
literal member fixture pins parse-time source escape normalization and
render-time canonical escaping through the same route. Second-parameter
identifier-return fixtures pin the non-first declared parameter branch for both
expression-bodied and block-bodied method routes. A local string declaration
plus return fixture pins ordered two-statement blocks and local identifier
binding. The mixed local-variable member-list fixture composes that two-statement
block with a property member and pins arena member ordering. An escaped local
string declaration fixture proves the same source escape normalization and
canonical render escaping for local initializers. The mixed escaped-local
member-list fixture composes that escaped initializer path with a property
member and pins arena member ordering. A local identifier-initializer fixture
proves the single-parameter binding path inside local declarations without
assignment or mutation. The mixed local identifier-initializer fixture composes
that parameter-binding path with a property member and pins arena member
ordering before assignment support. A local assignment-update fixture proves a
minimal three-statement block with one local assignment while keeping calls,
member access, and broader control flow out of scope. The mixed local
assignment-update fixture composes that three-statement block with a property
member and pins arena member ordering. A local literal assignment-update fixture
pins the same assignment statement with a regular string literal expression.
The mixed local literal assignment-update fixture composes that literal branch
with a property member and pins arena member ordering. An escaped literal
assignment-update fixture pins source escape normalization and canonical
rendering for assignment expressions. The mixed escaped literal assignment
fixture composes that escaped assignment branch with a property member. The
no-parameter literal assignment-update fixture pins the parameterless branch of
the same local assignment shape with regular string literals. The mixed
no-parameter literal assignment-update fixture composes that parameterless
branch with a property member and pins arena member ordering. The no-parameter
escaped literal assignment-update fixture pins escaped assignment normalization
inside the parameterless branch. The mixed no-parameter escaped assignment
fixture composes that branch with a property member. A method-before-property
fixture pins reversed member order instead of only property-first mixed member
lists. A two-method fixture pins method-to-method member ordering before wider
member-list composition. A property-plus-two-method fixture composes the
two-method shape with a leading property member. A method-property-method
fixture pins property placement between two supported methods. Java remains the
closest architecture peer and fallback, while TypeScript/JavaScript is the closest
downstream-first alternative because of corpus availability, current TSConfig
relevance, parser ecosystem maturity, and Hugging Face experiment fit. A
two-property fixture pins repeated property member indexing, and a
property-property-method fixture pins a trailing supported method after
repeated property members. A property-method-property fixture pins a supported
method between repeated properties, and a method-property-property fixture pins
repeated properties after a leading method. A method-method-property fixture
completes the two-method/one-property member-order permutations, while a
property-method-property-method fixture pins a four-member alternating list.
A method-property-method-property fixture pins the opposite four-member
alternating order. A property-property-method-method fixture pins grouped
property then method traversal, and a method-method-property-property fixture
pins the opposite grouped method then property traversal. A three-parameter
expression-bodied identifier-return fixture lifts the plain method arity
boundary, and a three-parameter block-bodied identifier-return fixture pins the
same arity through a single-return block while keeping broader statement and
local-variable forms out of scope. A property-plus-three-parameter
expression-bodied member-list fixture composes that arity with an existing
property member, and a property-plus-three-parameter block-bodied fixture pins
the same member-list composition through a single-return block. A
three-parameter method-before-property fixture pins the opposite member order
for the same expression-bodied arity, and a three-parameter block-bodied
method-before-property fixture pins that opposite member order through a
single-return block. A three-parameter method-plus-method fixture composes the
arity with an existing second method member, and a no-parameter
method-before-three-parameter fixture pins the opposite expression-bodied method
order. A three-parameter block-bodied method-plus-method fixture pins the
method-pair composition through a single-return block, and a no-parameter
method-before-three-parameter block-bodied fixture pins the opposite
block-bodied method order. A property-method-three-parameter block-bodied
fixture composes a property, no-parameter method, and high-arity block-bodied
method in one member list, and a property-three-parameter block-bodied
method-method fixture pins the opposite method order under the same
property-prefixed composition.
Protocol Buffers, GraphQL/OpenAPI, and XML Schema remain useful side-validation
targets.

The historical first-target route decision accepted Go as the first non-JSON
target and accepted the JSON-Schema-backed arena GoIR bridge as a control route
after the pinned clean standard-library corpus passed generated TCT
encode/decode, canonical render, source-BPE, and Go lexer evidence. Current
forward work is syntax/grammar-level Go coverage, not new GoIR implementation.

Do not choose Tree-sitter or ANTLR as the durable grammar input before a spike.
The spike should compare grammar coverage, node/type metadata, ambiguity
handling, incremental parser support, generated parser footprint, licensing,
ecosystem quality for Go/TypeScript/JavaScript, and how cleanly each lowers
into a normalized `GrammarIR` that can then lower into `StructuralModel`.

The durable architecture direction is recorded in
`docs/structural-model-boundary-selection.md`, the research strategy notes, and
`docs/grammar-ir-contract.md`: keep the first path source-preserving through
parser-backed syntax metadata, use `StructuralModel` as the candidate shared
structural backend boundary only after repeated route evidence, and keep
`GrammarIR`, Tree-sitter, and ANTLR as upstream grammar-ingestion questions.
Keep LLVM IR as a deferred negative/control baseline, and revisit MLIR only
after at least two source-level frontends prove the shared abstraction needs a
dialect host. The historical non-JSON source-IR strategy report snapshot was
retired with the GoIR/C# arena surfaces; see `reports/README.md` and do not
restore it as a workflow pointer.

The durable source-frontend token-layout contract is documented in
`docs/source-frontend-token-layout-manifest.md`. GoIR-era checked layout
manifest fixtures under `tests/fixtures/go_syntax_sources/` were retired with
the residual GoIR investigation family; do not regenerate them.

## Retired Route-Planning Ladder

The old route-planning report ladder has been retired. Do not regenerate it as
a prerequisite for syntax-level Go or C# work. Use the research strategy notes,
the roadmap guardrails, the research roadmap, and the syntax backend gate fixtures
for current target order, boundary selection, and validation requirements.

Retained route-input helpers may still be used when a current syntax-level task
needs source manifests or route-input preflights:

```bash
python scripts/analysis/frontend_route_inputs.py --self-test
python scripts/analysis/route_source_manifest.py --self-test
python scripts/analysis/route_source_manifest.py \
  --target csharp \
  --source-root path/to/csharp/root \
  --output target/tct-analysis/csharp-sources.txt \
  --json-output target/tct-analysis/csharp-sources.manifest.json \
  --markdown-output target/tct-analysis/csharp-sources.manifest.md \
  --min-files 2 \
  --exclude-glob 'generated/*' \
  --corpus-id csharp-route-corpus-v1 \
  --corpus-source path-or-uri \
  --corpus-license license-label \
  --selection-note 'filtered generated sources'
python scripts/analysis/frontend_route_inputs.py \
  --target csharp \
  --source-manifest path/to/csharp-sources.txt \
  --source-manifest-report target/tct-analysis/csharp-sources.manifest.json \
  --source-root path/to/csharp/root \
  --require-inputs-ready
```

Any new route-input evidence should preserve:

- Candidate matrix: language or format, parser/toolchain, formatter/renderer,
  validator, corpus source, license constraints, and expected fixture subset.
- Corpus plan: pinned manifest shape, source hashes, filtering rules, and
  minimum held-out fixtures needed before implementation.
- Toolchain smoke: parse, render or format, reparse, functional validation when
  available, and expected failure classes.
- Frontend fit: arena IR shape, recursion handling, payload/string handling,
  comments and trivia policy, canonicalization policy, and unsupported syntax
  boundaries.
- Tokenizer fit: structural versus payload split, token-range invariants, BPE
  baseline needs, source-lexer baseline availability, and generated-code size
  or build-time risks.
- Decision: why the target is the next implementation target, a side-validation
  target, or deferred; which first failing tests would lock the route contract.

The spike must not start a durable GoIR, TSIR/JSIR, JavaIR, C# IR, or other
implementation until the first-target evidence and frontend route decision are
current. The checked C# first-contract fixture is current evidence for only the
`minimal_class` route under `tests/fixtures/csharp_syntax_sources/minimal_class/`
and the matching metadata under
`tests/fixtures/csharp_syntax_metadata/minimal_class/`; it is not broad C#
frontend readiness. The historical csharp-first route-contract report was
retired with the GoIR/C# arena surfaces; see `reports/README.md`.

## Candidate Comparison

| Target | Tooling | Corpus | Functional validation | Complexity | Reviewer value | Current role |
| --- | --- | --- | --- | --- | --- | --- |
| Go | Excellent standard parser, AST, formatter, and optional type checker | High-quality open-source corpora are easy to pin | `gofmt`, `go/parser`, later `go/types` | Moderate for syntax-level corpus closure; historical arena GoIR remains lower-risk control evidence | Strong for beyond-JSON scope and deterministic-language claims | First syntax/grammar target |
| TypeScript/JavaScript | Strong parser ecosystem, TypeScript compiler, Tree-sitter grammars | Very high corpus availability | `tsc`, eslint, parser/reparse checks depending on subset | Higher because JS syntax, TSX/JSX, decorators, regex/template strings, comments, and project context need scoping | Very strong due to current schemas, pretrained models, and downstream HF experiments | High-impact candidate, not preselected |
| Java | Stable parsers and formatters through `javac`, JavaParser, or Tree-sitter | High-quality open-source corpora are easy to pin | `javac` parse/compile checks for scoped fixtures | Moderate; generics, annotations, comments, and package context need policy | Strong second-language architecture signal with less syntax ambiguity than JS | Architecture peer/fallback |
| C# | Roslyn parser, compiler, semantic model, and formatting APIs are mature | Good corpus availability once project-context-heavy files are filtered | Roslyn parse/compile checks | Moderate; project files, preprocessor directives, attributes, partial types, and interpolation need first-slice scoping | Strong architecture signal for typed language breadth and explicit frontend API boundaries | Preferred architecture-validation candidate if feasibility matches Java |
| Protocol Buffers | Stable `protoc`, descriptors, and schemas | Good schema corpora, smaller text diversity | `protoc` compile and descriptor checks | Moderate; schema-first and deterministic | Good support for deterministic grammar claims, less direct to current deployments than Go/TS | Side validation or later target |
| SQL DDL | Mature parsers, but dialect split matters | Good, dialect-specific corpora | Database parse/plan checks or schema loaders | High because dialect and semantics must be fixed | Strong for reviewer relevance and functional usefulness | Later target after frontend abstraction exists |
| SQL queries | Mature parsers, but dialect and runtime semantics dominate | High corpus availability | Parse, plan, or execute against controlled fixtures | High due to name binding, dialects, and execution context | Strong but easy to overclaim | Later functional-evaluation target |
| Terraform/HCL | Good parser and formatter, provider/schema tooling exists | Strong infrastructure relevance | `terraform validate` with controlled providers | High because providers, interpolation, modules, and external state matter | Strong for infrastructure and CI/CD relevance | Later domain-specific target |
| XML Schema | Mature schema tooling | Available but less aligned with current training paths | Schema validation and round trips | Moderate as a schema-language comparison | Useful for schema-language breadth, weaker for pretrained code-token story | Side validation |
| Lean | Parser and elaborator available through Lean toolchain | Smaller but high-value proof corpus | Parse/elaborate checks with pinned toolchain | High because elaboration context, imports, and proof-state semantics matter | Strong formal-methods story but not an immediate implementation proof | Feasibility target |

GraphQL and OpenAPI remain useful domain-specific side targets, but they should
not displace programming-language evidence unless a refreshed spike shows their
validator strength and corpus fit make them the better post-Go target.

## Cross-Repo Alignment

`the downstream paper repository/reviews` motivates three constraints:

- Reviewer-facing generalization needs more than JSON Schema, but non-JSON work
  must not be reported as paper evidence without preserved artifacts and
  commands.
- Grammar-integrated vocabularies and action-sequence baselines should be
  compared against shared fixtures instead of being conflated with the Go
  bridge.
- Functional usefulness needs a downstream validator path; syntax validity and
  tokenizer round trips are necessary but not sufficient.

the downstream training repository motivates the ownership boundary:

- Downstream fine-tuning, pretrained tokenizer integration, semantic-quality
  evaluation, and multi-seed reports stay outside this repository.
- This repository should provide generated tokenizer artifacts, wrapper
  metadata, payload bounds, BOS/EOS/schema-token contracts, and provenance
  needed by those experiments.
- TSConfig remains the natural first Hugging Face structured-response schema;
  TypeScript source frontends are a related but separate mid-term extension.

## Historical Acceptance For The GoIR Control Bridge

This section is retained as provenance only. It does not define current forward
success for source-language TCT; the current success path is syntax/grammar-level
Go coverage and C# `minimal_class` on the shared boundary.

The historical GoIR bridge once recorded pinned corpus manifests, parse/render
success, TCT encode/decode over GoIR JSON, source-BPE and lexer baselines, and
token-range invariants. Those operational surfaces (`test-goir-evidence.sh`,
`tct-goir-evidence`, residual GoIR structural-model and token-layout fixtures)
were retired with the GoIR residual investigation family. Do not revive GoIR
route-decision workflows, regenerate GoIR evidence packages, or treat retired
helpers as current gates. Forward coverage and claim-grade evidence use the
inventory-derived Go syntax artifact, syntax gates, and M2/M4/M5 reports.
