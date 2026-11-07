# Security Policy

## Reporting a Vulnerability

Please report suspected vulnerabilities privately through GitHub's private
vulnerability reporting: open the repository's **Security** tab and choose
**Report a vulnerability**. Do not open a public issue for a security problem.

Include the schema or token stream that triggers the problem, the exact command
or API call, and the TCT version or commit.

## Supported Versions

TCT is pre-1.0. Only the latest release and the `main` branch receive security
fixes. The experimental frontends (the `experimental` Cargo feature) carry no
stability or security guarantees.

## What Is In Scope

TCT compiles a JSON Schema into Rust source code, and the generated tokenizers
decode token streams that may come from a language model. The following are
treated as security issues:

- **Compile time.** Running `tct`, `TctCompiler`, or `tct.compile` on a schema
  from an untrusted source must not execute code, read or write files other than
  the requested output, or open network connections. A crash (panic) or
  unbounded resource use on a well-formed schema is a bug worth reporting.
- **Generated code.** A generated tokenizer must not make network requests or
  read local files while it runs. External `$ref`s resolve only against the
  schemas vendored into the compiler (`src/generator/external_schemas.rs`); a
  reference to anything else makes the affected `anyOf`/`oneOf`/`allOf` branch
  fail to validate instead of being fetched. This is covered by the
  `test_generated_validators_never_fetch_external_refs` regression test.
- **Decoding.** `decode`, `decode_strict`, and their `_full` variants must
  return an error rather than panic or allocate without bound for any token
  stream. Collection lengths are capped (`DEFAULT_MAX_COLLECTION_LENGTH`).

## What Is Not In Scope

- **Running generated code.** Compiling and running a generated tokenizer is as
  trustworthy as running any code you build. When the schema is not yours,
  review generated output the way you would review other build artifacts.
- **Value constraints.** Generated tokenizers guarantee the structure of a
  decoded value, not value-level constraints such as `pattern`, `minLength`,
  `maxItems`, or numeric ranges; see "Decode Contract" in the README. Validate
  decoded values when you rely on those constraints.
- **Third-party schemas and fixtures** under `schemas/` and `tests/data/`;
  report findings in those upstream.
