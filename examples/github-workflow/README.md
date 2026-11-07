# TCT GitHub Workflow Tokenizer

Schema-aware tokenizer for GitHub Actions workflows with BPE compression.

## Installation

Build the wheel from the schema with the TCT compiler, then install it:

```bash
cargo build --release
./target/release/tct schemas/popular/github-workflow.json \
  --target python --python-name tct_github_workflow \
  --bpe-merges schemas/bpe/github-workflow-tct-bpe-500.json --streaming
pip install wheels/tct_github_workflow/target/wheels/tct_github_workflow-*.whl
pip install -r requirements.txt
```

**Requirements**: a Python version and platform supported by your local
`maturin` build (see `DEVELOPMENT.md`, "Python Wheels").

## Quick Start

```python
from tct_github_workflow import encode, decode, vocab_size
import json

workflow = {
    "name": "CI",
    "on": "push",
    "jobs": {
        "build": {
            "runs-on": "ubuntu-latest",
            "steps": [{"uses": "actions/checkout@v4"}]
        }
    }
}

# Encode
tokens = encode(json.dumps(workflow))
print(f"Tokens: {len(tokens)} (vocab: {vocab_size()})")

# Decode
decoded_json, consumed, total = decode(tokens)
decoded = json.loads(decoded_json)
assert decoded == workflow
```

## API Reference

### Core Functions

```python
encode(json_str: str) -> List[int]
decode(tokens: List[int]) -> Tuple[str, int, int]  # (json, tokens_consumed, total_tokens);
                                                    # counts are in BPE-expanded raw tokens
vocab_size() -> int  # Raw tokens plus BPE merges plus PAD
pad_token() -> int   # Returns PAD token ID
```

## Example Scripts

Run these to see TCT in action:

```bash
python3 example_quickstart.py         # Minimal example
python3 example_vs_tiktoken.py        # Compare with Tiktoken
```

## Compression Architecture

TCT uses BPE on top of schema-aware base encoding:

1. **Schema-aware base encoding**
   - Encodes structure, not field names as strings

2. **BPE compression**
   - Merges trained on plain TCT token sequences

**Vocabulary**: Base TCT tokens + BPE merges + PAD.

## Performance

Schema-aware base encoding removes structural redundancy (field names,
punctuation, enum spellings), and BPE merges can compress repeated value-byte
sequences while preserving a lossless round trip. Token counts depend on the
schema, corpus, and trained merges. Measure your corpus with
`scripts/benchmarking/run-reproducible-tokenizer-benchmark.sh` and report the
comparison tokenizer and configuration alongside the results.

## Tokenizer Files

Tokenizer deployments should use BPE-only artifacts trained on plain TCT token
sequences. Legacy dictionary-compression artifacts are not supported.

**Generating a new wheel**: see the Installation section above; pass a
different `--bpe-merges` table to change the compression vocabulary.

## Troubleshooting

### Import Error
```bash
pip install --force-reinstall wheels/tct_github_workflow/target/wheels/tct_github_workflow-*.whl
```

### Schema Validation Error
Ensure the workflow has the required fields `on` and `jobs`, with `jobs` an
object keyed by job id.
