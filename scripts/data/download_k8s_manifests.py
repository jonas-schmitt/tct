#!/usr/bin/env python3
"""
Download ALL Kubernetes manifests from substratusai/the-stack-yaml-k8s dataset.

By default, collects every manifest except those that carry credentials
(Kubernetes Secrets, see carries_credentials). Use --max-string-length to
filter manifests with long strings.

Expected dataset size: ~300k manifests
"""

import argparse
import hashlib
import json
import re
import sys
from pathlib import Path


def carries_credentials(manifest):
    """True for a Secret, or a list wrapping one: their data fields hold credentials."""
    if not isinstance(manifest, dict):
        return False
    if manifest.get("kind") in ("Secret", "SecretList"):
        return True
    items = manifest.get("items")
    return isinstance(items, list) and any(carries_credentials(item) for item in items)


def manifest_filename(kind, index):
    """File name for a manifest: its kind reduced to safe characters plus a running index.

    The kind comes from the downloaded YAML, so it must never be able to name a
    path outside the output directory.
    """
    stem = re.sub(r"[^a-z0-9_-]+", "_", str(kind).lower()).strip("_") or "unknown"
    return f"{stem}_{index:06d}.json"


def get_manifest_hash(manifest):
    """Generate a content hash for deduplication."""
    content = json.dumps(manifest, sort_keys=True)
    return hashlib.sha256(content.encode()).hexdigest()[:16]


def load_existing_hashes(directories):
    """
    Load content hashes from already-collected manifests.

    Args:
        directories: List of directories containing existing manifests

    Returns:
        Set of content hashes
    """
    hashes = set()
    for dir_path in directories:
        if not dir_path.exists():
            continue
        for json_file in dir_path.glob("*.json"):
            try:
                with open(json_file) as f:
                    manifest = json.load(f)
                    hashes.add(get_manifest_hash(manifest))
            except Exception:
                pass  # Skip files that can't be read
    return hashes


def main():
    parser = argparse.ArgumentParser(description="Download Kubernetes manifests")
    parser.add_argument("--output", type=Path, default=Path("data/k8s-manifests"),
                        help="Output directory for new manifests")
    parser.add_argument("--max-string-length", type=int, default=None,
                        help="Maximum string length filter (default: no filter)")
    parser.add_argument("--skip-existing", type=Path, nargs="*", default=[],
                        help="Directories with existing manifests to skip (for deduplication)")
    args = parser.parse_args()

    # Configuration
    output_dir = args.output
    output_dir.mkdir(parents=True, exist_ok=True)

    print(f"📦 Downloading Kubernetes manifests from Hugging Face...")
    print(f"   Dataset: substratusai/the-stack-yaml-k8s (~300k manifests)")
    print(f"   Filter: Secrets are never written")
    if args.max_string_length:
        print(f"   Filter: max string length = {args.max_string_length} chars")
    print(f"   Output: {output_dir}")

    # Load existing hashes for deduplication
    existing_hashes = set()
    if args.skip_existing:
        print(f"   Dedup: Loading hashes from {len(args.skip_existing)} directories...")
        existing_hashes = load_existing_hashes(args.skip_existing)
        print(f"   Dedup: Loaded {len(existing_hashes)} existing manifest hashes")
    print()

    # Load dataset (streaming mode to avoid downloading entire dataset)
    print("🔄 Loading dataset stream...")
    try:
        from datasets import load_dataset
    except ImportError:
        print("Error: the datasets library is required. Install with: pip install datasets")
        sys.exit(1)
    dataset = load_dataset("substratusai/the-stack-yaml-k8s", split="train", streaming=True)

    # Download manifests
    import yaml
    collected = 0
    processed = 0
    skipped_parse_error = 0
    skipped_string_length = 0
    skipped_invalid = 0
    skipped_duplicate = 0
    skipped_credentials = 0
    kinds = {}
    seen_hashes = set(existing_hashes)  # Include existing hashes in dedup

    for i, example in enumerate(dataset):
        processed += 1

        # Each example should have 'content' field with YAML
        content = example.get('content', '')
        if not content:
            skipped_invalid += 1
            continue

        try:
            # Parse YAML to JSON
            manifest = yaml.safe_load(content)

            if not manifest or not isinstance(manifest, dict):
                skipped_invalid += 1
                continue

            if carries_credentials(manifest):
                skipped_credentials += 1
                continue

            # Check for duplicates
            content_hash = get_manifest_hash(manifest)
            if content_hash in seen_hashes:
                skipped_duplicate += 1
                continue
            seen_hashes.add(content_hash)

            # Optional string length filter (disabled by default)
            if args.max_string_length:
                def check_string_lengths(obj, max_length, path=""):
                    if isinstance(obj, dict):
                        for key, value in obj.items():
                            valid, reason = check_string_lengths(value, max_length, f"{path}.{key}" if path else key)
                            if not valid:
                                return False, reason
                    elif isinstance(obj, list):
                        for i, item in enumerate(obj):
                            valid, reason = check_string_lengths(item, max_length, f"{path}[{i}]")
                            if not valid:
                                return False, reason
                    elif isinstance(obj, str) and len(obj) > max_length:
                        return False, f"String at {path} exceeds {max_length} chars (length: {len(obj)})"
                    return True, ""

                valid, reason = check_string_lengths(manifest, args.max_string_length)
                if not valid:
                    skipped_string_length += 1
                    if skipped_string_length <= 10:
                        print(f"  ⚠️  Skipped (string too long): {reason}", file=sys.stderr)
                    continue

            # Get kind for categorization
            kind = manifest.get('kind', 'Unknown')
            if not isinstance(kind, str):
                skipped_invalid += 1
                continue
            kinds[kind] = kinds.get(kind, 0) + 1

            # Save as JSON
            output_path = output_dir / manifest_filename(kind, collected)

            with open(output_path, 'w') as f:
                json.dump(manifest, f, indent=2)

            collected += 1

            # Progress reporting
            if collected % 1000 == 0:
                print(f"  ✓ Collected {collected:,} new manifests (processed {processed:,}, skipped {skipped_duplicate:,} dupes)...")
            elif collected % 100 == 0 and collected < 1000:
                print(f"  ✓ Collected {collected} new manifests...")

        except yaml.YAMLError as e:
            skipped_parse_error += 1
            if skipped_parse_error <= 10:
                print(f"  ⚠️  Skipped (YAML parse error): {e}", file=sys.stderr)
            continue
        except Exception as e:
            skipped_invalid += 1
            if skipped_invalid <= 10:
                print(f"  ⚠️  Skipped (error): {e}", file=sys.stderr)
            continue

    print()
    print(f"✅ Downloaded {collected:,} NEW Kubernetes manifests")
    print()
    print("📊 Statistics:")
    print(f"  - Total processed: {processed:,}")
    print(f"  - NEW manifests collected: {collected:,}")
    print(f"  - Skipped (duplicate): {skipped_duplicate:,}")
    print(f"  - Skipped (parse error): {skipped_parse_error:,}")
    print(f"  - Skipped (carries credentials): {skipped_credentials:,}")
    if args.max_string_length:
        print(f"  - Skipped (string too long): {skipped_string_length:,}")
    print(f"  - Skipped (invalid): {skipped_invalid:,}")
    print()
    print("📊 Manifest types:")
    for kind, count in sorted(kinds.items(), key=lambda x: -x[1])[:20]:
        print(f"  - {kind}: {count:,}")
    if len(kinds) > 20:
        print(f"  ... and {len(kinds) - 20} more types")
    print()
    print(f"Output directory: {output_dir}")

    return 0


if __name__ == "__main__":
    sys.exit(main())
