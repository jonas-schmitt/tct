# Third-party source and redistribution notices

TCT-authored implementation is licensed under [Apache-2.0](LICENSE), with
copyright retained by Jonas Schmitt. Imported material keeps its own terms.
[NOTICE](NOTICE) is a companion attribution notice, not a blanket license
for every input schema or fixture.

## Source records

The inventories bind exact file bytes to source and notice evidence:

- [Schemas](third_party/clearance/schemas/README.md): immutable upstream
  versions, original hashes, transformations, modification notices, and
  applicable per-file terms. The compiler's embedded resources are recorded
  separately from optional catalog files.
- [Configuration fixtures](third_party/clearance/instances/README.md): exact
  upstream files and revisions, YAML-to-JSON conversions, licenses, and
  [CC-BY attribution](third_party/clearance/instances/ATTRIBUTION.md). New
  authored fixtures have their own additions record.
- [Vendored syntax inputs](third_party/clearance/vendored/README.md): Go
  source and derived metadata, C# source, tree-sitter exports, locally authored
  examples, and retained notice documents.

Licenses and notices are retained under [third_party/licenses](third_party/licenses).
A file's presence in an inventory is not enough by itself: pending and excluded
entries are identified explicitly. The reviewed public source selection omits
uncleared copies. The original private release archive remains separate;
changing repository visibility would expose a different scope from that export.

## Embedded ESLint schema source

The committed upstream ESLint plugin schema has MPL-2.0 source terms, with
additional Apache and MIT notices for its contributions and inputs. Its exact
source is [partial-eslint-plugins.json](schemas/external/partial-eslint-plugins.json).
The [component notice](third_party/clearance/schemas/ESLINT_SOURCE_NOTICE.md)
identifies its versions, modifications, source form, and required notices.
This file-specific obligation does not change the license of unrelated TCT
implementation files.

The compiler's root Python wheel includes the exact embedded component source
and applicable notices. Generated schema packages conservatively include the pinned component payload
when a recognized reference is found in the input or its vendored resources,
or the input matches the known component source. This can include a reference
in an unused definition or annotation; it does not assert that generated code
incorporates that resource. When distributing standalone generated Rust
that incorporates it, provide the same modified schema source and notices;
a floating link to a different version does not identify the included source.
Arbitrary input schemas can have additional terms that the compiler cannot
infer or replace with its own license.

## Generated artifacts and dependencies

Generated Rust contains copied TCT implementation as well as schema-derived
structures. Preserve its scoped attribution and applicable LICENSE/NOTICE
files. Tokenizing data does not change the data's license. Source inputs,
compiled dependencies, generated implementation, and fixture corpora can have
different redistribution obligations.

BPE examples for a public release must use the cleared source revision and
recorded training inputs. The original private merge tables and research
results are not relabeled as results from the public fixtures. A source rights
review does not replace inspection of each distributed wheel or binary and
its actual dependency notices.
