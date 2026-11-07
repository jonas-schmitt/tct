# Notices of changes to imported schema files

These notices accompany the exact files and content hashes below. Upstream
licenses and copyright notices are preserved separately in the paths recorded
by [embedded-schemas.json](embedded-schemas.json).

- **`schemas/external/jscpd.json` was modified by TCT:** the SchemaStore file at
  commit `d6f9a7fe6b027cc682f0b43baa72a37e634c28f5` was parsed and reserialized
  with four-space indentation, ASCII escaping, and one final newline. No JSON
  value was changed. The pre-publication-notice SHA-256 is
  `0d8c5901c0787fd0b6a99c1094d6d66fc78cb390dd4763152898f7f53b17b040`.
- **`schemas/external/partial-eslint-plugins.json` was modified by TCT:** the
  SchemaStore file at commit `4c41ffc223df321b410f504099351dc7d1613f8a` was
  parsed and reserialized using the same operation. No JSON value was changed.
  The pre-publication-notice SHA-256 is
  `710695547cdd01d06940993e7d85d93405cabe8326fd20e84db078ccdd9ff4d9`.
  The semantic changes between its earlier extractor base and the SchemaStore
  revision are upstream changes, recorded in
  [eslint-schema-changes.json](eslint-schema-changes.json); they are not
  attributed to TCT. This schema file and its modifications are available under
  MPL-2.0 with the additional upstream notices described in the
  [source-scope review](README.md).

The exact reserialization is:

```python
(json.dumps(json.loads(upstream_bytes), indent=4, ensure_ascii=True) + "\n").encode("utf-8")
```

Both public-copy files now additionally carry a prominent `$comment` change
notice inside the JSON. Existing comments are retained. The ledger records the
exact appended text, serialization, and redistributed SHA-256 separately from
the historical hashes above. The other five embedded schemas are byte-identical
to the ledger's pinned sources.

The complete modified catalog scope is recorded in
[approved-notice-paths.json](approved-notice-paths.json) and
[approved-derived-notice-paths.json](approved-derived-notice-paths.json): 24
upstream reserializations, two Kubernetes derivations, seven package subsets,
and one workflow schema with recorded compatibility edits. Each public file
carries its own change notice. [catalog-inventory.json](catalog-inventory.json)
binds the current bytes to the applicable source and transformation ledgers.

These notices do not describe a future revision of these files. Distributions
containing a modified file must retain its associated change notice, license
and attribution.
