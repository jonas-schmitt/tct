# Vendored source and syntax-fixture clearance

This review covers the exact files in [inventory.json](inventory.json) from
`jonas-schmitt/tct` commit `94dc54434f5164343d0626958a472c16fb5204e3`.
It concerns a source distribution. It does not clear other files, Git history,
downloaded corpora, trained artifacts, binaries, or wheels. The private research
repository is not the reviewed release target.

The machine-readable inventory records every file's SHA-256, source identity,
modification or transformation, retained notices, and redistribution decision.
An upstream entry is admitted only with its original terms and notices. Local
synthetic fixtures and contract descriptions remain TCT material under the
project owner's Apache-2.0 authorization. That classification is based on
inspected fixture contents and owner authority, not merely failure to find an
upstream match. Mixed implementation entries describe the listed third-party
dependencies; they do not establish ownership of unrelated implementation code.

## Verified sources

| Material | Exact comparison and scope | Retained terms |
| --- | --- | --- |
| Go corpus | All 155 imported `.go` files match `golang/go@34c8b14ca9f4096383d658fbd748322a993a2bd2`; the `go1.24.3` tag resolves to that commit. The 13 other corpus programs are local syntax examples. | [Go BSD-3-Clause](../../licenses/go/LICENSE), original per-file copyright headers, and the [patent grant](../../licenses/go/PATENTS). |
| Go AST inventory | Its recorded hashes for `src/go/ast/ast.go` and `src/go/token/token.go` match that same immutable source. The JSON adds TCT grammar/trivia policy. | Go BSD notice plus TCT notices. |
| C# mini corpus | All five sources match `dotnet/samples@c6eb3268ba912447e820ba78e3d35ee2c5d7e0b0`, including original BOM/newline bytes. | [Exact upstream MIT code license](../../licenses/dotnet/LICENSE-CODE). The older corpus license differs only in whitespace. |
| Tree-sitter metadata | All four `node-types.json` exports and their existing license files match the immutable commits recorded in their C, JavaScript, JSON, and Rust pins. | Exact [C](../../licenses/tree-sitter/c/LICENSE), [JavaScript](../../licenses/tree-sitter/javascript/LICENSE), [JSON](../../licenses/tree-sitter/json/LICENSE), and [Rust](../../licenses/tree-sitter/rust/LICENSE) MIT notices. |

The complete upstream Go archive was checked for license files on each selected
source's ancestor path. The one directory exception,
[`crypto/internal/boring/LICENSE`](../../licenses/go/BORING-LICENSE), says Go
source/supporting files retain the usual Go license. Its additional BoringSSL
terms concern linked object files, which this corpus does not contain.

`crypto/internal/fips140/edwards25519/scalar.go` retains an embedded fiat-crypto
BSD-1-Clause notice referring to `scalar_fiat.go`. The latter is not in the
corpus. The [unchanged comment excerpt](../../licenses/go/fiat-crypto-NOTICE.txt)
is also retained centrally so that derivatives can carry it without the full
source file. This does not label the whole Go source as dual licensed.

The pinned dotnet
[ThirdPartyNotices.md](https://github.com/dotnet/samples/blob/c6eb3268ba912447e820ba78e3d35ee2c5d7e0b0/ThirdPartyNotices.md)
distinguishes MIT code from CC-BY-4.0 documentation. The five included code files
fall under the former; no upstream documentation is copied into this notice
bundle. Complete recursive upstream tree listings were checked for nested
license exceptions along the selected paths. The four tree-sitter trees contain
only a root license file. No upstream `grammar.json` file is tracked in this
release; downloaded versions were comparison evidence only.

## Derivatives and embedded material

All 215 Go and 54 C# syntax-metadata fixtures record input paths and hashes that
match the inventoried inputs. The 143 Go projection reports and one source-span
report likewise match their source identities and, where present, their checked
metadata identities. Their transformations and parser versions are recorded;
this rights review does not reproduce or promote their experimental findings.
Roslyn's use as a parser does not make every output Microsoft code: local
synthetic inputs remain local material, while metadata derived from the five
dotnet sources retains the MIT notice conservatively. Roslyn binaries are not
part of this source-only clearance.

Five complete Go source literals in `src/syntax_generated_codec.rs` match the
cleared source files exactly. Additional source literals are included through
`include_str!`; the ledger identifies the source/metadata consumers and their
required notices. Retain these notices when exporting generated code containing
those fixture payloads. Merely excluding corpus directories from an archive does
not remove copies embedded in a source file or generated output.

The remaining inventoried selection lists, layout pins, contract descriptions,
and toy programs are local TCT fixtures. A pin naming external stress modules
does not clear a subsequently downloaded module tree. Any such publication needs
its own exact-version file and notice review.

## Export and verification

Treat the inventory as a path-and-hash allowlist for this scope, together with
each entry's `required_notices`. Keep the original headers. A changed or newly
included file needs a new review. The root `LICENSE` and `NOTICE` must accompany
TCT material; upstream content keeps its own license. Preserve the central
notice bundle even when an export omits the original corpus license locations.

Run from the repository root:

```sh
python3 third_party/clearance/vendored/verify_inventory.py
```

This checks current file hashes, completeness of the named fixture directories,
retained notice hashes, and source/dependency identities. It does not contact a
network service or make legal conclusions. With the original downloaded evidence
directory available, add `--upstream-cache /path/to/evidence` to repeat the
upstream byte comparisons. Immutable URLs and downloaded hashes remain in the
inventory so an independent reviewer can retrieve the evidence again.

Unlisted repository files, schemas, instance data, training/BPE artifacts,
migration patches, untracked generated output, and existing Git history are
outside this allowlist. This is an implementable source-scope decision, not a
blanket assertion that every TCT artifact can be redistributed.
