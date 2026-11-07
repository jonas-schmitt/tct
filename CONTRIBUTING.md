# Contributing to TCT

Thanks for your interest in TCT. Bug reports, schema compatibility reports,
and pull requests are welcome.

## Reporting Problems

Open an issue with:

- the schema (or a minimal schema that reproduces the problem),
- the instance JSON, if encoding or decoding is involved,
- the exact command or API call and the full error output,
- the TCT commit or version and the Rust toolchain from `rust-toolchain.toml`.

Schema compatibility issues are most useful when reduced to a small synthetic
schema under `schemas/synthetic/` with a matching instance under
`tests/data/instances/`.

Report suspected vulnerabilities privately instead of opening an issue; see
[SECURITY.md](SECURITY.md).

## Development Setup

```bash
git clone https://github.com/jonas-schmitt/tct.git
cd tct
cargo build --release
cargo test
```

`rust-toolchain.toml` pins the toolchain; `rustup` installs it on first use.
`DEVELOPMENT.md` documents the command reference, validation layers, and the
generated-crate and wheel workflows.
Read `GLOSSARY.md` for the project vocabulary and `ARCHITECTURE.md` before
changing a compiler pass, the decode contract, or sequence wrappers.

## Engineering Contracts

- Keep discovery, analysis, optimization, and generation as separate passes.
- Implement core compiler and tokenizer behavior in Rust; use Python for
  external utilities.
- Specialize generated hot paths at compile time, without runtime schema
  introspection or avoidable allocation.
- Reject unsupported schema features and invalid input with contextual typed
  errors instead of panics or silent fallbacks.
- Classify token-layout changes with `tct-token-diff` before reusing downstream
  merge tables or model artifacts. String encoding, `max_base_vocab`, BPE base
  vocabulary, and top-level token ranges all affect compatibility.
- Keep reproducible provenance for published results and model experiments:
  schema, corpus, tokenizer configuration, BPE artifact, seed, command, and
  commit. Retain BPE merge tables' `*.provenance.json` sidecars.

## Pull Requests

- Keep each pull request to one coherent change: one bug fix, one feature, or
  one documentation update.
- Add or update a focused test first for correctness bugs, schema semantics,
  token accounting, and generated-code behavior, then make the smallest change
  that fixes it.
- Run the stable checks before opening the PR:

  ```bash
  cargo fmt --all -- --check
  cargo clippy --all-targets
  cargo test
  ```

  `./scripts/testing/quality-check.sh` runs these together with the Python
  maintenance tests and a release build. Escalate to the round-trip,
  target-schema, generated-reproducibility, or wheel
  gates described in `DEVELOPMENT.md` when the change touches generated code,
  token layouts, BPE artifacts, or Python packaging.
- Round-trip integrity is mandatory for compiler and tokenizer changes:
  `JSON -> tokens -> JSON` must preserve semantic data for supported schemas.
- Every emitted token ID must stay below `VOCAB_SIZE`, and the base range
  (shared by byte, end-marker, and structural tokens), BPE merge IDs, and
  `PAD_TOKEN` must not overlap. Payload must never contain `PAD_TOKEN`.
- Avoid `unwrap()` and `expect()` in library code unless a documented invariant
  makes failure impossible; prefer typed errors with schema paths or token
  positions.
- Update `README.md`, `ARCHITECTURE.md`, or `DEVELOPMENT.md` in the same change
  when commands, flags, or contracts change. Maintained docs must name existing
  paths, tests, flags, and variables; put volatile measurements and test counts
  in reports under `target/`.

## Commit Messages

Describe the engineering change and how it was validated.

## License

By contributing, you agree that your contributions are licensed under the
Apache License, Version 2.0 (see `LICENSE`).
