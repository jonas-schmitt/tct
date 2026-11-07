# Embedded ESLint plugin schema: source and license notice

This component contains `schemas/external/partial-eslint-plugins.json` in its
preferred, editable JSON source form. **That schema source file and its
modifications are made available under MPL-2.0.** The accompanying license is
`third_party/licenses/jsonschema-extractor/LICENSE`. This file-specific notice
does not change the license of unrelated TCT implementation files.

The immediate source is SchemaStore
`4c41ffc223df321b410f504099351dc7d1613f8a`, path
`src/schemas/json/partial-eslint-plugins.json`. Retain its Apache-2.0 license
and attribution at `third_party/licenses/schemastore/LICENSE` and
`third_party/licenses/schemastore/NOTICE`.

Its base matches the committed `schemas/partial-eslint.schema.json` in
`fox-projects/jsonschema-extractor` at
`648bbeb51fdb459c193acdc7756d5f79e1b3865a`, where it is published within the
MPL-2.0 source repository. SchemaStore's intermediate regeneration revision is
`beb246f25acc8f6cf473071d8ec15cf8359855f0`. This source-file provenance, rather
than merely use of a generator, is the basis for the MPL treatment.

The schema includes rule metadata from MIT-licensed packages. Preserve these
copyright and permission notices:

- `third_party/licenses/schema-eslint-inputs/angular-eslint-eslint-plugin-18.0.1/LICENSE`
- `third_party/licenses/schema-eslint-inputs/typescript-eslint-eslint-plugin-7.13.1/LICENSE`
- `third_party/licenses/schema-eslint-inputs/eslint-plugin-import-2.29.1/LICENSE`
- `third_party/licenses/schema-eslint-inputs/eslint-plugin-import-2.29.1/memo-parser-LICENSE`
- `third_party/licenses/schema-eslint-inputs/eslint-plugin-unicorn-54.0.0/license`
- `third_party/licenses/schema-eslint-inputs/eslint-plugin-vue-9.26.0/LICENSE`
- `third_party/licenses/schema-eslint-inputs/eslint-8.57.0/LICENSE`

TCT parsed and reformatted the pinned SchemaStore JSON without changing its
JSON values, then appended a prominent publication notice in `$comment`.
Upstream semantic fixes between the extractor base and SchemaStore's final
revision are recorded separately and are not attributed to TCT. Exact hashes,
source URLs and transformations are in [embedded-schemas.json](embedded-schemas.json),
[eslint-plugin-inputs.json](eslint-plugin-inputs.json), and
[eslint-schema-changes.json](eslint-schema-changes.json).

When this component accompanies a binary, wheel or generated crate, recipients
can obtain the exact modified schema source from the
`schemas/external/partial-eslint-plugins.json` file shipped in this same
component payload. Retain that source, this notice and the listed licenses
together. Distribution terms for the larger artifact must not restrict the
recipient's MPL rights in this source file.
