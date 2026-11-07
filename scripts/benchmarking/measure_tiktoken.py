#!/usr/bin/env python3
"""
Measure tiktoken (GPT-4o) token counts on validation set.

This provides a baseline comparison to see how TCT's BPE compares
to OpenAI's production tokenizer.
"""

import json
import sys
from pathlib import Path
from typing import List

def measure_tiktoken_tokens(val_dir: Path, encoding_name: str = "o200k_base") -> dict:
    """
    Measure average tokens per instance using tiktoken.

    Args:
        val_dir: Path to validation directory
        encoding_name: Tiktoken encoding name (o200k_base for GPT-4o)

    Returns:
        Dictionary with statistics
    """
    # Load tiktoken encoding
    try:
        import tiktoken
        enc = tiktoken.get_encoding(encoding_name)
    except ImportError:
        print("ERROR: tiktoken not installed. Install with: pip install tiktoken")
        sys.exit(1)
    except Exception as e:
        print(f"ERROR: Failed to load encoding '{encoding_name}': {e}")
        print("Available encodings: cl100k_base (GPT-4), o200k_base (GPT-4o)")
        sys.exit(1)

    # Find all JSON files
    json_files = list(val_dir.glob("*.json"))
    if not json_files:
        print(f"ERROR: No JSON files found in {val_dir}")
        sys.exit(1)

    print(f"Processing {len(json_files)} validation instances...")

    total_tokens = 0
    instance_count = 0
    token_counts = []

    for json_file in json_files:
        try:
            # Read JSON file
            with open(json_file, 'r', encoding='utf-8') as f:
                content = f.read()

            # Encode with tiktoken
            tokens = enc.encode(content)
            token_count = len(tokens)

            total_tokens += token_count
            token_counts.append(token_count)
            instance_count += 1

            if instance_count % 1000 == 0:
                print(f"  Processed {instance_count} instances...")

        except Exception as e:
            print(f"WARNING: Failed to process {json_file}: {e}")
            continue

    if instance_count == 0:
        print("ERROR: No instances were successfully processed")
        sys.exit(1)

    avg_tokens = total_tokens / instance_count
    min_tokens = min(token_counts)
    max_tokens = max(token_counts)

    # Calculate percentiles
    sorted_counts = sorted(token_counts)
    p50 = sorted_counts[len(sorted_counts) // 2]
    p90 = sorted_counts[int(len(sorted_counts) * 0.9)]
    p99 = sorted_counts[int(len(sorted_counts) * 0.99)]

    return {
        "encoding": encoding_name,
        "instance_count": instance_count,
        "total_tokens": total_tokens,
        "avg_tokens_per_instance": avg_tokens,
        "min_tokens": min_tokens,
        "max_tokens": max_tokens,
        "median_tokens": p50,
        "p90_tokens": p90,
        "p99_tokens": p99,
        "vocab_size": enc.n_vocab,
    }


def main():
    import argparse

    # Parse command line arguments
    parser = argparse.ArgumentParser(description="Measure tiktoken token counts on validation set")
    parser.add_argument("--val-dir", type=Path, default=Path("data/dict-analysis/val"),
                        help="Path to validation directory")
    parser.add_argument("--output-dir", type=Path, default=Path("target/tct-tiktoken"),
                        help="Path to output directory")
    parser.add_argument("--encoding", type=str, default="o200k_base",
                        help="Tiktoken encoding name (o200k_base for GPT-4o)")
    args = parser.parse_args()

    val_dir = args.val_dir

    if not val_dir.exists():
        print(f"ERROR: Validation directory not found: {val_dir}")
        sys.exit(1)

    print("=" * 60)
    print("Tiktoken (GPT-4o) Token Count Measurement")
    print("=" * 60)
    print()

    # Measure with specified encoding
    stats = measure_tiktoken_tokens(val_dir, encoding_name=args.encoding)

    print()
    print("=" * 60)
    print("Results")
    print("=" * 60)
    print(f"Encoding:              {stats['encoding']}")
    print(f"Vocabulary Size:       {stats['vocab_size']:,} tokens")
    print(f"Instances:             {stats['instance_count']:,}")
    print(f"Total Tokens:          {stats['total_tokens']:,}")
    print(f"Avg Tokens/Instance:   {stats['avg_tokens_per_instance']:.2f}")
    print(f"Min Tokens:            {stats['min_tokens']}")
    print(f"Max Tokens:            {stats['max_tokens']}")
    print(f"Median (P50):          {stats['median_tokens']}")
    print(f"P90:                   {stats['p90_tokens']}")
    print(f"P99:                   {stats['p99_tokens']}")
    print()

    # Save results to CSV
    output_file = args.output_dir / "tiktoken_results.csv"
    output_file.parent.mkdir(parents=True, exist_ok=True)

    with open(output_file, 'w') as f:
        f.write("encoding,vocab_size,instance_count,avg_tokens_per_instance\n")
        f.write(f"{stats['encoding']},{stats['vocab_size']},{stats['instance_count']},{stats['avg_tokens_per_instance']:.2f}\n")

    print(f"Results saved to: {output_file}")

    # Also save detailed JSON
    json_output = output_file.with_suffix('.json')
    with open(json_output, 'w') as f:
        json.dump(stats, f, indent=2)

    print(f"Detailed stats saved to: {json_output}")


if __name__ == "__main__":
    main()
