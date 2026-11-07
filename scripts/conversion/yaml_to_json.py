#!/usr/bin/env python3
"""
YAML to JSON Converter for TCT Testing

Converts YAML files (especially GitHub workflows) to JSON format
for use with TCT's JSON schema validation and round-trip testing.

Usage:
    python3 yaml_to_json.py input.yml output.json
    python3 yaml_to_json.py --url https://raw.github.com/.../workflow.yml output.json
"""

import sys
import json
import argparse
import urllib.request
from pathlib import Path

try:
    import yaml
except ImportError:
    print("Error: PyYAML is required. Install with: pip install pyyaml")
    sys.exit(1)


def convert_yaml_to_json(yaml_content: str, output_path: Path) -> bool:
    """Convert YAML content to JSON and save to file."""
    try:
        # Parse YAML
        data = yaml.safe_load(yaml_content)

        # Convert to JSON with pretty formatting
        json_content = json.dumps(data, indent=2, sort_keys=False)

        # Write to output file
        with open(output_path, 'w', encoding='utf-8') as f:
            f.write(json_content)

        return True

    except yaml.YAMLError as e:
        print(f"Error parsing YAML: {e}")
        return False
    except json.JSONEncodeError as e:
        print(f"Error encoding JSON: {e}")
        return False
    except IOError as e:
        print(f"Error writing file: {e}")
        return False


def download_and_convert(url: str, output_path: Path) -> bool:
    """Download YAML from URL and convert to JSON."""
    try:
        print(f"Downloading: {url}")
        with urllib.request.urlopen(url) as response:
            yaml_content = response.read().decode('utf-8')

        return convert_yaml_to_json(yaml_content, output_path)

    except urllib.error.URLError as e:
        print(f"Error downloading URL: {e}")
        return False


def main():
    parser = argparse.ArgumentParser(description="Convert YAML to JSON for TCT testing")
    parser.add_argument("input", help="Input YAML file path or URL (with --url)")
    parser.add_argument("output", help="Output JSON file path")
    parser.add_argument("--url", action="store_true", help="Treat input as URL")

    args = parser.parse_args()

    input_path = Path(args.input)
    output_path = Path(args.output)

    # Ensure output directory exists
    output_path.parent.mkdir(parents=True, exist_ok=True)

    success = False

    if args.url:
        # Download from URL
        success = download_and_convert(args.input, output_path)
    else:
        # Read from file
        if not input_path.exists():
            print(f"Error: Input file does not exist: {input_path}")
            return 1

        try:
            with open(input_path, 'r', encoding='utf-8') as f:
                yaml_content = f.read()

            success = convert_yaml_to_json(yaml_content, output_path)

        except IOError as e:
            print(f"Error reading file: {e}")
            return 1

    if success:
        print(f"✅ Successfully converted to: {output_path}")
        return 0
    else:
        print("❌ Conversion failed")
        return 1


if __name__ == "__main__":
    sys.exit(main())