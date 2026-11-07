# Syntax StructuralModel Stable Gate Plan

> **Experimental-frontend note.** This document was written inside the private
> TCT research repository and is kept as a contract reference for the
> experimental non-JSON frontends, which are compiled only with the
> `experimental` Cargo feature. It may cite roadmap, report, or
> sibling-repository documents that are not part of this repository; treat
> those citations as historical context. The stable JSON Schema core is
> documented in `README.md`, `ARCHITECTURE.md`, and `DEVELOPMENT.md`.

This plan maps the versioned syntax `StructuralModel` boundary manifest onto
the same gate categories used for the stable JSON Schema backend. It is an
implementation plan for the feature-limited stable syntax route, not a broad
source-language or model-quality promotion.

The required boundary input is
`tests/fixtures/syntax_structural_model/boundary-manifest-v1.json`. That
manifest records artifact kind `tct_syntax_structural_model_boundary_manifest`,
version 1, the `GrammarIR -> StructuralModel` sidecar boundary, and the current
toy, C#, and Go syntax targets.

## Gate Mapping

| Category | Manifest-backed requirement |
| --- | --- |
| Input validation | Validate that each generated syntax target is present in the manifest with source or metadata hash, parser provenance when parser-backed, `structural-model-v0` identity, proof checks, and unchanged compiler-boundary flags. The checked input-validation diagnostics contract in `src/syntax_input_diagnostics.rs` and `tests/fixtures/syntax_backend/input-validation-diagnostics-v1.json` now pins required input fields, diagnostic fields, unsupported syntax error shape, source-or-metadata SHA-256, parser-or-grammar identity, sidecar status, and non-goals. The negative fixture gate in `tests/fixtures/syntax_backend/input-validation-negative-fixtures-v1.json` proves malformed and unsupported toy, C#, and Go checked-input records carry rule/path diagnostics, source-or-metadata SHA-256, grammar or parser identity, sidecar boundary SHA-256, and diagnostics contract SHA-256. External parser, renderer, formatter, compiler, and functional checks remain sidecar evidence unless a later gate explicitly promotes one. |
| Generated-code compilation | The first stable-gate implementation hook is the manifest-backed syntax generated-code gate handoff in `tests/syntax_generated_codec_gate_test.rs`. The generated codec gate consumes the manifest before compiling toy, C#, and Go sidecar-generated codecs rendered by `src/syntax_generated_codec.rs`. |
| Semantic round trips | Keep the existing toy assignment, C# minimal-class, local return/prefix-unary/parenthesized/binary/declaration/assignment/control-flow/iteration/parameter-list/collection-initializer/element-access/range-element-access fixtures, and Go minimal-function semantic round trips tied to manifest target ids and source identities, rather than treating target-specific arena IR shape as backend evidence. The Go minimal-function path now includes the sidecar-only `strings.Compare` real-source generated-codec demo with `gofmt` plus `go/parser` validation and a volatile token-count report under `target/tct-syntax-real-source-demo/`. The cross-target equivalence gate in `tests/syntax_structural_model_cross_target_equivalence_test.rs` compares matched Go and C# local constructs, including the Go short-var/C# implicit-local pair, the Go integer short-var/C# integer implicit-local pair, the Go/C# integer, float, and boolean return pairs, the zero-argument invocation categories for the Go no-argument call-return and C# zero-argument invocation-return sidecars, the argument-call categories for `go_local_argument_call_return_expression_syntax_v0` where `len("world")` carries a `CallExpr.Args -> BasicLit` edge comparable to the C# invocation argument literal edge, the if/else control-flow categories for condition, then-body, else-body, branch assignment, and return payloads, the condition-only loop categories for loop statement, condition, body, branch assignment, and return payloads, the single-case switch categories for selector/expression, case label, case body assignment, and return payloads, the string-iteration categories for range/foreach loop, string source literal, loop body assignment, and return payloads, the callable-wrapper categories for declaration, name, return type, body block, and return literal over the Go/C# integer-return routes, the float return literal route pair, and the boolean return route pair, the callable parameter-list categories for parameter-list edge, parameter binding, parameter name payload, and parameter type payload over `go_minimal_function_syntax_v0` and `csharp_two_parameter_method_syntax_v0`, the member-list container categories for repeated member/declaration list, data/type member declaration, callable member declaration, member name payloads, and member type payloads over `go_package_type_declaration_syntax_v0` and `csharp_property_plus_method_syntax_v0`, the data-member field/property categories for data-member container, data-member binding, member name payload, and member type payload over `go_struct_type_syntax_v0` and `csharp_property_plus_method_syntax_v0`, the method-member callable categories for method declaration/container, method name payload, return type payload, body block, return statement, and return expression edge over `go_method_declaration_syntax_v0` and `csharp_property_plus_method_syntax_v0`, the package/class data declaration categories for data declaration container, data declaration binding, data declaration name payload, data declaration type payload, initializer field profile, companion callable member, and companion return edge over `go_package_var_declaration_syntax_v0` and `csharp_property_plus_method_syntax_v0`, the collection/composite initializer categories for callable/member container, collection creation node, element type reference and payload, repeated initializer element list, and integer element payloads over `go_array_composite_literal_syntax_v0` and `csharp_collection_initializer_syntax_v0`, the index/element-access categories for callable/member container, parameter array/element type, index target identifier, bracketed/index argument list, and integer index payload over `go_index_expression_syntax_v0` and `csharp_element_access_syntax_v0`, the slice/range categories for callable/member container, parameter array/element type, range target identifier, slice/range expression, and integer lower/start payload over `go_slice_expression_syntax_v0` and `csharp_range_element_access_syntax_v0`, the unary-expression categories for callable/member container, return body, unary expression edge, unary operator payload, and integer operand payload over `go_local_unary_integer_return_expression_syntax_v0` and `csharp_local_unary_integer_return_syntax_v0`, the binary-expression categories for callable/member container, return body, binary expression edge, binary operator payload, and left/right integer operand payloads over `go_local_binary_integer_return_expression_syntax_v0` and `csharp_local_binary_integer_return_syntax_v0`, and the boolean return categories for callable wrapper, return type, return statement, and boolean payload leaf over `go_local_boolean_return_identifier_syntax_v0` and `csharp_local_boolean_return_literal_syntax_v0`, at the `StructuralModel` boundary before any stable backend promotion. The gate records that argument-list evidence exists on both syntax targets for the argument-call, index/element-access, and slice/range slices, keeps the C# zero-argument empty argument-list wrapper and C# argument-list and parameter payload evidence target-specific, and keeps Go file/package/function-type wrappers, Go import wrappers, exact parameter counts, Go type-spec wrappers, Go struct-field wrappers, C# folded property binding, C# property initializer, Go package var keyword, Go package value-list cardinality, Go package initializer literal, C# property initializer string payload, Go receiver list/name/type binding, Go method return literal, C# method return-identifier spelling, member ordering, C# namespace/class/member wrappers, Go composite literal spelling, C# array-creation initializer wrapper, C# element-access ArgumentList/ArgumentSyntax wrappers, C# range argument-list wrappers, keyed composite values, Go/C# if/else, loop setup/header fields, switch section/label wrappers, C# break statements, Go range binding absence, C# foreach type/header wrappers, C# foreach iteration variables, Go open-ended high-bound absence, C# bounded range right operand, Go `UnaryExpr.Op`/`UnaryExpr.X` spelling, C# `PrefixUnaryExpressionSyntax.OperatorToken`/`Operand` spelling, Go `BinaryExpr.Op`/`X`/`Y` spelling, C# `BinaryExpressionSyntax.OperatorToken`/`Left`/`Right` spelling, Go boolean identifier spelling, C# boolean literal spelling, plus target-specific wrappers and operator/type tokens, outside the shared boundary claim. |
| Strict/recovery decode behavior | Keep strict EOF, trailing-token, malformed structural token, malformed payload, and recovery-normalization policy evidence target-scoped, and record which manifest target each generated-code decode contract belongs to. |
| Consumed-token accounting | Keep complete, surplus, truncated, malformed, and partial-prefix consumed-token accounting in the generated-code gate, and link those checks to the matching manifest target. |
| Token/vocabulary/BPE invariants | Use manifest token-range roles and proof checks as the cross-target boundary input for `VOCAB_SIZE`, `PAD_TOKEN`, structural range, payload range, and optional BPE collision evidence. The route-boundary gate in `src/syntax_token_vocabulary_bpe_routing.rs` and `tests/fixtures/syntax_backend/token-vocabulary-bpe-routing-gate-v1.json` pins the current sidecar token layout and BPE ownership claims. |
| Generated-artifact reproducibility | Generated-code reproducibility metadata records the manifest path, artifact kind, version, SHA-256, target id, source kind, parser identity where present, and sidecar-boundary claim alongside rendered-source, grammar input, structural model, token layout, generated-crate identity, generation option hashes, generated artifact envelope identity, fixture hash, persistence policy, decode policy, token-layout metadata, validation-hook contract identity, validation-hook sidecar evidence identity, generated artifact replay identity, source checks, renderer/formatter sidecar status, external-check non-promotion policy, input diagnostics contract identity, input diagnostics negative fixture identity, fixture hash, fixture IDs, fixture kinds, required field sets, unsupported syntax error shape, diagnostics provenance, sidecar status, provenance fields, and non-goals. The route-boundary reproducibility gate in `src/syntax_reproducibility_routing.rs` and `tests/fixtures/syntax_backend/reproducibility-routing-gate-v1.json` pins deterministic hash fields, volatile report fields, source fixture hashes, replay ownership, generated-code evidence markers, and sidecar-only route state before production routing. |
| Maintained documentation | Keep `docs/syntax-backend-gate-strategy.md`, `docs/structural-model-boundary-selection.md`, `docs/structural-model-contract.md`, `DEVELOPMENT.md`, and the research roadmap aligned with the manifest-backed gate path. |
| Audit coverage | the research repository's test-quality audit must report this plan, verify these category markers, verify the generated-code handoff markers, verify the cross-target equivalence gate markers, verify the stable-gate helper markers, verify the M6 cross-target contract report markers, verify the production-boundary contract fixture, verify `src/syntax_backend_boundary.rs`, verify `src/syntax_generated_codec.rs`, verify `src/syntax_tct_compiler_routing.rs`, verify `tests/fixtures/syntax_backend/tct-compiler-routing-gate-v1.json`, verify `src/syntax_json_backend_parity.rs`, verify `tests/fixtures/syntax_backend/json-backend-parity-routing-gate-v1.json`, verify `src/syntax_target_agnostic_routing_boundary.rs`, verify `tests/fixtures/syntax_backend/target-agnostic-routing-boundary-gate-v1.json`, verify `src/syntax_strict_recovery_decode_routing.rs`, verify `tests/fixtures/syntax_backend/strict-recovery-decode-routing-gate-v1.json`, verify `src/syntax_consumed_token_accounting_routing.rs`, verify `tests/fixtures/syntax_backend/consumed-token-accounting-routing-gate-v1.json`, verify `src/syntax_token_vocabulary_bpe_routing.rs`, verify `tests/fixtures/syntax_backend/token-vocabulary-bpe-routing-gate-v1.json`, verify `src/syntax_reproducibility_routing.rs`, verify `tests/fixtures/syntax_backend/reproducibility-routing-gate-v1.json`, verify `src/syntax_audit_coverage_routing.rs`, verify `tests/fixtures/syntax_backend/audit-coverage-routing-gate-v1.json`, verify `src/syntax_stable_promotion_review.rs`, verify `tests/fixtures/syntax_backend/stable-promotion-review-gate-v1.json`, verify `src/syntax_production_route.rs`, verify `SYNTAX_PRODUCTION_ROUTE_CONSTRUCTS`, verify `tests/fixtures/syntax_backend/production-route-implementation-gate-v1.json`, verify `src/syntax_validation_matrix.rs`, verify `tests/fixtures/syntax_backend/validation-matrix-gate-v1.json`, verify `tct_syntax_backend_validation_matrix_gate`, verify `scripts/testing/run-syntax-validation-matrix.py`, verify `tct_syntax_backend_validation_matrix_execution_report`, record the retired source-program typecheck evidence family as historical inventory only (src/syntax_source_program_typecheck_execution_preservation.rs, tests/fixtures/syntax_backend/source-program-typecheck-execution-preservation-v1.json, src/syntax_source_program_typecheck_feasibility.rs, src/bin/tct-syntax-source-program-typecheck-feasibility.rs, src/syntax_source_program_typecheck_execution.rs, src/bin/tct-syntax-source-program-typecheck-execution.rs, tct_syntax_backend_source_program_typecheck_feasibility_report, tct_syntax_backend_source_program_typecheck_execution_report, tct_syntax_backend_source_program_typecheck_execution_preservation_gate), verify `src/syntax_go_real_source_program_validity_corpus.rs`, verify `tests/fixtures/syntax_backend/go-real-source-program-validity-corpus-v1.json`, verify `tct_syntax_backend_go_real_source_program_validity_corpus_gate`, verify `src/syntax_go_real_source_generated_codec_corpus.rs`, verify `tests/fixtures/syntax_backend/go-real-source-generated-codec-corpus-v1.json`, verify `tct_syntax_backend_go_real_source_generated_codec_corpus_gate`, verify parser-backed negative fixture, replay coverage, parser-backed source-corpus evidence, the source-program handoff profile, the source-program external validation controls gate, the source-program external validation execution preservation gate, the retired source-program type-checker feasibility helper, the retired source-program type-checker execution preservation gate, the Go real-source source-program validity corpus gate, the Go real-source generated-codec corpus gate, and the checked Go real-source corpus helper `scripts/analysis/go_syntax_corpus_check.go` with `tests/fixtures/go_syntax_corpus/manifest-v1.json`, verify the generated-code handoffs for those identities, record the retired `syntax backend source-program type-checker execution preservation gate` as historical metadata only, record the completed `syntax backend Go real-source source-program validity corpus gate`, record the completed `syntax backend Go real-source generated-codec corpus round-trip gate`, and name the current handoff class as `syntax backend Go real-source generated-codec corpus completion handoff gate`. |

Completed gate marker inventory retained for audit compatibility:
`syntax backend input validation diagnostics contract fixture`, `syntax
backend input validation diagnostics handoff gate`, `syntax backend input
validation negative fixture gate`, `syntax backend input validation negative
fixture handoff gate`, `syntax backend parser-backed input validation negative
fixture gate`, `syntax backend parser-backed validation hook sidecar evidence
gate`, `syntax backend generated artifact reproducibility replay gate`,
`syntax backend production TctCompiler routing gate`, `syntax backend
JSON-backend parity routing gate`, `syntax backend target-agnostic routing
boundary gate`, `syntax backend reproducibility routing gate`,
`src/syntax_reproducibility_routing.rs`,
`tests/fixtures/syntax_backend/reproducibility-routing-gate-v1.json`, `syntax
backend audit coverage routing gate`, `syntax backend stable promotion review
gate`, `src/syntax_audit_coverage_routing.rs`,
`tests/fixtures/syntax_backend/audit-coverage-routing-gate-v1.json`,
`src/syntax_stable_promotion_review.rs`,
`tests/fixtures/syntax_backend/stable-promotion-review-gate-v1.json`,
`src/syntax_production_route.rs`,
`tests/fixtures/syntax_backend/production-route-implementation-gate-v1.json`,
`syntax backend production route implementation gate`, `syntax backend Go
local assignment route tranche`, `syntax backend Go escaped local assignment
route tranche`, `syntax backend Go short variable declaration route tranche`,
`syntax backend Go typed local declaration route tranche`, `syntax backend Go
zero-value typed local declaration route tranche`, `syntax backend Go integer
typed local declaration route tranche`, `syntax backend Go untyped integer
local declaration route tranche`, `syntax backend Go integer local assignment
route tranche`, `syntax backend Go integer short variable declaration route
tranche`, `syntax backend Go integer return literal route tranche`, `syntax
backend Go string return literal route tranche`, `syntax backend Go boolean
return identifier route tranche`, `syntax backend Go nil return identifier
route tranche`, `syntax backend Go parenthesized integer return expression
route tranche`, `syntax backend Go unary integer return expression route
tranche`, `syntax backend Go binary integer return expression route tranche`,
`syntax backend Go call return expression route tranche`, `syntax backend Go
argument call return expression route tranche`, `syntax backend route setup
normalization tranche`, `syntax backend C# architecture-parity route
tranche`, `syntax backend C# local string return literal route tranche`,
`syntax backend C# integer return literal route tranche`, `syntax backend C#
boolean return literal route tranche`, `syntax backend C#
null return literal route tranche`, `syntax backend C#
prefix unary integer return route tranche`, `syntax backend C#
parenthesized integer return route tranche`, `syntax backend C#
local typed declaration route tranche`, `syntax backend C# implicit local
declaration route tranche`, `syntax backend C# integer implicit local
declaration route tranche`, `syntax backend C# local assignment route
tranche`, `syntax backend C# local if/else route tranche`, `syntax backend C#
condition-only for route tranche`, `syntax backend C# single-case switch route tranche`,
`syntax backend C# local foreach string route tranche`, `syntax backend C# two-parameter method route tranche`, `syntax backend C# property plus method route tranche`, `syntax backend C#
escaped local assignment route tranche`, `syntax
backend C# representative real-source corpus selection gate`,
`scripts/testing/test-csharp-real-source-corpus.sh`,
`corpora/csharp/dotnet-samples-mini/manifest-v1.json`,
`csharp-dotnet-samples-mini-v1`,
`tct_csharp_real_source_corpus_selection`,
`scripts/analysis/csharp_real_source_metadata_gaps.py`,
`scripts/testing/test-csharp-real-source-metadata-gaps.sh`,
`csharp_block_scoped_namespace_syntax_metadata`,
`tests/fixtures/csharp_syntax_sources/block_scoped_namespace/source.cs`,
`tests/fixtures/csharp_syntax_metadata/block_scoped_namespace/syntax-metadata-v0.json`,
`csharp_block_scoped_namespace_syntax_v0`, `syntax backend C# block-scoped
namespace route tranche`, `csharp_using_directive_syntax_metadata`,
`tests/fixtures/csharp_syntax_sources/using_directive/source.cs`,
`tests/fixtures/csharp_syntax_metadata/using_directive/syntax-metadata-v0.json`,
`csharp_using_directive_syntax_v0`, `syntax backend C# using-directive route
tranche`, `csharp_struct_declaration_syntax_metadata`,
`tests/fixtures/csharp_syntax_sources/struct_declaration/source.cs`,
`tests/fixtures/csharp_syntax_metadata/struct_declaration/syntax-metadata-v0.json`,
`csharp_struct_declaration_syntax_v0`, `syntax backend C# struct declaration
route tranche`, `csharp_enum_declaration_syntax_metadata`,
`tests/fixtures/csharp_syntax_sources/enum_declaration/source.cs`,
`tests/fixtures/csharp_syntax_metadata/enum_declaration/syntax-metadata-v0.json`,
`csharp_enum_declaration_syntax_v0`, `syntax backend C# enum declaration route
tranche`, `csharp_event_declaration_syntax_metadata`,
`tests/fixtures/csharp_syntax_sources/event_declaration/source.cs`,
`tests/fixtures/csharp_syntax_metadata/event_declaration/syntax-metadata-v0.json`,
`csharp_event_declaration_syntax_v0`, `syntax backend C# event declaration
route tranche`, `csharp_yield_statement_syntax_metadata`,
`tests/fixtures/csharp_syntax_sources/yield_statement/source.cs`,
`tests/fixtures/csharp_syntax_metadata/yield_statement/syntax-metadata-v0.json`,
`csharp_yield_statement_syntax_v0`, `syntax backend C# yield statement route
tranche`, `csharp_attribute_syntax_metadata`,
`tests/fixtures/csharp_syntax_sources/attribute_syntax/source.cs`,
`tests/fixtures/csharp_syntax_metadata/attribute_syntax/syntax-metadata-v0.json`,
`csharp_attribute_syntax_v0`, `syntax backend C# attribute route tranche`,
`csharp_generic_name_syntax_metadata`,
`tests/fixtures/csharp_syntax_sources/generic_name/source.cs`,
`tests/fixtures/csharp_syntax_metadata/generic_name/syntax-metadata-v0.json`,
`csharp_generic_name_syntax_v0`, `syntax backend C# generic name route
tranche`, `csharp_qualified_name_syntax_metadata`,
`tests/fixtures/csharp_syntax_sources/qualified_name/source.cs`,
`tests/fixtures/csharp_syntax_metadata/qualified_name/syntax-metadata-v0.json`,
`csharp_qualified_name_syntax_v0`, `syntax backend C# qualified name route
tranche`, `csharp_constructor_declaration_syntax_metadata`,
`tests/fixtures/csharp_syntax_sources/constructor_declaration/source.cs`,
`tests/fixtures/csharp_syntax_metadata/constructor_declaration/syntax-metadata-v0.json`,
`csharp_constructor_declaration_syntax_v0`, `syntax backend C# constructor
declaration route tranche`, `csharp_while_postfix_syntax_metadata`,
`tests/fixtures/csharp_syntax_sources/while_postfix_loop/source.cs`,
`tests/fixtures/csharp_syntax_metadata/while_postfix_loop/syntax-metadata-v0.json`,
`csharp_while_postfix_loop_syntax_v0`, `syntax backend C# while/postfix loop
route tranche`, `csharp_interpolation_syntax_metadata`,
`tests/fixtures/csharp_syntax_sources/interpolated_string/source.cs`,
`tests/fixtures/csharp_syntax_metadata/interpolated_string/syntax-metadata-v0.json`,
`csharp_interpolated_string_syntax_v0`, `syntax backend C# interpolated string
route tranche`, `csharp_member_access_expression_syntax_metadata`,
`tests/fixtures/csharp_syntax_sources/member_access_expression/source.cs`,
`tests/fixtures/csharp_syntax_metadata/member_access_expression/syntax-metadata-v0.json`,
`csharp_member_access_expression_syntax_v0`, `syntax backend C# member access
expression route tranche`, `csharp_field_declaration_syntax_metadata`,
`tests/fixtures/csharp_syntax_sources/field_declaration/source.cs`,
`tests/fixtures/csharp_syntax_metadata/field_declaration/syntax-metadata-v0.json`,
`csharp_field_declaration_syntax_v0`, `syntax backend C# field declaration
route tranche`, `csharp_base_list_syntax_metadata`,
`tests/fixtures/csharp_syntax_sources/base_list/source.cs`,
`tests/fixtures/csharp_syntax_metadata/base_list/syntax-metadata-v0.json`,
`csharp_base_list_syntax_v0`, `syntax backend C# base list route tranche`,
`csharp_conditional_access_syntax_metadata`,
`tests/fixtures/csharp_syntax_sources/conditional_access/source.cs`,
`tests/fixtures/csharp_syntax_metadata/conditional_access/syntax-metadata-v0.json`,
`csharp_conditional_access_syntax_v0`, `syntax backend C# conditional access
route tranche`, `csharp_array_rank_object_creation_syntax_metadata`,
`tests/fixtures/csharp_syntax_sources/array_rank_object_creation/source.cs`,
`tests/fixtures/csharp_syntax_metadata/array_rank_object_creation/syntax-metadata-v0.json`,
`csharp_array_rank_object_creation_syntax_v0`, `syntax backend C#
array-rank/object-creation route tranche`,
`csharp_this_expression_syntax_metadata`,
`tests/fixtures/csharp_syntax_sources/this_expression/source.cs`,
`tests/fixtures/csharp_syntax_metadata/this_expression/syntax-metadata-v0.json`,
`csharp_this_expression_syntax_v0`, `syntax backend C# this-expression route
tranche`, `csharp_pattern_expression_syntax_metadata`,
`tests/fixtures/csharp_syntax_sources/pattern_expression/source.cs`,
`tests/fixtures/csharp_syntax_metadata/pattern_expression/syntax-metadata-v0.json`,
`csharp_pattern_expression_syntax_v0`, `syntax backend C# pattern-expression
route tranche`,
`csharp_arrow_expression_clause_syntax_metadata`,
`tests/fixtures/csharp_syntax_sources/arrow_expression_clause/source.cs`,
`tests/fixtures/csharp_syntax_metadata/arrow_expression_clause/syntax-metadata-v0.json`,
`csharp_arrow_expression_clause_syntax_v0`, `syntax backend C#
arrow-expression-clause route tranche`, `syntax backend
parser-backed source corpus evidence tranche`, `syntax backend
parser-backed source corpus report producer tranche`, `syntax backend
parser-backed source corpus report validator tranche`, `syntax backend
declarative route manifest tranche`, `syntax backend route manifest loader
tranche`, `syntax backend projector-driven metadata route setup tranche`,
`src/syntax_route_manifest.rs`,
`tests/fixtures/syntax_backend/production-route-manifest-v1.json`,
`checked_syntax_production_route_manifest_constructs`,
`src/bin/tct-syntax-source-corpus-evidence.rs`,
`src/syntax_source_corpus_evidence.rs`,
`tests/fixtures/syntax_backend/source-corpus-evidence-v1.json`,
`SYNTAX_PRODUCTION_ROUTE_CONSTRUCTS`, `src/syntax_json_backend_parity.rs`,
`tests/fixtures/syntax_backend/json-backend-parity-routing-gate-v1.json`,
`tests/fixtures/syntax_backend/validation-hook-sidecar-evidence-v1.json`,
`tests/fixtures/syntax_backend/input-validation-negative-fixtures-v1.json`,
and `tests/fixtures/syntax_backend/generated-artifact-replay-v1.json`.

Audit exact marker strings:

- syntax backend input validation diagnostics contract fixture
- syntax backend input validation diagnostics handoff gate
- syntax backend input validation negative fixture gate
- syntax backend input validation negative fixture handoff gate
- syntax backend parser-backed input validation negative fixture gate
- syntax backend parser-backed validation hook sidecar evidence gate
- syntax backend generated artifact reproducibility replay gate
- syntax backend production TctCompiler routing gate
- syntax backend JSON-backend parity routing gate
- syntax backend target-agnostic routing boundary gate
- syntax backend reproducibility routing gate
- src/syntax_reproducibility_routing.rs
- tests/fixtures/syntax_backend/reproducibility-routing-gate-v1.json
- src/syntax_validation_hooks.rs
- tests/fixtures/syntax_backend/validation-hook-contract-v1.json
- src/syntax_reproducibility_replay.rs
- src/syntax_source_program_handoff.rs
- tests/fixtures/syntax_backend/source-program-claim-handoff-v1.json
- syntax backend audit coverage routing gate
- syntax backend stable promotion review gate
- src/syntax_audit_coverage_routing.rs
- tests/fixtures/syntax_backend/audit-coverage-routing-gate-v1.json
- src/syntax_stable_promotion_review.rs
- tests/fixtures/syntax_backend/stable-promotion-review-gate-v1.json
- src/syntax_production_route.rs
- tests/fixtures/syntax_backend/production-route-implementation-gate-v1.json
- syntax backend production route implementation gate
- syntax backend production generated-code renderer API
- syntax backend Go local declaration route tranche
- syntax backend Go local assignment route tranche
- syntax backend Go escaped local assignment route tranche
- syntax backend Go short variable declaration route tranche
- syntax backend Go typed local declaration route tranche
- syntax backend Go zero-value typed local declaration route tranche
- syntax backend Go integer typed local declaration route tranche
- syntax backend Go untyped integer local declaration route tranche
- syntax backend Go integer local assignment route tranche
- syntax backend Go integer short variable declaration route tranche
- syntax backend Go integer return literal route tranche
- syntax backend Go string return literal route tranche
- syntax backend Go boolean return identifier route tranche
- syntax backend Go nil return identifier route tranche
- syntax backend Go parenthesized integer return expression route tranche
- syntax backend Go unary integer return expression route tranche
- syntax backend Go binary integer return expression route tranche
- syntax backend Go call return expression route tranche
- syntax backend Go argument call return expression route tranche
- syntax backend route setup normalization tranche
- syntax backend C# architecture-parity route tranche
- syntax backend C# local string return literal route tranche
- syntax backend C# integer return literal route tranche
- syntax backend C# boolean return literal route tranche
- syntax backend C# null return literal route tranche
- syntax backend C# nullable return type route tranche
- syntax backend C# prefix unary integer return route tranche
- syntax backend C# local typed declaration route tranche
- syntax backend C# implicit local declaration route tranche
- syntax backend C# integer implicit local declaration route tranche
- syntax backend C# local assignment route tranche
- syntax backend C# local if/else route tranche
- syntax backend C# condition-only for route tranche
- syntax backend C# single-case switch route tranche
- syntax backend C# local foreach string route tranche
- syntax backend C# two-parameter method route tranche
- syntax backend C# property plus method route tranche
- syntax backend C# escaped local assignment route tranche
- syntax backend C# representative real-source corpus selection gate
- scripts/testing/test-csharp-real-source-corpus.sh
- corpora/csharp/dotnet-samples-mini/manifest-v1.json
- csharp-dotnet-samples-mini-v1
- tct_csharp_real_source_corpus_selection
- scripts/analysis/csharp_real_source_metadata_gaps.py
- scripts/testing/test-csharp-real-source-metadata-gaps.sh
- csharp_block_scoped_namespace_syntax_metadata
- tests/fixtures/csharp_syntax_sources/block_scoped_namespace/source.cs
- tests/fixtures/csharp_syntax_metadata/block_scoped_namespace/syntax-metadata-v0.json
- csharp_block_scoped_namespace_syntax_v0
- syntax backend C# block-scoped namespace route tranche
- csharp_using_directive_syntax_metadata
- tests/fixtures/csharp_syntax_sources/using_directive/source.cs
- tests/fixtures/csharp_syntax_metadata/using_directive/syntax-metadata-v0.json
- csharp_using_directive_syntax_v0
- syntax backend C# using-directive route tranche
- csharp_struct_declaration_syntax_metadata
- tests/fixtures/csharp_syntax_sources/struct_declaration/source.cs
- tests/fixtures/csharp_syntax_metadata/struct_declaration/syntax-metadata-v0.json
- csharp_struct_declaration_syntax_v0
- syntax backend C# struct declaration route tranche
- csharp_enum_declaration_syntax_metadata
- tests/fixtures/csharp_syntax_sources/enum_declaration/source.cs
- tests/fixtures/csharp_syntax_metadata/enum_declaration/syntax-metadata-v0.json
- csharp_enum_declaration_syntax_v0
- syntax backend C# enum declaration route tranche
- csharp_event_declaration_syntax_metadata
- tests/fixtures/csharp_syntax_sources/event_declaration/source.cs
- tests/fixtures/csharp_syntax_metadata/event_declaration/syntax-metadata-v0.json
- csharp_event_declaration_syntax_v0
- syntax backend C# event declaration route tranche
- csharp_yield_statement_syntax_metadata
- tests/fixtures/csharp_syntax_sources/yield_statement/source.cs
- tests/fixtures/csharp_syntax_metadata/yield_statement/syntax-metadata-v0.json
- csharp_yield_statement_syntax_v0
- syntax backend C# yield statement route tranche
- csharp_attribute_syntax_metadata
- tests/fixtures/csharp_syntax_sources/attribute_syntax/source.cs
- tests/fixtures/csharp_syntax_metadata/attribute_syntax/syntax-metadata-v0.json
- csharp_attribute_syntax_v0
- syntax backend C# attribute route tranche
- csharp_generic_name_syntax_metadata
- tests/fixtures/csharp_syntax_sources/generic_name/source.cs
- tests/fixtures/csharp_syntax_metadata/generic_name/syntax-metadata-v0.json
- csharp_generic_name_syntax_v0
- syntax backend C# generic name route tranche
- csharp_qualified_name_syntax_metadata
- tests/fixtures/csharp_syntax_sources/qualified_name/source.cs
- tests/fixtures/csharp_syntax_metadata/qualified_name/syntax-metadata-v0.json
- csharp_qualified_name_syntax_v0
- syntax backend C# qualified name route tranche
- csharp_constructor_declaration_syntax_metadata
- tests/fixtures/csharp_syntax_sources/constructor_declaration/source.cs
- tests/fixtures/csharp_syntax_metadata/constructor_declaration/syntax-metadata-v0.json
- csharp_constructor_declaration_syntax_v0
- syntax backend C# constructor declaration route tranche
- csharp_while_postfix_syntax_metadata
- tests/fixtures/csharp_syntax_sources/while_postfix_loop/source.cs
- tests/fixtures/csharp_syntax_metadata/while_postfix_loop/syntax-metadata-v0.json
- csharp_while_postfix_loop_syntax_v0
- syntax backend C# while/postfix loop route tranche
- csharp_interpolation_syntax_metadata
- tests/fixtures/csharp_syntax_sources/interpolated_string/source.cs
- tests/fixtures/csharp_syntax_metadata/interpolated_string/syntax-metadata-v0.json
- csharp_interpolated_string_syntax_v0
- syntax backend C# interpolated string route tranche
- csharp_member_access_expression_syntax_metadata
- tests/fixtures/csharp_syntax_sources/member_access_expression/source.cs
- tests/fixtures/csharp_syntax_metadata/member_access_expression/syntax-metadata-v0.json
- csharp_member_access_expression_syntax_v0
- syntax backend C# member access expression route tranche
- csharp_field_declaration_syntax_metadata
- tests/fixtures/csharp_syntax_sources/field_declaration/source.cs
- tests/fixtures/csharp_syntax_metadata/field_declaration/syntax-metadata-v0.json
- csharp_field_declaration_syntax_v0
- syntax backend C# field declaration route tranche
- csharp_base_list_syntax_metadata
- tests/fixtures/csharp_syntax_sources/base_list/source.cs
- tests/fixtures/csharp_syntax_metadata/base_list/syntax-metadata-v0.json
- csharp_base_list_syntax_v0
- syntax backend C# base list route tranche
- csharp_conditional_access_syntax_metadata
- tests/fixtures/csharp_syntax_sources/conditional_access/source.cs
- tests/fixtures/csharp_syntax_metadata/conditional_access/syntax-metadata-v0.json
- csharp_conditional_access_syntax_v0
- syntax backend C# conditional access route tranche
- csharp_array_rank_object_creation_syntax_metadata
- tests/fixtures/csharp_syntax_sources/array_rank_object_creation/source.cs
- tests/fixtures/csharp_syntax_metadata/array_rank_object_creation/syntax-metadata-v0.json
- csharp_array_rank_object_creation_syntax_v0
- syntax backend C# array-rank/object-creation route tranche
- csharp_this_expression_syntax_metadata
- tests/fixtures/csharp_syntax_sources/this_expression/source.cs
- tests/fixtures/csharp_syntax_metadata/this_expression/syntax-metadata-v0.json
- csharp_this_expression_syntax_v0
- syntax backend C# this-expression route tranche
- csharp_pattern_expression_syntax_metadata
- tests/fixtures/csharp_syntax_sources/pattern_expression/source.cs
- tests/fixtures/csharp_syntax_metadata/pattern_expression/syntax-metadata-v0.json
- csharp_pattern_expression_syntax_v0
- syntax backend C# pattern-expression route tranche
- csharp_arrow_expression_clause_syntax_metadata
- tests/fixtures/csharp_syntax_sources/arrow_expression_clause/source.cs
- tests/fixtures/csharp_syntax_metadata/arrow_expression_clause/syntax-metadata-v0.json
- csharp_arrow_expression_clause_syntax_v0
- syntax backend C# arrow-expression-clause route tranche
- syntax backend parser-backed source corpus evidence tranche
- syntax backend parser-backed source corpus report producer tranche
- syntax backend parser-backed source corpus report validator tranche
- syntax backend declarative route manifest tranche
- syntax backend route manifest loader tranche
- syntax backend projector-driven metadata route setup tranche
- src/syntax_route_manifest.rs
- tests/fixtures/syntax_backend/production-route-manifest-v1.json
- checked_syntax_production_route_manifest_constructs
- src/bin/tct-syntax-source-corpus-evidence.rs
- src/syntax_source_corpus_evidence.rs
- tests/fixtures/syntax_backend/source-corpus-evidence-v1.json
- SYNTAX_PRODUCTION_ROUTE_CONSTRUCTS
- src/syntax_json_backend_parity.rs
- tests/fixtures/syntax_backend/json-backend-parity-routing-gate-v1.json
- tests/fixtures/syntax_backend/validation-hook-sidecar-evidence-v1.json
- tests/fixtures/syntax_backend/input-validation-negative-fixtures-v1.json
- tests/fixtures/syntax_backend/generated-artifact-replay-v1.json
- syntax backend source-program external validation execution report gate
- syntax backend source-program external validation execution preservation gate
- syntax backend source-program type-checker control feasibility gate (retired; historical inventory only)
- syntax backend source-program type-checker execution report gate (retired; historical inventory only)
- syntax backend JSON-backend-grade validation matrix route ownership gate
- src/syntax_go_real_source_generated_codec_corpus.rs
- tests/fixtures/syntax_backend/go-real-source-generated-codec-corpus-v1.json
- tct_syntax_backend_go_real_source_generated_codec_corpus_gate
- syntax backend Go real-source generated-codec corpus round-trip gate
- src/syntax_go_real_source_generated_codec_completion_handoff.rs
- tests/fixtures/syntax_backend/go-real-source-generated-codec-completion-handoff-v1.json
- tct_syntax_backend_go_real_source_generated_codec_completion_handoff_gate
- syntax backend Go real-source generated-codec corpus completion handoff gate
- src/syntax_stable_metadata_promotion.rs
- tests/fixtures/syntax_backend/stable-metadata-promotion-implementation-v1.json
- tct_syntax_backend_stable_metadata_promotion_implementation_gate
- syntax backend stable metadata promotion implementation gate
- python3 scripts/testing/run-syntax-validation-matrix.py --output target/tct-syntax-validation-matrix/execution-report-v1.json
- python3 scripts/testing/run-syntax-validation-matrix.py --validate-report target/tct-syntax-validation-matrix/execution-report-v1.json
- syntax backend validation matrix execution report helper gate
- src/syntax_validation_matrix_preservation.rs
- tests/fixtures/syntax_backend/validation-matrix-execution-preservation-v1.json
- tct_syntax_backend_validation_matrix_execution_preservation_gate
- syntax backend validation matrix execution evidence preservation gate
- syntax backend stable metadata promotion implementation gate
- release_depth_execution_evidence_preservation_policy

The final stable-promotion decision gate is now checked through
`src/syntax_final_promotion_decision.rs` and
`tests/fixtures/syntax_backend/final-stable-promotion-decision-v1.json`. It
records `tct_syntax_backend_final_stable_promotion_decision_gate`,
`stable_promotion_ready_decision`, and
`ready_for_stable_metadata_promotion_implementation`. The stable metadata
implementation is now complete for
`feature_limited_parser_backed_syntax_backend`; the current source-language
handoff refinement is type-checker execution preservation consumption of the
execution source input summary.

## First Implementation Hook

The manifest-backed syntax generated-code gate handoff extends
`tests/syntax_generated_codec_gate_test.rs` and its volatile reproducibility
report under `target/` so generated-code metadata consumes
`tests/fixtures/syntax_structural_model/boundary-manifest-v1.json` and records
the manifest identity for toy, C#, and Go targets before any generated codec is
treated as syntax backend evidence.

The audit verifies that the generated-code gate references the manifest
artifact kind, version, manifest SHA-256, expected targets, and sidecar-only
boundary claim. This is the first concrete stable-gate implementation hook
because generated code already exercises compilation, semantic round trips,
strict/recovery decode behavior, consumed-token accounting, token/vocabulary
invariants, BPE evidence, and reproducibility metadata for all three targets.

## Promotion Boundaries

Do not route the manifest, `GrammarIR`, or `StructuralModel` through
`TctCompiler` from this plan. Do not treat Roslyn, Go parser, renderer,
formatter, compiler, or functional-validator evidence as stable syntax backend
gates until a later roadmap item explicitly promotes one of those sidecar
checks.

The input-validation diagnostics contract fixture is now the completed
implementation slice: `syntax backend input validation diagnostics contract fixture`.
The generated-code diagnostics handoff is now the completed implementation
slice: `syntax backend input validation diagnostics handoff gate`. The
input-validation negative fixture gate is now the completed implementation
slice: `syntax backend input validation negative fixture gate`. The
generated-code negative fixture handoff is now the completed implementation
slice: `syntax backend input validation negative fixture handoff gate`. The
parser-backed negative fixture coverage is now the completed implementation
slice: `syntax backend parser-backed input validation negative fixture gate`.
The validation-hook sidecar evidence fixture is now the completed
implementation slice: `syntax backend parser-backed validation hook sidecar evidence gate`.
The production `TctCompiler` routing gate is now the completed implementation
slice: `syntax backend production TctCompiler routing gate`. The JSON-backend
parity routing gate is now the completed implementation slice: `syntax backend
JSON-backend parity routing gate`. The target-agnostic routing boundary gate is
now the completed implementation slice: `syntax backend target-agnostic routing
boundary gate`. The strict/recovery decode routing gate is now the completed
implementation slice: `syntax backend strict/recovery decode routing gate`. The
consumed-token accounting routing gate is now the completed implementation
slice: `syntax backend consumed-token accounting routing gate`. The next
token/vocabulary/BPE routing gate is now the completed implementation slice:
`syntax backend token/vocabulary/BPE routing gate`. The reproducibility routing
gate is now the completed implementation slice: `syntax backend
reproducibility routing gate`. The audit coverage routing gate is now the
completed implementation slice: `syntax backend audit coverage routing gate`.
The stable-promotion review gate is now the completed implementation slice:
`syntax backend stable promotion review gate`. The production route
implementation gate is now the completed implementation slice: `syntax backend
production route implementation gate`. The route-owned validation matrix is now
the completed implementation slice: `syntax backend JSON-backend-grade
validation matrix route ownership gate`. The validation matrix execution report
helper is now the completed implementation slice: `syntax backend validation
matrix execution report helper gate`. The validation matrix execution
preservation gate is now the completed implementation slice: `syntax backend
validation matrix execution evidence preservation gate`. The final
stable-promotion decision gate is now the completed implementation slice:
`syntax backend final stable-promotion review gate`. The Go short variable declaration route
tranche is now the completed implementation slice:
`syntax backend Go short variable declaration route tranche`. The Go typed
local declaration route tranche is now the completed implementation slice:
`syntax backend Go typed local declaration route tranche`. The Go zero-value
typed local declaration route tranche is now the completed implementation
slice: `syntax backend Go zero-value typed local declaration route tranche`.
The Go integer typed local declaration route tranche is now the completed
implementation slice: `syntax backend Go integer typed local declaration route
tranche`. The Go untyped integer local declaration route tranche is now the
completed implementation slice: `syntax backend Go untyped integer local
declaration route tranche`. The Go integer local assignment route tranche is
now the completed implementation slice: `syntax backend Go integer local
assignment route tranche`. The Go integer short variable declaration route
tranche is now the completed implementation slice: `syntax backend Go integer
short variable declaration route tranche`. The Go integer return literal route
tranche is now the completed implementation slice: `syntax backend Go integer
return literal route tranche`. The Go string return literal route tranche is
now the completed implementation slice:
`syntax backend Go string return literal route tranche`. The Go boolean return
identifier route tranche is now the completed implementation slice:
`syntax backend Go boolean return identifier route tranche`. The Go nil return
identifier route tranche is now the completed implementation slice:
`syntax backend Go nil return identifier route tranche`. The Go parenthesized
integer return expression route tranche is now the completed implementation
slice: `syntax backend Go parenthesized integer return expression route tranche`.
The Go unary integer return expression route tranche is now the completed
implementation slice:
`syntax backend Go unary integer return expression route tranche`.
The Go binary integer return expression route tranche is now the completed
implementation slice:
`syntax backend Go binary integer return expression route tranche`.
The Go call return expression route tranche is now the completed
implementation slice:
`syntax backend Go call return expression route tranche`.
The Go argument call return expression route tranche is now the completed
implementation slice:
`syntax backend Go argument call return expression route tranche`, with the
parser-backed `len("world")` fixture preserving `CallExpr.Args` through the
generated-codec route.
The Go control-flow, package type declaration, method declaration, struct type,
composite literal, function literal, key/value composite literal, array
composite literal, index expression, slice expression, star expression,
package-level var declaration, local compound assignment, local
increment/decrement, and local branch statement route tranches are now completed
implementation slices:
`syntax backend Go if/else route tranche`, `syntax backend Go condition-only
for route tranche`, `syntax backend Go single-case switch route tranche`,
`syntax backend Go string range route tranche`, `syntax backend Go package type
declaration route tranche`, `syntax backend Go method declaration route
tranche`, `syntax backend Go struct type route tranche`, `syntax backend Go
composite literal route tranche`, `syntax backend Go function literal route
tranche`, `syntax backend Go key/value composite literal route tranche`,
`syntax backend Go array composite literal route tranche`, `syntax backend Go
index expression route tranche`, `syntax backend Go slice expression route
tranche`, `syntax backend Go star expression route tranche`,
`syntax backend Go package var declaration route tranche`,
`syntax backend Go ValueSpec.Comment route tranche`,
`syntax backend Go ImportSpec.Name route tranche`,
`syntax backend Go TypeSpec.Assign route tranche`,
`syntax backend Go local compound assignment route tranche`,
`syntax backend Go local increment/decrement route tranche`, and
`syntax backend Go local branch statement route tranche`.
The completed function-literal class is
`syntax backend Go function literal route tranche`.
The completed composite-literal class remains
`syntax backend Go composite literal route tranche`.
The completed key/value composite-literal class is
`syntax backend Go key/value composite literal route tranche`.
The completed array composite-literal class is
`syntax backend Go array composite literal route tranche`.
The completed C# collection-initializer class is
`syntax backend C# collection initializer route tranche`.
The completed C# range element-access class is
`syntax backend C# range element access route tranche`.
The completed index-expression class is
`syntax backend Go index expression route tranche`.
The completed slice-expression class is
`syntax backend Go slice expression route tranche`.
The completed star-expression class is
`syntax backend Go star expression route tranche`.
The completed package-level var declaration class is
`syntax backend Go package var declaration route tranche`.
The completed import spec name class is
`syntax backend Go ImportSpec.Name route tranche`.
The completed type spec assign class is
`syntax backend Go TypeSpec.Assign route tranche`.
The completed local compound assignment class is
`syntax backend Go local compound assignment route tranche`.
The completed local increment/decrement class is
`syntax backend Go local increment/decrement route tranche`.
The completed local branch statement class is
`syntax backend Go local branch statement route tranche`.
The route setup normalization tranche is now the completed implementation
slice: `syntax backend route setup normalization tranche`. The C#
architecture-parity, local string return literal, integer return literal, prefix unary integer return, local typed declaration,
implicit local declaration, integer implicit local declaration, local assignment,
local if/else, condition-only for, single-case switch, local foreach string, two-parameter method, property plus method, and escaped local assignment route tranches are now completed implementation slices:
`syntax backend C# architecture-parity route tranche`,
`syntax backend C# local string return literal route tranche`,
`syntax backend C# integer return literal route tranche`,
`syntax backend C# boolean return literal route tranche`,
`syntax backend C# prefix unary integer return route tranche`,
`syntax backend C# local typed declaration route tranche`,
`syntax backend C# implicit local declaration route tranche`,
`syntax backend C# integer implicit local declaration route tranche`,
`syntax backend C# local assignment route tranche`,
`syntax backend C# local if/else route tranche`,
`syntax backend C# condition-only for route tranche`,
`syntax backend C# single-case switch route tranche`,
`syntax backend C# local foreach string route tranche`,
`syntax backend C# two-parameter method route tranche`,
`syntax backend C# property plus method route tranche`,
`syntax backend C# nullable local declaration route tranche`,
`syntax backend C# nullable property declaration route tranche`,
`syntax backend C# nullable parameter declaration route tranche`, and
`syntax backend C# escaped local assignment route tranche`. The parser-backed
source-corpus evidence tranche is now the completed implementation slice:
`syntax backend parser-backed source corpus evidence tranche`. The source-corpus
report producer and validator are now completed implementation slices:
`syntax backend parser-backed source corpus report producer tranche` and
`syntax backend parser-backed source corpus report validator tranche`. The
checked declarative route manifest is now a completed implementation slice:
`syntax backend declarative route manifest tranche`. The manifest-backed route
setup loader is now a completed implementation slice: `syntax backend route
manifest loader tranche`. The projector metadata setup is now a completed
implementation slice: `syntax backend projector-driven metadata route setup
tranche`. The completed Go minimal function production route entry is now
recorded as `syntax backend Go minimal function production route tranche`. The
checked Go real-source corpus helper is now recorded as
`syntax backend Go real-source corpus sidecar tranche` and is ready after the
local float return literal route covers the pinned `basic_lit_other` gap. The
stable metadata promotion gate over the route-owned syntax backend is now
complete. The checked source-program handoff profile now consumes the stable
metadata promotion implementation and Go real-source generated-codec completion
handoff as explicit promotion-source inputs, including retired type-checker execution
and feasibility report identities, retired type-checker preservation report
contract stable fields, pass-envelope and sidecar-only policy, corpus
manifest/report identity, generated-codec report commands, and parser-backed
generated-codec round-trip records, is recorded as `syntax backend source-program claim handoff gate`, the handoff report producer is recorded as
`syntax backend source-program handoff report producer gate`,
the external validation controls gate now consumes those promotion-source
inputs and is recorded as `syntax backend source-program external validation
control evidence gate`, the external validation execution report helper carries
the handoff promotion-source inputs and is recorded as `syntax backend
source-program external validation execution report gate`, uses
`src/bin/tct-syntax-source-program-external-validation.rs` to write
`tct_syntax_backend_source_program_external_validation_execution_report`
artifacts from `src/syntax_source_program_external_validation.rs` and
`tests/fixtures/syntax_backend/source-program-external-validation-controls-v1.json`,
the external validation execution preservation gate is recorded as `syntax
backend source-program external validation execution preservation gate` through
`src/syntax_source_program_external_validation_preservation.rs` and
`tests/fixtures/syntax_backend/source-program-external-validation-execution-preservation-v1.json`,
records the execution report's source handoff promotion-source fields, the C#
generated-codec completion handoff source input, and the
`tct_syntax_backend_source_program_external_validation_execution_preservation_gate`
artifact kind,
the source-program type-checker feasibility/execution evidence family was
retired (2026-06). Retained handoff and Go real-source corpus gates pin its
identity pins and stable field names as historical fixture metadata only; no
live typecheck modules or CLI binaries remain. Retired inventory markers kept
for audit compatibility: syntax backend source-program type-checker control
feasibility gate, src/syntax_source_program_typecheck_feasibility.rs,
src/bin/tct-syntax-source-program-typecheck-feasibility.rs,
tct_syntax_backend_source_program_typecheck_feasibility_report, syntax backend
source-program type-checker execution report gate,
src/syntax_source_program_typecheck_execution.rs,
src/bin/tct-syntax-source-program-typecheck-execution.rs,
tct_syntax_backend_source_program_typecheck_execution_report, syntax backend
source-program type-checker execution preservation gate,
src/syntax_source_program_typecheck_execution_preservation.rs,
tests/fixtures/syntax_backend/source-program-typecheck-execution-preservation-v1.json,
tct_syntax_backend_source_program_typecheck_execution_preservation_gate,
source_program_typecheck_execution_report_preservation_policy, and
go_local_call_return_expression_syntax_v0 / recover() historical pass-envelope
pins. The
Go real-source corpus boundary is now preserved by
`src/syntax_go_real_source_program_validity_corpus.rs` and
`tests/fixtures/syntax_backend/go-real-source-program-validity-corpus-v1.json`.
That gate records
`tct_syntax_backend_go_real_source_program_validity_corpus_gate`, the five
parser-backed real-source corpus records, the volatile
`target/tct-analysis/go-syntax-corpus-check/report.json` helper contract, zero
current unsupported corpus records, the retired type-checker execution
preservation family's source handoff promotion-source fields and expanded
blocked-claim contract as historical fixture metadata only, and the completed
`syntax backend Go real-source source-program validity corpus gate`. The
external validation controls gate now consumes the source-program handoff source
input summary, and the external validation execution preservation gate now
preserves the execution report's source handoff source-input fields. The
retired typecheck family is no longer a live refinement step; retained
downstream gates pin its historical fields only. The
generated-codec corpus
gate is now preserved by
`src/syntax_go_real_source_generated_codec_corpus.rs` and
`tests/fixtures/syntax_backend/go-real-source-generated-codec-corpus-v1.json`.
That gate records
`tct_syntax_backend_go_real_source_generated_codec_corpus_gate`, the checked
source-program corpus gate's preserved type-checker handoff fields (historical
metadata from the retired typecheck family), expanded blocked claims, and
sidecar-only no-stable boundary, the checked stdlib-relative corpus identities
`strings/compare.go`, `sort/example_interface_test.go`,
`sort/example_search_test.go`, `strings/search.go`, and `path/match.go`
generated-codec round trips, the volatile
generated-codec report contracts under `target/tct-syntax-real-source-demo/`,
zero blocked generated-codec corpus records, the sidecar-only
`whole_file_generated_codec_claim_for_checked_go_real_source_corpus` claim, and
the completed `syntax backend Go real-source generated-codec corpus round-trip
gate`. The completion handoff
is now preserved by
`src/syntax_go_real_source_generated_codec_completion_handoff.rs` and
`tests/fixtures/syntax_backend/go-real-source-generated-codec-completion-handoff-v1.json`.
That gate records
`tct_syntax_backend_go_real_source_generated_codec_completion_handoff_gate`,
consumes the completed corpus, including its source-program handoff fields,
retired type-checker preservation report contract stable fields, retired
type-checker execution and feasibility report identities, pass-envelope and
sidecar-only policy,
corpus manifest/report identity, generated-codec report
commands, generated-codec no-stable claims, supported record counts, record
identities, and parser-backed generated-codec round-trip records, plus the
final stable-promotion decision, keeps
source-program, whole-file generated-codec, paper, model-quality,
type-correctness, arena IR, generated hot-path, and generated JSON Schema
output claims blocked, and names `syntax backend stable metadata promotion
implementation gate` as the next source-input preservation prerequisite. The stable metadata
promotion implementation is now preserved by
`src/syntax_stable_metadata_promotion.rs` and
`tests/fixtures/syntax_backend/stable-metadata-promotion-implementation-v1.json`.
That gate records
`tct_syntax_backend_stable_metadata_promotion_implementation_gate`, consumes
the Go completion handoff's source-program stable report fields,
type-checker preservation report contract stable fields, source-program blocked
claims, type-checker execution and feasibility report identities, pass-envelope
and sidecar-only policy, corpus manifest/report
identity, generated-codec report commands, source-program and generated-codec
no-stable claims, supported record counts, record identities, and
parser-backed generated-codec round-trip records, plus the final
stable-promotion decision and production route metadata. It promotes only
`stable_route_metadata_promoted` for the feature-limited parser-backed syntax
backend, preserves the JSON Schema backend, leaves source-program and paper
claims blocked, and names `Build Go Source-Record Raw-Token Generator Core` as the next
route-owned state API prerequisite. The checked source-program handoff profile/report now
consumes that stable metadata input summary, including the type-checker
preservation report contract stable fields, without promoting source-program,
paper, model-quality, type-correctness, arena IR, generated hot-path, or
generated JSON Schema output claims. The external-validation controls gate now
consumes the source-program handoff's stable metadata input summary under the
same sidecar-only claim boundary, and the type-checker feasibility report now
consumes the external-validation source handoff summary. The type-checker
execution report now consumes that feasibility source input summary explicitly;
the type-checker execution preservation gate now consumes the execution source
input summary. The Go real-source source-program validity corpus gate now
consumes the type-checker preservation source input summary. The Go real-source
generated-codec corpus gate now consumes the source-program corpus source input
summary. The Go real-source generated-codec completion handoff now consumes the
generated-codec corpus source input summary. The cross-target equivalence gate
now proves zero-argument invocation categories for the matched Go `recover()`
call-return and C# `Format()` invocation-return sidecars, string argument
categories for the matched Go `len("world")` call-return and C# invocation
with-argument return sidecars, plus matched if/else, condition-only loop,
single-case switch, and string-iteration control-flow categories,
callable-wrapper declaration, name, return-type, body-block, and return-literal
categories over the Go/C# integer-return routes, and callable parameter-list
categories for parameter-list edge, parameter binding, parameter name payload,
and parameter type payload over the Go minimal-function and C# two-parameter
method syntax routes, and member-list container categories for repeated
member/declaration list, data/type member declaration, callable member
declaration, member name payloads, and member type payloads over the Go package
type declaration and C# property-plus-method syntax routes, and data-member
field/property categories for data-member container, data-member binding,
member name payload, and member type payload over the Go struct type and C#
property-plus-method syntax routes, and method-member callable categories for
method declaration/container, method name payload, return type payload, body
block, return statement, and return expression edge over the Go method
declaration and C# property-plus-method syntax routes, package/class data
declaration categories for data declaration container, data declaration binding,
data declaration name payload, data declaration type payload, initializer field
profile, companion callable member, and companion return edge over the Go
package var declaration and C# property-plus-method syntax routes, and
collection/composite initializer categories for callable/member container,
collection creation node, element type reference and payload, repeated
initializer element list, and integer element payloads over the Go array
composite literal and C# collection initializer syntax routes, and
index/element-access categories for callable/member container, parameter
array/element type, index target identifier, bracketed/index argument list,
and integer index payload over the Go index expression and C# element-access
syntax routes, slice/range categories for callable/member container, parameter
array/element type, range target identifier, slice/range expression, and
integer lower/start payload over the Go slice expression and C# range
element-access syntax routes, and unary-expression categories for
callable/member container, return body, unary expression edge, unary operator
payload, and integer operand payload over the Go unary integer return
expression and C# prefix-unary integer return syntax routes, and
parenthesized-expression categories for callable/member container, return
body, parenthesized-expression edge, and integer operand payload over the Go
parenthesized integer return expression and C# parenthesized integer return
syntax routes
(`go_local_parenthesized_integer_return_expression_syntax_v0` and
`csharp_local_parenthesized_integer_return_syntax_v0`), while
pinning the C# zero-argument empty argument-list wrapper, C# parameter payloads,
Go file/package/function-type wrappers, Go import wrappers, exact parameter
counts, Go type-spec wrappers, Go struct-field wrappers, C# folded property
binding, C# property initializer, Go package var keyword, Go package value-list
cardinality, Go package initializer literal, C# property initializer string
payload, Go receiver list/name/type binding, Go method return literal, C#
method return-identifier spelling, member ordering, C# namespace/class/member
wrappers, Go composite literal spelling, C# array-creation initializer wrapper,
keyed composite values, C# element-access ArgumentList/ArgumentSyntax wrappers,
C# range argument-list wrappers, Go open-ended high-bound absence, C# bounded
range right operand, Go `UnaryExpr.Op`/`UnaryExpr.X` spelling, C#
`PrefixUnaryExpressionSyntax.OperatorToken`/`Operand` spelling,
Go `ParenExpr.X` spelling, C# `ParenthesizedExpressionSyntax.Expression` spelling,
loop setup/header fields, switch section/label
wrappers, C# break statements, Go range binding absence, C# foreach type/header wrappers, C#
foreach iteration variables, and operator/type tokens as target-specific
evidence. The generic matched Go/C# local construct `StructuralModel` parity
cases are now route-pinned to production route registrations and generated
round-trip markers. The Go/C# boolean return and nil/null return
`StructuralModel` parity gates now compare callable wrapper, return type,
return statement, and payload leaf evidence over the route-backed syntax
metadata while keeping Go boolean identifier spelling, C# boolean literal
spelling, Go nil identifier spelling, and C# null literal spelling
target-specific. The nil/null return categories are pinned over
`go_local_nil_return_identifier_syntax_v0` and
`csharp_local_null_return_literal_syntax_v0`. The C# nullable return type route
`csharp_local_nullable_return_type_syntax_v0` is now covered as
target-specific syntax evidence and pinned in the sidecar-only source-program
typecheck control envelope. The C# nullable local declaration route
`csharp_local_nullable_declaration_syntax_v0` now covers nullable local
declaration syntax with a null initializer and is pinned in the sidecar-only
source-program typecheck control envelope. The C# nullable property declaration
route `csharp_nullable_property_declaration_syntax_v0` now covers member
nullable annotation syntax with a null initializer and is pinned in the
sidecar-only source-program typecheck control envelope. The C# nullable
parameter declaration route `csharp_nullable_parameter_declaration_syntax_v0`
now covers nullable annotation syntax in callable signature context and is
pinned in the sidecar-only source-program typecheck control envelope. The C#
nullable context `StructuralModel` equivalence gate now compares the return,
local declaration, property declaration, and parameter declaration contexts by
asserting each parser-backed owner reference into `NullableTypeSyntax`, the
shared `NullableTypeSyntax.ElementType` reference, and the `QuestionToken`
string payload leaf. The C# fixture route-corpus toolchain evidence
preservation residual (src/syntax_csharp_fixture_route_corpus.rs and
tests/fixtures/syntax_backend/csharp-fixture-route-corpus-preservation-v1.json)
was retired with the arena frontend evidence consumers; current C# real-source
selection uses `scripts/testing/test-csharp-real-source-corpus.sh` and
`corpora/csharp/dotnet-samples-mini/manifest-v1.json` only. The
`syntax backend C# representative real-source corpus selection gate` is now
checked by `scripts/testing/test-csharp-real-source-corpus.sh` and
`corpora/csharp/dotnet-samples-mini/manifest-v1.json` with corpus id
`csharp-dotnet-samples-mini-v1` and artifact kind
`tct_csharp_real_source_corpus_selection`. The parser-backed C# real-source
syntax-metadata gap gate is now checked by
`scripts/analysis/csharp_real_source_metadata_gaps.py` and
`scripts/testing/test-csharp-real-source-metadata-gaps.sh`. The checked
generic-name sidecar route uses
`tests/fixtures/csharp_syntax_sources/generic_name/source.cs` and
`tests/fixtures/csharp_syntax_metadata/generic_name/syntax-metadata-v0.json`
as `csharp_generic_name_syntax_v0`, completing the `syntax backend C# generic
name route tranche`. The checked qualified-name sidecar route uses
`tests/fixtures/csharp_syntax_sources/qualified_name/source.cs` and
`tests/fixtures/csharp_syntax_metadata/qualified_name/syntax-metadata-v0.json`
as `csharp_qualified_name_syntax_v0`, completing the `syntax backend C#
qualified name route tranche`. The checked constructor-declaration sidecar route
uses `tests/fixtures/csharp_syntax_sources/constructor_declaration/source.cs` and
`tests/fixtures/csharp_syntax_metadata/constructor_declaration/syntax-metadata-v0.json`
as `csharp_constructor_declaration_syntax_v0`, completing the `syntax backend
C# constructor declaration route tranche`. The checked while/postfix loop
sidecar route uses `tests/fixtures/csharp_syntax_sources/while_postfix_loop/source.cs`
and
`tests/fixtures/csharp_syntax_metadata/while_postfix_loop/syntax-metadata-v0.json`
as `csharp_while_postfix_loop_syntax_v0`, completing the `syntax backend C#
while/postfix loop route tranche`; the interpolated-string sidecar route uses
`tests/fixtures/csharp_syntax_sources/interpolated_string/source.cs` and
`tests/fixtures/csharp_syntax_metadata/interpolated_string/syntax-metadata-v0.json`
as `csharp_interpolated_string_syntax_v0`, completing the `syntax backend C#
interpolated string route tranche`; the field declaration sidecar route uses
`tests/fixtures/csharp_syntax_sources/field_declaration/source.cs` and
`tests/fixtures/csharp_syntax_metadata/field_declaration/syntax-metadata-v0.json`
as `csharp_field_declaration_syntax_v0`, completing the `syntax backend C#
field declaration route tranche`; the base-list sidecar route uses
`tests/fixtures/csharp_syntax_sources/base_list/source.cs` and
`tests/fixtures/csharp_syntax_metadata/base_list/syntax-metadata-v0.json` as
`csharp_base_list_syntax_v0`, completing the `syntax backend C# base list
route tranche`; the conditional-access sidecar route uses
`tests/fixtures/csharp_syntax_sources/conditional_access/source.cs` and
`tests/fixtures/csharp_syntax_metadata/conditional_access/syntax-metadata-v0.json`
as `csharp_conditional_access_syntax_v0`, completing the `syntax backend C#
conditional access route tranche`; the array-rank/object-creation sidecar
route uses `tests/fixtures/csharp_syntax_sources/array_rank_object_creation/source.cs`
and
`tests/fixtures/csharp_syntax_metadata/array_rank_object_creation/syntax-metadata-v0.json`
as `csharp_array_rank_object_creation_syntax_v0`, completing the `syntax
backend C# array-rank/object-creation route tranche`; the `this` expression
sidecar route uses `tests/fixtures/csharp_syntax_sources/this_expression/source.cs`
and
`tests/fixtures/csharp_syntax_metadata/this_expression/syntax-metadata-v0.json`
as `csharp_this_expression_syntax_v0`, completing the `syntax backend C#
this-expression route tranche`; the pattern-expression sidecar route uses
`tests/fixtures/csharp_syntax_sources/pattern_expression/source.cs` and
`tests/fixtures/csharp_syntax_metadata/pattern_expression/syntax-metadata-v0.json`
as `csharp_pattern_expression_syntax_v0`, completing the `syntax backend C#
pattern-expression route tranche`; the arrow-expression-clause sidecar route
uses `tests/fixtures/csharp_syntax_sources/arrow_expression_clause/source.cs` and
`tests/fixtures/csharp_syntax_metadata/arrow_expression_clause/syntax-metadata-v0.json`
as `csharp_arrow_expression_clause_syntax_v0`, completing the `syntax backend
C# arrow-expression-clause route tranche`; the gap classifier now counts completed
assignment-expression, qualified-name, constructor, while/postfix,
interpolated-string, member-access expression, field declaration, and
base-list/conditional-access/array-rank/object-creation/this-expression/pattern-expression/arrow-expression-clause
route nodes as supported and reports zero remaining C# syntax-metadata gap
classes, while keeping source-program validity, arena IR, generated hot-path,
and stable backend claims blocked.
The source-corpus evidence fixture now consumes the no-gap result as
`csharp_no_gap_metadata_report`. The checked C# real-source generated-codec
feasibility gate consumes that handoff, the C# corpus manifest, and the
production route manifest, and names
`csharp_compilation_unit_source_file_syntax_v0` as the supported whole-source
generated-codec route for `pattern-matching/Shapes.cs`,
`delegates-and-events/Logger.cs`, `events/VersionOne.cs`, and
`iterators/IteratorMethods.cs`, and `serialization/Loan.cs`. The checked C#
generated-codec corpus gate preserves those round-trip reports, the C#
generated-codec completion handoff consumes that corpus, and the source-program
external-validation controls plus execution-preservation gate now carry the C#
completion handoff. The retired type-checker feasibility/execution/preservation family previously
carried the same C# completion handoff beside the Go handoff; those pins survive
only as historical fixture metadata while source-program validity,
type-correctness, paper, model-quality, arena IR, generated-hot-path, and stable
backend claims remain blocked.
Audit marker: syntax backend C# representative real-source corpus selection gate; scripts/testing/test-csharp-real-source-corpus.sh; corpora/csharp/dotnet-samples-mini/manifest-v1.json; csharp-dotnet-samples-mini-v1; tct_csharp_real_source_corpus_selection; scripts/analysis/csharp_real_source_metadata_gaps.py; scripts/testing/test-csharp-real-source-metadata-gaps.sh; csharp_block_scoped_namespace_syntax_metadata; tests/fixtures/csharp_syntax_sources/block_scoped_namespace/source.cs; tests/fixtures/csharp_syntax_metadata/block_scoped_namespace/syntax-metadata-v0.json; csharp_block_scoped_namespace_syntax_v0; syntax backend C# block-scoped namespace route tranche; csharp_using_directive_syntax_metadata; tests/fixtures/csharp_syntax_sources/using_directive/source.cs; tests/fixtures/csharp_syntax_metadata/using_directive/syntax-metadata-v0.json; csharp_using_directive_syntax_v0; syntax backend C# using-directive route tranche; csharp_struct_declaration_syntax_metadata; tests/fixtures/csharp_syntax_sources/struct_declaration/source.cs; tests/fixtures/csharp_syntax_metadata/struct_declaration/syntax-metadata-v0.json; csharp_struct_declaration_syntax_v0; syntax backend C# struct declaration route tranche; csharp_enum_declaration_syntax_metadata; tests/fixtures/csharp_syntax_sources/enum_declaration/source.cs; tests/fixtures/csharp_syntax_metadata/enum_declaration/syntax-metadata-v0.json; csharp_enum_declaration_syntax_v0; syntax backend C# enum declaration route tranche; csharp_event_declaration_syntax_metadata; tests/fixtures/csharp_syntax_sources/event_declaration/source.cs; tests/fixtures/csharp_syntax_metadata/event_declaration/syntax-metadata-v0.json; csharp_event_declaration_syntax_v0; syntax backend C# event declaration route tranche; csharp_yield_statement_syntax_metadata; tests/fixtures/csharp_syntax_sources/yield_statement/source.cs; tests/fixtures/csharp_syntax_metadata/yield_statement/syntax-metadata-v0.json; csharp_yield_statement_syntax_v0; syntax backend C# yield statement route tranche; csharp_attribute_syntax_metadata; tests/fixtures/csharp_syntax_sources/attribute_syntax/source.cs; tests/fixtures/csharp_syntax_metadata/attribute_syntax/syntax-metadata-v0.json; csharp_attribute_syntax_v0; syntax backend C# attribute route tranche; csharp_generic_name_syntax_metadata; tests/fixtures/csharp_syntax_sources/generic_name/source.cs; tests/fixtures/csharp_syntax_metadata/generic_name/syntax-metadata-v0.json; csharp_generic_name_syntax_v0; syntax backend C# generic name route tranche; csharp_qualified_name_syntax_metadata; tests/fixtures/csharp_syntax_sources/qualified_name/source.cs; tests/fixtures/csharp_syntax_metadata/qualified_name/syntax-metadata-v0.json; csharp_qualified_name_syntax_v0; syntax backend C# qualified name route tranche; csharp_constructor_declaration_syntax_metadata; tests/fixtures/csharp_syntax_sources/constructor_declaration/source.cs; tests/fixtures/csharp_syntax_metadata/constructor_declaration/syntax-metadata-v0.json; csharp_constructor_declaration_syntax_v0; syntax backend C# constructor declaration route tranche; csharp_while_postfix_syntax_metadata; tests/fixtures/csharp_syntax_sources/while_postfix_loop/source.cs; tests/fixtures/csharp_syntax_metadata/while_postfix_loop/syntax-metadata-v0.json; csharp_while_postfix_loop_syntax_v0; syntax backend C# while/postfix loop route tranche; csharp_interpolation_syntax_metadata; tests/fixtures/csharp_syntax_sources/interpolated_string/source.cs; tests/fixtures/csharp_syntax_metadata/interpolated_string/syntax-metadata-v0.json; csharp_interpolated_string_syntax_v0; syntax backend C# interpolated string route tranche; csharp_member_access_expression_syntax_metadata; tests/fixtures/csharp_syntax_sources/member_access_expression/source.cs; tests/fixtures/csharp_syntax_metadata/member_access_expression/syntax-metadata-v0.json; csharp_member_access_expression_syntax_v0; syntax backend C# member access expression route tranche; csharp_field_declaration_syntax_metadata; tests/fixtures/csharp_syntax_sources/field_declaration/source.cs; tests/fixtures/csharp_syntax_metadata/field_declaration/syntax-metadata-v0.json; csharp_field_declaration_syntax_v0; syntax backend C# field declaration route tranche; csharp_base_list_syntax_metadata; tests/fixtures/csharp_syntax_sources/base_list/source.cs; tests/fixtures/csharp_syntax_metadata/base_list/syntax-metadata-v0.json; csharp_base_list_syntax_v0; syntax backend C# base list route tranche; csharp_conditional_access_syntax_metadata; tests/fixtures/csharp_syntax_sources/conditional_access/source.cs; tests/fixtures/csharp_syntax_metadata/conditional_access/syntax-metadata-v0.json; csharp_conditional_access_syntax_v0; syntax backend C# conditional access route tranche; csharp_array_rank_object_creation_syntax_metadata; tests/fixtures/csharp_syntax_sources/array_rank_object_creation/source.cs; tests/fixtures/csharp_syntax_metadata/array_rank_object_creation/syntax-metadata-v0.json; csharp_array_rank_object_creation_syntax_v0; syntax backend C# array-rank/object-creation route tranche; csharp_this_expression_syntax_metadata; tests/fixtures/csharp_syntax_sources/this_expression/source.cs; tests/fixtures/csharp_syntax_metadata/this_expression/syntax-metadata-v0.json; csharp_this_expression_syntax_v0; syntax backend C# this-expression route tranche; csharp_pattern_expression_syntax_metadata; tests/fixtures/csharp_syntax_sources/pattern_expression/source.cs; tests/fixtures/csharp_syntax_metadata/pattern_expression/syntax-metadata-v0.json; csharp_pattern_expression_syntax_v0; syntax backend C# pattern-expression route tranche; csharp_arrow_expression_clause_syntax_metadata; tests/fixtures/csharp_syntax_sources/arrow_expression_clause/source.cs; tests/fixtures/csharp_syntax_metadata/arrow_expression_clause/syntax-metadata-v0.json; csharp_arrow_expression_clause_syntax_v0; syntax backend C# arrow-expression-clause route tranche.
