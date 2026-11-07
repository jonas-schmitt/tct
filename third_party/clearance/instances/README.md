# Instance source and rights record

This record fixes the instance inventory of the original private release snapshot
`94dc544` (root `817cd20d7f6c3f1b51693b362d865a475ecb9b97`). The private six-commit
archive remains the recovery source. Public-export exclusions do not authorize
removing that archive or relabeling its original training results.

`ledger.json` records all 1,339 original paths and SHA-256 hashes. It selects 133
imported JSON files for retention with their upstream licenses and notices: all
113 copied workflows and 20 other configurations. Each imported file has a
repository, source path, immutable commit, source hash, local hash, conversion
record, and same-revision license evidence. Whole parsed values match; matching
only a filename, a current repository license label, or a few fields was not
accepted as source evidence.

The record also retains 210 repository-authored test fixtures based on the
original `PROVENANCE.md` declarations: 142 files beneath `synthetic/`, 66
synthetic-schema/decoder regression cases, and the two explicitly named workflow
examples `complete.json` and `partial.json`. These records state the declaration
being relied upon; a Git author or committer by itself does not establish
third-party copyright ownership. The two original Markdown documentation files
need their source descriptions updated for the public scope.

## Upstream attribution and license scope

The imported fixtures retain their original licenses. The root Apache-2.0
license does not replace those licenses. See [ATTRIBUTION.md](ATTRIBUTION.md) for
repository credits, including the CC-BY-4.0 sources, and `ledger.json` for the
exact source revision and modification record for every file.

`notices.json` lists the upstream URLs and SHA-256 hashes of the unmodified
license and notice copies under `../../licenses/instances/`. Source copyright
and license comment blocks lost during format conversion are preserved there
as separate notices. Workflow-directory license paths were checked at the same
revisions; the recorded HTTP 404 results describe those named-path checks, not
an assertion that every possible licensing exception was searched.

JSON, TOML, and YAML configurations were converted to JSON. Workflow conversion
also quotes YAML's `on` key and applies the checked-in collector's comment
preprocessing. Formatting and comments are not present in the JSON copies;
semantic equality was checked after this conversion. No upstream endorsement is
implied. Microsoft root third-party notice files are retained conservatively;
their presence does not assert that this release incorporates every dependency
listed in them.

## Approved public-export exclusions

`excluded-paths.json` is the exact path/hash list for 994 original JSON files:

- 923 Kubernetes manifests sourced through `substratusai/the-stack-yaml-k8s`.
  The [dataset card](https://huggingface.co/datasets/substratusai/the-stack-yaml-k8s/blob/0ff728d19f730ecce3392f64dccc2885598b308e/README.md)
  includes The Stack's conditions: original source licenses remain applicable,
  and dataset sharing requires passing on its terms and obtaining recipient
  agreement. The record does not equate the dataset's permissive-source
  description with unrestricted redistribution permission. The public export
  excludes these copies because the dataset terms and original-file notices
  have not both been cleared. Recovered dataset row metadata stays in private
  scratch evidence, outside the public release.
- 71 other configuration/API fixtures whose exact original source or authorship
  was not established. Some resemble hand-written examples; resemblance alone
  is not the basis for shipping them. Their private originals remain available
  for later individual clearance.

These exclusions target the identified copies, not all real-world configurations
or all third-party data. Fresh repository-authored fixtures have a separate
additions record and do not inherit the removed files' provenance.

## Tests and BPE artifacts

The Kubernetes exclusions affect the three original default smoke fixtures in
`tests/round_trip_test.rs`, the Kubernetes streaming-wheel fixture, the full
Kubernetes round-trip corpus, and `kubernetes-tct-bpe-500`. The other exclusions
affect the original ESLint default smoke, `tsconfig/real/strict.json`, several
focused smoke families, and the old ESLint/package/tsconfig BPE input sets. The
workflow input set is individually sourced; its handwritten examples remain
repository fixtures.

`original-bpe-impact.json` binds each of the six original BPE sidecars to its
SHA-256 and lists all excluded training inputs. Their recorded training root
stays `817cd20d7f6c3f1b51693b362d865a475ecb9b97`. Public artifacts must be retrained
after the cleared source snapshot is committed, with new provenance; changing
or pruning input files does not validate an old sidecar. The original tables,
sidecars, and private test evidence remain in the private archive.

Run `python3 third_party/clearance/instances/verify.py` to check retained file
hashes and all vendored notice hashes. Add `--require-exclusions` after applying
the public-export exclusions. `--snapshot 94dc544` verifies the original inventory
against that Git snapshot instead of checking live fixture paths. These local
checks verify the recorded artifacts, not upstream legal conclusions.
