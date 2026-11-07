#!/usr/bin/env python3
"""
Clean workflow corpus by removing non-encodable instances and duplicates.

This script:
1. Loads all workflows from specified directories
2. Tests each workflow for TCT encodability
3. Removes duplicates by content hash
4. Saves exactly the target number of clean, encodable workflows
"""

import argparse
import json
import hashlib
import subprocess
import sys
import tempfile
import shutil
from pathlib import Path
from typing import Dict, List, Set, Tuple


def hash_workflow(workflow: Dict) -> str:
    """Generate content hash for duplicate detection."""
    content = json.dumps(workflow, sort_keys=True)
    return hashlib.sha256(content.encode()).hexdigest()


def test_encodability(workflow_path: Path, schema_path: Path, timeout: int = 30) -> Tuple[bool, str]:
    """
    Test if a workflow can be encoded with TCT.

    Returns:
        (success: bool, error_message: str)
    """
    try:
        # Use tct-measure-tokens to test encodability
        result = subprocess.run(
            [
                "target/release/tct-measure-tokens",
                "--schema", str(schema_path),
                "--instances", str(workflow_path)
            ],
            capture_output=True,
            text=True,
            timeout=timeout
        )

        # Check if encoding succeeded
        if result.returncode == 0:
            return True, ""
        else:
            # Extract error message
            error = result.stderr.strip() if result.stderr else result.stdout.strip()
            return False, error

    except subprocess.TimeoutExpired:
        return False, f"Encoding timeout ({timeout}s)"
    except Exception as e:
        return False, f"Encoding error: {str(e)}"


def load_workflows(directories: List[Path]) -> List[Tuple[Path, Dict]]:
    """Load all workflow JSON files from specified directories."""
    workflows = []

    for directory in directories:
        if not directory.exists():
            print(f"Warning: Directory {directory} does not exist, skipping")
            continue

        json_files = list(directory.glob("*.json"))
        print(f"Loading {len(json_files)} workflows from {directory}...")

        for json_file in json_files:
            try:
                with open(json_file, 'r', encoding='utf-8') as f:
                    workflow = json.load(f)
                workflows.append((json_file, workflow))
            except Exception as e:
                print(f"Warning: Failed to load {json_file}: {e}")

    return workflows


def clean_corpus(
    source_dirs: List[Path],
    output_dir: Path,
    schema_path: Path,
    target_count: int,
    timeout: int = 30
) -> None:
    """
    Clean workflow corpus ensuring all are encodable and deduplicated.

    Args:
        source_dirs: List of directories containing workflow JSON files
        output_dir: Directory to save cleaned workflows
        schema_path: Path to JSON schema for validation
        target_count: Target number of clean workflows
        timeout: Timeout for encoding test (seconds)
    """
    print("=" * 80)
    print("TCT Workflow Corpus Cleaning")
    print("=" * 80)
    print(f"Target count: {target_count}")
    print(f"Schema: {schema_path}")
    print(f"Output: {output_dir}")
    print()

    # Create output directory
    output_dir.mkdir(parents=True, exist_ok=True)

    # Load all workflows
    print("Step 1: Loading workflows...")
    all_workflows = load_workflows(source_dirs)
    print(f"Loaded {len(all_workflows)} total workflows")
    print()

    # Deduplicate by content hash
    print("Step 2: Removing duplicates...")
    seen_hashes: Set[str] = set()
    unique_workflows: List[Tuple[Path, Dict]] = []

    for path, workflow in all_workflows:
        workflow_hash = hash_workflow(workflow)
        if workflow_hash not in seen_hashes:
            seen_hashes.add(workflow_hash)
            unique_workflows.append((path, workflow))

    print(f"Removed {len(all_workflows) - len(unique_workflows)} duplicates")
    print(f"Unique workflows: {len(unique_workflows)}")
    print()

    # Test encodability
    print("Step 3: Testing encodability...")
    print("This may take a while...")

    encodable_workflows: List[Tuple[Path, Dict]] = []
    failed_workflows: List[Tuple[Path, str]] = []

    for i, (path, workflow) in enumerate(unique_workflows):
        if i % 100 == 0:
            print(f"Progress: {i}/{len(unique_workflows)} tested "
                  f"({len(encodable_workflows)} encodable, {len(failed_workflows)} failed)")

        # Save to temporary file for testing
        with tempfile.NamedTemporaryFile(mode='w', suffix='.json', delete=False) as tmp_file:
            json.dump(workflow, tmp_file)
            tmp_path = Path(tmp_file.name)

        try:
            success, error = test_encodability(tmp_path, schema_path, timeout)
            if success:
                encodable_workflows.append((path, workflow))
            else:
                failed_workflows.append((path, error))
        finally:
            tmp_path.unlink()

    print()
    print(f"Encodability test complete:")
    print(f"  Encodable: {len(encodable_workflows)}")
    print(f"  Failed: {len(failed_workflows)}")
    print()

    # Check if we have enough encodable workflows
    if len(encodable_workflows) < target_count:
        print(f"ERROR: Only {len(encodable_workflows)} encodable workflows found, "
              f"but need {target_count}")
        print(f"Please collect {target_count - len(encodable_workflows)} more workflows")
        sys.exit(1)

    # Save exactly target_count workflows
    print(f"Step 4: Saving {target_count} clean workflows to {output_dir}...")

    for i, (original_path, workflow) in enumerate(encodable_workflows[:target_count]):
        output_file = output_dir / f"workflow_{i:05d}.json"
        with open(output_file, 'w', encoding='utf-8') as f:
            json.dump(workflow, f, indent=2)

    print(f"Successfully saved {target_count} clean workflows")

    # Save failure report
    if failed_workflows:
        failure_report = output_dir / "failed_workflows.jsonl"
        print(f"Saving failure report to {failure_report}...")
        with open(failure_report, 'w', encoding='utf-8') as f:
            for path, error in failed_workflows:
                f.write(json.dumps({
                    "path": str(path),
                    "error": error
                }) + "\n")

    print()
    print("=" * 80)
    print("Corpus cleaning complete!")
    print("=" * 80)
    print(f"Clean workflows: {target_count}")
    print(f"Location: {output_dir}")


def main():
    parser = argparse.ArgumentParser(
        description="Clean workflow corpus for TCT training/validation",
        formatter_class=argparse.RawDescriptionHelpFormatter,
        epilog="""
Examples:
  # Clean corpus from multiple directories
  python3 scripts/data/clean_corpus.py \\
    --sources data/github-workflows/json data/github-workflows-new \\
    --output data/workflows-clean \\
    --schema schemas/popular/github-workflow.json \\
    --target 20000

  # Quick test with smaller corpus
  python3 scripts/data/clean_corpus.py \\
    --sources data/github-workflows/json-1k \\
    --output data/workflows-test \\
    --schema schemas/popular/github-workflow.json \\
    --target 100
        """
    )

    parser.add_argument(
        "--sources",
        nargs="+",
        required=True,
        help="Source directories containing workflow JSON files"
    )
    parser.add_argument(
        "--output",
        required=True,
        help="Output directory for cleaned workflows"
    )
    parser.add_argument(
        "--schema",
        required=True,
        help="Path to JSON schema for validation"
    )
    parser.add_argument(
        "--target",
        type=int,
        default=20000,
        help="Target number of clean workflows (default: 20000)"
    )
    parser.add_argument(
        "--timeout",
        type=int,
        default=30,
        help="Timeout for encoding test in seconds (default: 30)"
    )

    args = parser.parse_args()

    # Convert paths
    source_dirs = [Path(d) for d in args.sources]
    output_dir = Path(args.output)
    schema_path = Path(args.schema)

    # Validate schema exists
    if not schema_path.exists():
        print(f"ERROR: Schema file not found: {schema_path}")
        sys.exit(1)

    # Run cleaning
    clean_corpus(
        source_dirs=source_dirs,
        output_dir=output_dir,
        schema_path=schema_path,
        target_count=args.target,
        timeout=args.timeout
    )


if __name__ == "__main__":
    main()
