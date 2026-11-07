# C# dotnet/samples Mini Corpus

This checked corpus is sidecar route-selection evidence for C# frontend
planning. It is not syntax-backend stable evidence, source-program validity
evidence, type-correctness evidence, or generated-codec real-source evidence.

The files were selected from `https://github.com/dotnet/samples` at commit
`c6eb3268ba912447e820ba78e3d35ee2c5d7e0b0`. Upstream code is MIT licensed
under `LICENSE-CODE`; the normalized checked copy is
`LICENSE-CODE.MIT.txt`.

Selected files are intentionally small standalone source files that the current
C# evidence helper can compile one at a time as library inputs:

- `delegates-and-events/Logger.cs` from
  `csharp/delegates-and-events/Logger.cs`
- `events/VersionOne.cs` from `csharp/events/VersionOne.cs`
- `iterators/IteratorMethods.cs` from
  `csharp/iterators/IteratorMethods.cs`
- `pattern-matching/Shapes.cs` from
  `csharp/PatternMatching/Shapes.cs`
- `serialization/Loan.cs` from `csharp/serialization/Loan.cs`

`sources.txt` is generated with:

```bash
python3 scripts/analysis/route_source_manifest.py \
  --target csharp \
  --source-root corpora/csharp/dotnet-samples-mini/src \
  --output corpora/csharp/dotnet-samples-mini/sources.txt \
  --json-output target/tct-csharp-real-source-corpus/route-source-manifest.json \
  --markdown-output target/tct-csharp-real-source-corpus/route-source-manifest.md \
  --min-files 5 \
  --corpus-id csharp-dotnet-samples-mini-v1 \
  --corpus-source https://github.com/dotnet/samples/tree/c6eb3268ba912447e820ba78e3d35ee2c5d7e0b0 \
  --corpus-license MIT \
  --selection-note 'Small standalone C# files from dotnet/samples chosen for parser/toolchain route planning; sidecar evidence only.'
```

Use `./scripts/testing/test-csharp-real-source-corpus.sh` to regenerate the
source manifest under `target/`, compare it to the checked manifest, and
validate `route_source_manifest` fingerprints against the selection manifest.
Isolated `dotnet` toolchain and syntax-metadata smokes remain separate helpers.
The selection helper writes route-manifest outputs under
`target/tct-csharp-real-source-corpus/`.
