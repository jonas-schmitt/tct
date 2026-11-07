#!/usr/bin/env python3
"""
TCT Quick Start Example

Demonstrates TCT tokenization on a real GitHub Actions workflow
from Aider-AI (popular AI coding assistant).
"""

import json
from tct_github_workflow import decode, encode, pad_token, sequence_metadata_json, vocab_size

# Real workflow from Aider-AI/aider repository
workflow = {
    "name": "Docker Build Test",
    "on": {"push": {"branches": ["main"]}, "pull_request": {}},
    "jobs": {
        "docker_build_and_push": {
            "runs-on": "ubuntu-latest",
            "steps": [
                {"name": "Checkout code", "uses": "actions/checkout@v4"},
                {"name": "Set up Docker Buildx", "uses": "docker/setup-buildx-action@v3"},
                {
                    "name": "Build Docker images",
                    "uses": "docker/build-push-action@v5",
                    "with": {
                        "context": ".",
                        "file": "./docker/Dockerfile",
                        "platforms": "linux/amd64,linux/arm64",
                        "push": True,
                        "tags": "aider:latest"
                    }
                }
            ]
        }
    }
}

# Convert to JSON string
workflow_json = json.dumps(workflow)

# Encode
tokens = encode(workflow_json)
print(tokens[0:])


# Analyze
print(f"Workflow size: {len(workflow_json)} characters")
print(f"Token count: {len(tokens)} tokens")
print(f"Compression: {len(workflow_json) / len(tokens):.2f} chars/token")
print(f"Vocabulary: {vocab_size()} tokens")
bpe_range = json.loads(sequence_metadata_json())["raw_bpe_tokens"]
bpe_tokens = sum(1 for t in tokens if bpe_range["start"] <= t < bpe_range["end_exclusive"])
print(f"BPE usage: {bpe_tokens / len(tokens) * 100:.1f}% of tokens are merges")

# Test round-trip. decode returns (json, tokens_consumed, total_tokens); with BPE
# the counts are in expanded raw tokens, not in the compressed input length.
decoded_json, consumed, total = decode(tokens)
assert json.loads(decoded_json) == workflow
print(f"✅ Round-trip successful! (consumed {consumed}/{total} tokens)")

# Test surplus detection
print(f"\nTesting surplus detection...")
# Add surplus tokens at the END (not corrupting the valid structure)
tokens_with_surplus = tokens + [pad_token()] * 10
decoded_surplus, consumed, total_with_surplus = decode(tokens_with_surplus)
print(f"Decoded with surplus: consumed {consumed}/{total_with_surplus} tokens")
if consumed < total_with_surplus:
    print(f"⚠️  Detected {total_with_surplus - consumed} surplus tokens!")
else:
    print(f"✅ All tokens consumed (no surplus)")