#!/usr/bin/env python3
"""
Recover source file mapping for TCT-prepared datasets.

This script reconstructs which source file corresponds to which line
in the JSONL files, using the deterministic shuffle with seed 42.

Usage:
    python recover_file_mapping.py data tsconfig > mapping.jsonl
    python recover_file_mapping.py data eslintrc --line 100
"""

import argparse
import glob
import json
import random
from pathlib import Path


def recover_mapping(data_dir: Path, schema: str) -> list[dict]:
    """
    Recover the file-to-line mapping for a dataset.

    Returns list of dicts: [{"line": 0, "source": "/path/to/file.json"}, ...]
    """
    # Load base metadata to get source glob pattern
    base_dir = data_dir / f"{schema}-tct-base"
    base_meta_path = base_dir / "metadata.json"

    if not base_meta_path.exists():
        raise FileNotFoundError(f"Base metadata not found: {base_meta_path}")

    with open(base_meta_path) as f:
        base_meta = json.load(f)

    # Get source pattern and expand it
    source_pattern = base_meta["sources"][0]
    # Expand ~ in path
    source_pattern = str(Path(source_pattern).expanduser())

    # Collect files in sorted order (same as original encoding)
    files = sorted(glob.glob(source_pattern))

    if len(files) != base_meta["total_files"]:
        print(f"Warning: Found {len(files)} files, expected {base_meta['total_files']}")
        print(f"  Pattern: {source_pattern}")
        print("  File mapping may be incorrect if files were added/removed")

    # Apply first shuffle (base encoding, seed=42)
    random.seed(42)
    random.shuffle(files)

    # Apply second shuffle (BPE encoding, seed=42)
    # This is what tct-prepare-data does when using --encoded mode
    random.seed(42)
    random.shuffle(files)

    # Build mapping
    mapping = [{"line": i, "source": f} for i, f in enumerate(files)]

    return mapping


def main():
    parser = argparse.ArgumentParser(
        description="Recover source file mapping for TCT datasets"
    )
    parser.add_argument("data_dir", type=Path, help="Data directory (e.g., data)")
    parser.add_argument("schema", help="Schema name (tsconfig, eslintrc, kubernetes)")
    parser.add_argument("--line", type=int, help="Show mapping for specific line number")
    parser.add_argument("--source", help="Find line number for specific source file")
    parser.add_argument("--output", "-o", type=Path, help="Output mapping to file")
    args = parser.parse_args()

    mapping = recover_mapping(args.data_dir.expanduser(), args.schema)

    if args.line is not None:
        if 0 <= args.line < len(mapping):
            print(json.dumps(mapping[args.line]))
        else:
            print(f"Error: Line {args.line} out of range (0-{len(mapping)-1})")
            return 1

    elif args.source:
        # Find line for source file
        for entry in mapping:
            if args.source in entry["source"]:
                print(json.dumps(entry))
                return 0
        print(f"Source file not found: {args.source}")
        return 1

    elif args.output:
        with open(args.output, "w") as f:
            for entry in mapping:
                f.write(json.dumps(entry) + "\n")
        print(f"Wrote {len(mapping)} entries to {args.output}")

    else:
        # Print all mappings as JSONL
        for entry in mapping:
            print(json.dumps(entry))


if __name__ == "__main__":
    main()
