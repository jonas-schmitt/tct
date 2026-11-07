# Schema source redistribution review

Review date: 2026-11-07. Release baseline:
`94dc54434f5164343d0626958a472c16fb5204e3`.

[embedded-schemas.json](embedded-schemas.json) records immutable, content-matched
sources for all seven schemas embedded by `src/generator/external_schemas.rs`.
Before the publication notices, five files were byte-for-byte copies and two
were exact JSON reserializations. The latter now also carry prominent `$comment`
change notices. Both the original and redistributed hashes and transformations
are retained in the ledger and [MODIFICATIONS.md](MODIFICATIONS.md). Each source revision's
root LICENSE and NOTICE was read and hash-compared with the retained copies;
the schema contents, per-file history and repository tree were checked for
separate origin or license terms. A matching revision proves content identity;
it does not identify the historical download date.

The six files other than `partial-eslint-plugins.json` have no detected narrower
license exception. Their source redistribution is covered by the retained
[SchemaStore Apache-2.0 license](../../licenses/schemastore/LICENSE) and
[NOTICE](../../licenses/schemastore/NOTICE), with the recorded change notice for
the reformatted `jscpd.json`.

## The ESLint plugin schema

This file receives additional treatment because its own `$comment` identifies
an upstream generator and its contents include plugin rule metadata.

1. SchemaStore commit `beb246f25acc8f6cf473071d8ec15cf8359855f0` contains a
   schema JSON-value-identical to the **committed schema file**
   [`schemas/partial-eslint.schema.json`](https://github.com/fox-projects/jsonschema-extractor/blob/648bbeb51fdb459c193acdc7756d5f79e1b3865a/schemas/partial-eslint.schema.json)
   in the extractor repository. That revision supplies a root MPL-2.0 license
   and an MPL-2.0 package declaration, with no nested license exception.
2. Four subsequent SchemaStore commits make the changes recorded as an exact
   JSON-value patch in [eslint-schema-changes.json](eslint-schema-changes.json).
   The final upstream schema then reproduces the local bytes through the
   documented serialization.
3. The extractor source names five plugin inputs. Their exact versions and
   archive SHA-512 values come from its pinned lockfile. All five downloaded
   archives match those lock values, and their MIT license notices are retained.
   The pinned ESLint runtime notice is also retained because the TypeScript
   plugin inherits core rule schemas (`schema: baseRule.meta.schema`). See
   [eslint-plugin-inputs.json](eslint-plugin-inputs.json).

For redistribution, this review follows the extractor's committed source grant
conservatively: **the plugin schema source file and its modifications are made
available under MPL-2.0**, with Apache-2.0 notices retained for SchemaStore
contributions and MIT notices retained for copied rule metadata. The basis is
the separately published, committed schema file; running an MPL-licensed
generator alone would not establish the license of arbitrary output. This
does not change the license of unrelated TCT files or replace the upstream
grants. MPL permits combining its files with Apache-licensed files in a larger
work; see [Mozilla's MPL FAQ, questions 8–13](https://www.mozilla.org/en-US/MPL/2.0/FAQ/).

The source form is `schemas/external/partial-eslint-plugins.json` in this same
source distribution, identified by the SHA-256 in the ledger. Retain the
[MPL license](../../licenses/jsonschema-extractor/LICENSE), this notice, all
listed upstream notices, and the source when distributing this checkout.
For an executable, wheel, or generated crate that embeds this schema, supply
the same exact schema source alongside the artifact and tell recipients where
it is provided. A floating link to another version is insufficient to identify
the modified source included in that artifact. This source review does not
qualify any binary package.

## Wider catalog

[catalog-inventory.json](catalog-inventory.json) hashes and classifies every
retained JSON file under `schemas/`, including BPE artifacts. The review is
split into disjoint groups so a filename, directory, or catalog entry does not
stand in for rights evidence:

- [schemastore-catalog.json](schemastore-catalog.json): exact upstream copies
  and reserializations, with immutable per-file content and source-license
  scope. The [additional-source review](catalog-additional-sources.json)
  preserves Taplo's MIT notice for its imported schemas and their descendants,
  Rustfmt's own notices, and the mprocs author's identified SchemaStore
  contribution. The Gist's public availability is not the permission basis.
- [owner-authored.json](owner-authored.json): individually inspected small
  test schemas, tied to their introduction commits, paths, authors and hashes.
  The large copied package subsets are excluded from this group.
- [derived-schemas.json](derived-schemas.json): seven exactly reconstructed
  package subsets and the workflow schema. Property selections, JSON patches,
  a byte-exact workflow patch, original hashes and publication notices are
  retained. The workflow's original import exactly matches its pinned source.
- [kubernetes-schemas.json](kubernetes-schemas.json): the default Kubernetes
  schema and its unenhanced precursor. The pinned Kubernetes 1.30 definitions
  and two retained historical TCT transformations reproduce the original bytes
  exactly. The public copies add prominent change notices. The upstream
  attribution to Gareth Rushgrove and Yanh Hamon and the Kubernetes license
  are retained.
- [retained-framework-schemas.json](retained-framework-schemas.json): Babel,
  Cypress and GitLab CI. Babel's historical documentation has an ISC declaration and
  later MIT terms; both original notices are retained alongside SchemaStore's
  notices. The Cypress file is byte-identical to its original v9.5.4 source at
  an immutable commit, with that revision's MIT notice. GitLab CI likewise
  matches its immutable original source, whose path falls under the retained
  root license's MIT terms and outside its doc/ee/jh exceptions.
- [pending-schemas.json](pending-schemas.json): explicit unresolved paths and
  original-source leads. These include other external schemas, the invalid
  Lottie download response, BPE artifacts, and the OASIS SARIF terms.

At this reviewed snapshot there are 239 reviewed paths and 41 pending paths;
the verifier checks that the groups cover all 280 inventoried JSON files without
overlap. The pending paths require separate clearance or exclusion from the
public candidate. Existing instance-data provenance tables do not clear
schemas or artifacts, and this review does not clear instance data or Git
history.

The [26 initial](approved-notice-paths.json) and
[eight additional](approved-derived-notice-paths.json) approved public-copy
edits add only `$comment` annotations and JSON formatting. Original private
schema hashes and upstream transformations are retained. Token-layout and
compiler-semantic qualification of this new public candidate remains a separate
gate; content/provenance verification is not that qualification.

## Verification

Run from the release repository:

```sh
python3 third_party/clearance/schemas/verify.py
```

The default command checks the complete private review snapshot and its pending
paths. It does not declare all those paths cleared. For a public export, use:

```sh
python3 third_party/clearance/schemas/verify.py --public-scope
```

Public mode requires every reviewed schema with its recorded hash and rejects
any remaining pending schema or unreviewed extra JSON. It also checks the
version 2 `third_party/clearance/bpe-artifacts.json` manifest: exactly six public
tables and six original sidecars, their byte counts and hashes, the six authored
training recipes, complete fixture coverage, and the current content-addressed
source snapshot. The manifest's `original_training` commit/tree/receipt fields
are explicitly archival and are never resolved from public Git history.

Run `python3 scripts/analysis/public_bpe_source_manifest.py` for the same BPE
source/input checks alone. Actual retraining is the separate mandatory
`python3 scripts/testing/test-bpe-artifact-reproducibility.py --public-artifacts`
gate. See [`docs/publication.md`](../../../docs/publication.md) for the source
snapshot contract and release sequence. The original 280-file review ledger is
retained unchanged to document which pending files were excluded.

To rederive the seven source matches, historical SchemaStore notice identities,
catalog matches and the extractor transformation from local upstream clones:

```sh
python3 third_party/clearance/schemas/verify.py \
  --schemastore /path/to/schemastore \
  --extractor /path/to/jsonschema-extractor
```

Those repositories must contain the pinned objects. The verifier does not
install or execute upstream packages. Notice copies are bound to their original
URLs and hashes in the ledger; the npm archive integrity values are additionally
bound to the historical extractor lockfile.

Add `--kubernetes-definitions /path/to/v1.30.0/_definitions.json` to reproduce
the two Kubernetes schema files from their hash-bound input. Add
`--research /path/to/preserved-private-repository` to re-read the authored-test
introduction receipts. These optional checks do not publish that history or
change its contents.

`--babel-website /path/to/babel-website` re-reads the historical documentation,
ISC declaration, and retained ISC/MIT notice sources. `--cypress-schema
/path/to/pinned/cypress.schema.json` verifies the exact original Cypress bytes.
`--gitlab-schema /path/to/pinned/ci.json` performs the corresponding GitLab check.
