# Round-Trip Instance Data

JSON instances used by the round-trip tests (`tests/round_trip_test.rs`) and
the target schema gates. Each directory is named after a schema in `schemas/`
(for example `package/` pairs with `schemas/popular/package.json`, `eslintrc/`
with `schemas/popular/eslintrc.json`, and `api-response/` with
`schemas/api/api-response.json`); directories without a schema counterpart
exercise synthetic schemas under `schemas/synthetic/`.

## Layout

- `<schema>/real/` - individually reviewed imported configuration files. Exact
  sources and transformations are in the provenance ledger.
- `<schema>/synthetic/` - hand-written instances that target specific schema
  features or decoder edge cases.
- `<schema>/large-scale/` - larger corpora used only by full round-trip runs
  (`ROUND_TRIP_FULL=1`). Only `github-workflow/large-scale/` exists, as a small
  sample; see [PROVENANCE.md](PROVENANCE.md) for how to rebuild the full corpus.
- Directories with files directly inside them are small instance sets.

## How the tests use them

The default round-trip smoke (`cargo test --test round_trip_test
test_round_trip`) compiles the default schema set and round-trips a few
explicitly listed instances per schema (see `DEFAULT_SMOKE_FIXTURES` in
`tests/round_trip_test.rs`). `SCHEMA_NAME` and `SCHEMA_FOLDER` select one
schema; `ROUND_TRIP_FULL=1` walks every instance under the schema's directory.
Results are written to `target/tct-round-trip-report.json`. See
`DEVELOPMENT.md` ("Round-Trip Testing") for the full option list.

## Adding instances

1. Put the file under `tests/data/instances/<schema>/real/` or `.../synthetic/`.
2. Make sure it validates against the schema; the round trip compares the
   decoded JSON with the input semantically.
3. Add it to a smoke fixture list in `tests/round_trip_test.rs` only when it
   should run in the default gate; everything else is covered by
   `ROUND_TRIP_FULL=1`.
4. Do not add files that contain credentials, even placeholder-looking ones.
5. For an imported file, record its immutable source revision, content hash,
   applicable file/directory terms, conversion, and required notices in the
   instance ledger. Follow [PROVENANCE.md](PROVENANCE.md); a current repository
   license label alone does not clear redistribution.

Third-party files keep their original licenses; [PROVENANCE.md](PROVENANCE.md)
lists the sources and the rule that decides what is shipped.
