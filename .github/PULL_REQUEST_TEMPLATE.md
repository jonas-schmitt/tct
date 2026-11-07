## What and why

<!-- One coherent change: a bug fix, a feature, or a documentation update. -->

## Validation

<!-- Which checks did you run? For correctness bugs, add or update a focused
test that fails without the change. -->

- [ ] `cargo fmt --all -- --check`
- [ ] `cargo clippy --all-targets`
- [ ] `cargo test` (and `cargo test --features experimental` if you touched an experimental frontend)
- [ ] Round-trip, target-schema, generated-reproducibility, or wheel gates from `DEVELOPMENT.md`, if the change touches generated code, token layouts, BPE artifacts, or Python packaging
- [ ] `README.md`, `ARCHITECTURE.md`, or `DEVELOPMENT.md` updated if commands, flags, or contracts changed
