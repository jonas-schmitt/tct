# TCT Schema Catalog

Schemas in this catalog are used by the compiler's test suites and examples.
Compression artifacts are BPE-only on top of plain TCT token sequences; legacy
dictionary-compression artifacts are not accepted.

## JSON Schemas

### `popular/`
Popular JSON schemas including:
- `github-workflow.json` - GitHub Actions workflow schema (production)
- `package.json` - npm package.json
- `tsconfig.json` - TypeScript configuration
- And many more...

`target-schemas.txt` is the maintained regression manifest consumed by
`../scripts/testing/test-target-schemas.sh`.

### `synthetic/`
Test schemas for validation and edge case testing:
- Minimal schemas for debugging
- Edge case patterns (recursive, conditional, etc.)
- Token signature validation
- `decoder/` - union decoding regression cases
- `encoding_patterns/` - encoding-shape cases for the generator

### Other Schema Collections
- `other/` - Additional JSON schemas
- `common/` - Commonly used schemas
- `specialized/` - Domain-specific schemas
- `configuration/` - Configuration file schemas
- `api/` - API specification schemas
- `cloud/` - Cloud configuration schemas
- `external/` - Vendored external schemas embedded by the generator
- `bpe/` - Provenance-backed BPE merge artifacts used by tests and the
  artifact manifest (`scripts/analysis/bpe_artifact_manifest.py`)
- `bpe_merges/` - Conventional location for your own trained merge tables

`target-schema-depth.txt` lists the schemas checked by
`../scripts/testing/test-target-schema-depth.sh`.

## Training Tokenizers

Merge tables are generated with `tct-prepare-data` and `tct-train-bpe`. See
`../DEVELOPMENT.md` for the training commands and report contracts. Keep the
schema, corpus, BPE artifact, command, and output metadata needed to reproduce
any measurement you report.

## Source and license records

The public catalog includes only reviewed source material. Immutable source
versions, modifications, and excluded optional downloads are recorded under
`third_party/clearance/schemas/`. Directory names such as `synthetic/` do not
replace per-file provenance: derived package-schema subsets retain their
upstream notices.
