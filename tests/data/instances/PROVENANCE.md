# Provenance of the test instances

This document describes the reviewed public source selection. A private
preparation checkout can also hold original inputs excluded from that selection;
its history must not be made public as a substitute for the reviewed export.

## Retained imported files

[The instance ledger](../../../third_party/clearance/instances/ledger.json)
records original local hashes, exact upstream repositories, source paths,
immutable revisions, conversions, and license/notice evidence. Only entries
explicitly approved for retention belong in the public source. A repository's
current detected license, a familiar filename, or public availability alone is
not an inclusion rule.

Copied GitHub workflows and other retained configurations keep their original
terms. [Required notices](../../../third_party/clearance/instances/notices.json)
are retained under `third_party/licenses/instances/`. YAML sources were
converted to JSON; the recorded conversion and hashes identify the copies.
[CC-BY attribution](../../../third_party/clearance/instances/ATTRIBUTION.md)
identifies those sources and changes separately. No upstream endorsement is
implied.

## Authored examples

Repository-authored synthetic cases are identified explicitly in the original
ledger and the [additions record](../../../third_party/clearance/instances/authored-additions.json).
The new public examples were composed from schema fields independently of the
excluded files. Directory names alone do not establish origin or rights.

The public Kubernetes cases are authored examples. Copies acquired through
`substratusai/the-stack-yaml-k8s` are excluded: their dataset conditions and
original-file notices were not both cleared for this distribution. Other
configurations with unresolved origins are excluded individually. Their private
originals are preserved for recovery and later review; they are not relabeled
as synthetic or as inputs of newly trained public artifacts.

## BPE tables

Public merge tables must name a clean public source commit, the exact included
training inputs, compiler configuration, fixed training date, and output hashes.
The public training recipe uses authored examples. The old private tables and
results retain their original provenance and are not measurements of the public
fixture corpus. Artifact filename budgets are training targets; actual vocabulary
and merge counts come from the artifact metadata.

## Adding a fixture

Record its exact path, content hash, and origin. For an import, pin the original
file and its license/notice scope at an immutable revision, retain required
notices, and record transformations. For a new authored case, identify it as
such and validate it against its schema. Do not add credentials. Extend the
explicit public selection and its tests before distributing the new file.
