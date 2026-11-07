#!/usr/bin/env python3
"""
TCT vs Tiktoken Comparison

Compares TCT's schema-aware tokenization with OpenAI's Tiktoken (o200k_base)
on a GitHub Actions workflow.
"""

from tct_github_workflow import encode, decode, vocab_size
import json

# Load sample workflow
with open('workflows/complex.json', 'r') as f:
    workflow = json.load(f)
    workflow_json = json.dumps(workflow)

print(f"Workflow: {workflow.get('name', 'N/A')}")
print(f"Size: {len(workflow_json)} chars\n")

# TCT encoding
tct_tokens = encode(workflow_json)
print(f"TCT:      {len(tct_tokens)} tokens ({vocab_size()} vocab)")

# Tiktoken encoding
try:
    import tiktoken
    enc = tiktoken.get_encoding("o200k_base")
    tk_tokens = enc.encode(workflow_json)
    print(f"Tiktoken: {len(tk_tokens)} tokens ({enc.n_vocab} vocab)")

    diff = len(tk_tokens) - len(tct_tokens)
    pct = abs(diff / len(tk_tokens)) * 100
    if diff > 0:
        print(f"\nTCT uses {diff} fewer tokens ({pct:.1f}% better)")
    else:
        print(f"\nTCT uses {abs(diff)} more tokens ({pct:.1f}% worse)")
except ImportError:
    print("Tiktoken not installed (pip install tiktoken)")

# Verify round-trip. decode returns (json, tokens_consumed, total_tokens); with BPE
# the counts are in expanded raw tokens, not in the compressed input length.
decoded_json, consumed, total = decode(tct_tokens)
decoded = json.loads(decoded_json)
print(f"\nRound-trip: {'✓ OK' if decoded == workflow else '✗ FAIL'}")
print(f"Expanded raw tokens consumed: {consumed}/{total}")
