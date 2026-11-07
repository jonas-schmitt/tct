#!/usr/bin/env python3
"""
Convert TOML files to JSON format for TCT testing.
Usage: python3 toml_to_json.py input.toml output.json
"""

import sys
import json
import tomllib
from pathlib import Path

def convert_toml_to_json(toml_path: str, json_path: str) -> bool:
    """Convert a TOML file to JSON format."""
    try:
        # Read TOML file
        with open(toml_path, 'rb') as f:
            toml_data = tomllib.load(f)

        # Write JSON file
        with open(json_path, 'w') as f:
            json.dump(toml_data, f, indent=2)

        print(f"Successfully converted {toml_path} -> {json_path}")
        return True

    except Exception as e:
        print(f"Error converting {toml_path}: {e}")
        return False

def main():
    if len(sys.argv) != 3:
        print("Usage: python3 toml_to_json.py input.toml output.json")
        sys.exit(1)

    toml_path = sys.argv[1]
    json_path = sys.argv[2]

    if not Path(toml_path).exists():
        print(f"Error: Input file {toml_path} does not exist")
        sys.exit(1)

    # Create output directory if needed
    Path(json_path).parent.mkdir(parents=True, exist_ok=True)

    success = convert_toml_to_json(toml_path, json_path)
    sys.exit(0 if success else 1)

if __name__ == "__main__":
    main()