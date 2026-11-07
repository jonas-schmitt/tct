# BPE Merge Tables

This directory is the conventional location for trained Byte Pair Encoding
(BPE) merge tables that generated tokenizers load at build time. It ships
empty; train merge tables for your own schema and corpus, or use the
provenance-backed example artifacts under `../bpe/`.

## Format

Each merge table JSON file contains:

```json
{
  "schema": "github-workflow",
  "base_vocab_size": 257,
  "merges": [
    {
      "pair": [token1, token2],
      "new_token": merged_token_id,
      "frequency": occurrence_count
    }
  ],
  "metadata": {
    "trained_on_instances": 12345,
    "training_date": "2026-01-01",
    "max_merges": 1788,
    "dict_size": null
  }
}
```

`base_vocab_size` must match the generated tokenizer's `BASE_VOCAB_SIZE`, and
merge IDs are allocated contiguously above it. Legacy dictionary-compression
artifacts (`dict_size` greater than zero) are not supported.

## Training

Prepare plain TCT token sequences with `tct-prepare-data`, then train:

```bash
./target/release/tct-prepare-data --mode tct \
  --schema schemas/popular/github-workflow.json \
  --sources 'data/workflows/*.json' \
  --output target/prepared/github-workflow
./target/release/tct-train-bpe \
  --schema schemas/popular/github-workflow.json \
  --encoded target/prepared/github-workflow/all.jsonl \
  --target-vocab 2048 \
  -o schemas/bpe_merges/github-workflow-bpe2048.json
```

## Usage

```bash
./target/release/tct schemas/popular/github-workflow.json \
  --target python --bpe-merges schemas/bpe_merges/github-workflow-bpe2048.json
```

```rust
let table = tct::bpe::BpeMergeTable::from_file("schemas/bpe_merges/github-workflow-bpe2048.json")?;
```

See `DEVELOPMENT.md` for the full training and measurement workflow and the
JSON report contracts.
