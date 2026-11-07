#!/usr/bin/env python3
"""
Adaptive Multi-Pass GitHub Workflow Collection

Collects 100K+ workflows by adaptively discovering star thresholds.
Each pass collects from top 1,000 repos, then uses the lowest star count
as the upper threshold for the next pass.

Features:
- Automatic checkpoint/resume: Can be safely interrupted and resumed
- Adaptive thresholds: Discovers optimal star ranges automatically
- Progress tracking: Detailed statistics for each pass

Usage:
    GITHUB_TOKEN=ghp_... python3 collect_workflows_adaptive.py --target 100000
"""

import argparse
import json
import os
import subprocess
import sys
from datetime import datetime
from pathlib import Path


def save_checkpoint(checkpoint_file: Path, pass_num: int, total_workflows: int,
                    target: int, next_min: int, next_max: int, last_pass_stats: dict):
    """Save checkpoint to allow resume after interruption."""
    checkpoint = {
        "version": 1,
        "timestamp": datetime.utcnow().isoformat() + "Z",
        "pass_num": pass_num,
        "total_workflows": total_workflows,
        "target_workflows": target,
        "next_pass": {
            "min_stars": next_min,
            "max_stars": next_max
        },
        "last_pass_stats": last_pass_stats
    }

    with open(checkpoint_file, 'w') as f:
        json.dump(checkpoint, f, indent=2)

    print(f"💾 Checkpoint saved: Pass {pass_num} complete, {total_workflows} workflows collected")


def load_checkpoint(checkpoint_file: Path) -> dict:
    """Load checkpoint if exists, otherwise return None."""
    if not checkpoint_file.exists():
        return None

    try:
        with open(checkpoint_file) as f:
            checkpoint = json.load(f)

        print("📂 Found checkpoint from previous run:")
        print(f"   Last completed pass: {checkpoint['pass_num']}")
        print(f"   Total workflows: {checkpoint['total_workflows']}")
        print(f"   Next star range: {checkpoint['next_pass']['min_stars']}", end="")
        if checkpoint['next_pass']['max_stars'] is not None:
            print(f"-{checkpoint['next_pass']['max_stars']}")
        else:
            print("+")

        return checkpoint
    except Exception as e:
        print(f"⚠️  Failed to load checkpoint: {e}")
        return None


def save_pass_history(history_file: Path, pass_num: int, stats: dict):
    """Append pass statistics to history file."""
    entry = {
        "timestamp": datetime.utcnow().isoformat() + "Z",
        "pass_num": pass_num,
        **stats
    }

    with open(history_file, 'a') as f:
        f.write(json.dumps(entry) + '\n')


def read_last_pass_from_history(history_file: Path) -> dict:
    """Read the last VALID adaptive pass from pass_history.jsonl.

    Skips corrupted entries with:
    - star_range = [0, 0] (failed/corrupted passes)
    - workflows_collected = 0 (unsuccessful passes)
    - Invalid star ranges (high < low, negative values, etc.)
    - Too broad ranges (width >= 1000, not truly adaptive)
    """
    if not history_file.exists():
        return None

    last_valid_pass = None
    with open(history_file) as f:
        for line in f:
            if not line.strip():
                continue

            entry = json.loads(line)
            star_range = entry.get('star_range', [0, 0])
            collected = entry.get('workflows_collected', 0)

            # Skip invalid entries
            if star_range == [0, 0]:
                continue  # Corrupted entry
            if collected == 0:
                continue  # Failed pass
            if len(star_range) != 2:
                continue  # Malformed
            if star_range[0] < star_range[1]:
                continue  # Invalid range (high should be >= low)

            # Valid adaptive entry found (accept ALL valid ranges - don't skip any!)
            last_valid_pass = entry

    return last_valid_pass


def run_collection_pass(
    min_stars: int,
    max_stars: int,
    limit: int,
    output_dir: Path,
    schema_path: Path,
    resume: bool = True
) -> dict:
    """
    Run a single collection pass and return statistics.

    Returns:
        dict with keys: collected, lowest_stars, highest_stars
    """
    cmd = [
        "python3", "scripts/data/collect_workflows.py",
        "--limit", str(limit),
        "--min-stars", str(min_stars),
        "--output", str(output_dir),
        "--schema", str(schema_path)
    ]

    if max_stars is not None:
        cmd.extend(["--max-stars", str(max_stars)])

    if resume:
        cmd.append("--resume")

    print(f"\n🚀 Running collection pass: min_stars={min_stars}, max_stars={max_stars}")
    print(f"   Command: {' '.join(cmd)}\n")

    # Run collection
    result = subprocess.run(cmd, capture_output=False, text=True)

    if result.returncode != 0:
        print(f"⚠️  Collection pass failed with return code {result.returncode}")
        return None

    # Read metadata to find lowest star count from this pass
    metadata_file = output_dir / "metadata.jsonl"
    if not metadata_file.exists():
        print("⚠️  No metadata file found")
        return None

    # Read metadata to find star range from this pass
    pass_repos = set()
    lowest_stars = float('inf')
    highest_stars = 0
    collected_this_pass = 0

    with open(metadata_file) as f:
        for line in f:
            if not line.strip():
                continue
            entry = json.loads(line)
            repo = entry['repo']
            stars = entry['stars']

            # Only count repos that match our current pass criteria
            if max_stars is not None:
                if stars >= min_stars and stars <= max_stars and repo not in pass_repos:
                    pass_repos.add(repo)
                    collected_this_pass += 1
                    lowest_stars = min(lowest_stars, stars)
                    highest_stars = max(highest_stars, stars)
            else:
                if stars >= min_stars and repo not in pass_repos:
                    pass_repos.add(repo)
                    collected_this_pass += 1
                    lowest_stars = min(lowest_stars, stars)
                    highest_stars = max(highest_stars, stars)

    if lowest_stars == float('inf'):
        lowest_stars = 0

    return {
        'collected': collected_this_pass,
        'lowest_stars': lowest_stars,
        'highest_stars': highest_stars,
        'unique_repos': len(pass_repos)
    }


def main():
    parser = argparse.ArgumentParser(
        description="Adaptive multi-pass workflow collection"
    )
    parser.add_argument(
        "--target",
        type=int,
        default=100000,
        help="Target number of workflows to collect (default: 100000)"
    )
    parser.add_argument(
        "--output",
        type=Path,
        default=Path("data/workflows-100k"),
        help="Output directory (default: data/workflows-100k)"
    )
    parser.add_argument(
        "--schema",
        type=Path,
        default=Path("schemas/popular/github-workflow.json"),
        help="Schema path (default: schemas/popular/github-workflow.json)"
    )
    parser.add_argument(
        "--workflows-per-pass",
        type=int,
        default=15000,
        help="Workflow limit per pass (default: 15000)"
    )
    parser.add_argument(
        "--resume-from-stars",
        type=int,
        default=999999,
        help="Resume adaptive search from this star count as upper bound (default: 999999 for fresh start)"
    )
    parser.add_argument(
        "--start-phase",
        type=int,
        choices=[1, 2],
        default=1,
        help="Starting phase: 1 (star-based repo search) or 2 (language-filtered code search)"
    )

    args = parser.parse_args()

    # Check for GitHub token
    if not os.getenv("GITHUB_TOKEN"):
        print("❌ Error: GITHUB_TOKEN environment variable not set")
        print("Create a token at: https://github.com/settings/tokens")
        print("Required scopes: public_repo (read-only)")
        return 1

    # Validate schema exists
    if not args.schema.exists():
        print(f"❌ Error: Schema file not found: {args.schema}")
        return 1

    # Setup checkpoint and history files
    checkpoint_file = args.output / "checkpoint.json"
    history_file = args.output / "pass_history.jsonl"

    # Check for previous work in pass history
    last_historical_pass = read_last_pass_from_history(history_file)

    # Check for checkpoint (mid-pass interruption)
    checkpoint = load_checkpoint(checkpoint_file)

    # Check current count from metadata
    current_count = 0
    metadata_file = args.output / "metadata.jsonl"
    if metadata_file.exists():
        with open(metadata_file) as f:
            current_count = sum(1 for line in f if line.strip())

    print("=" * 70)
    print("🎯 Adaptive Multi-Pass Collection Strategy")
    print("=" * 70)
    print(f"Current workflows: {current_count}")
    print(f"Target workflows:  {args.target}")
    print(f"Remaining needed:  {max(0, args.target - current_count)}")
    print(f"Workflows/pass:    {args.workflows_per_pass}")
    print("=" * 70)

    if current_count >= args.target:
        print(f"✅ Target already reached! ({current_count} >= {args.target})")
        return 0

    # Check if we should skip directly to Phase 2
    if args.start_phase == 2:
        print("\n⏩ Skipping Phase 1 (star-based search), starting directly at Phase 2 (language search)")
        # Jump directly to Phase 2 - pass_num will be incremented to 1 in first language loop
        pass_num = 0
    # Resume from checkpoint (mid-pass interruption)
    elif checkpoint and checkpoint['total_workflows'] < args.target:
        print("\n🔄 Resuming from checkpoint (mid-pass interruption)...")
        pass_num = checkpoint['pass_num'] + 1
        min_stars = checkpoint['next_pass']['min_stars']
        max_stars = checkpoint['next_pass']['max_stars']
        current_count = checkpoint['total_workflows']
    # Resume from pass history (previous work exists)
    elif last_historical_pass and current_count < args.target:
        print(f"\n🔄 Resuming from pass history (Pass {last_historical_pass['pass_num']})...")
        print(f"   Last pass searched: {last_historical_pass['star_range'][0]}-{last_historical_pass['star_range'][1]} stars")
        pass_num = last_historical_pass['pass_num'] + 1
        min_stars = 0
        max_stars = last_historical_pass['star_range'][1] - 1  # Continue from lowest-1
        print(f"   Resuming with: 0-{max_stars} stars")
    # Fresh start
    else:
        print("\n🆕 Starting fresh collection...")
        pass_num = 1
        min_stars = 0
        max_stars = args.resume_from_stars

    while current_count < args.target and args.start_phase == 1:
        print(f"\n{'=' * 70}")
        print(f"📊 Pass {pass_num}")
        print(f"{'=' * 70}")
        print(f"Star range: {min_stars}{f'-{max_stars}' if max_stars else '+'}")
        print(f"Current total: {current_count} workflows")
        print(f"Target for this pass: {args.workflows_per_pass} workflows")

        # Validate star range BEFORE API call to prevent invalid queries
        if min_stars < 0:
            print(f"\n❌ ERROR: Invalid min_stars={min_stars} (must be >= 0)")
            print("    Star-based search exhausted, switching to Phase 2")
            break

        if max_stars is not None and max_stars < 0:
            print(f"\n❌ ERROR: Invalid max_stars={max_stars} (must be >= 0)")
            print("    Star-based search exhausted, switching to Phase 2")
            break

        if max_stars is not None and min_stars > max_stars:
            print(f"\n❌ ERROR: Invalid range {min_stars}-{max_stars} (min > max)")
            print("    Star-based search exhausted, switching to Phase 2")
            break

        # Run collection pass
        stats = run_collection_pass(
            min_stars=min_stars,
            max_stars=max_stars,
            limit=args.workflows_per_pass,
            output_dir=args.output,
            schema_path=args.schema,
            resume=True
        )

        if stats is None:
            print("❌ Collection pass failed")
            break

        # Update counts from metadata
        with open(metadata_file) as f:
            new_count = sum(1 for line in f if line.strip())
        collected_this_pass = new_count - current_count
        current_count = new_count

        print(f"\n✅ Pass {pass_num} complete:")
        print(f"   Workflows collected this pass: {collected_this_pass}")
        print(f"   Star range this pass: {stats['highest_stars']} - {stats['lowest_stars']}")
        print(f"   Unique repos this pass: {stats['unique_repos']}")
        print(f"   Total workflows now: {current_count}")
        print(f"   Progress: {current_count}/{args.target} ({current_count/args.target*100:.1f}%)")

        # Save pass history
        save_pass_history(history_file, pass_num, {
            'workflows_collected': collected_this_pass,
            'star_range': [stats['highest_stars'], stats['lowest_stars']],
            'unique_repos': stats['unique_repos'],
            'total_workflows': current_count
        })

        # Check if we reached target
        if current_count >= args.target:
            print(f"\n🎉 Target reached! ({current_count} >= {args.target})")
            # Clean up checkpoint
            if checkpoint_file.exists():
                checkpoint_file.unlink()
                print("🗑️  Checkpoint removed (collection complete)")
            break

        # Prepare for next pass: Adaptive subdivision based on lowest star from this pass

        # Stop when zero workflows collected AND we cannot subdivide further
        if collected_this_pass == 0:
            current_range_width = (max_stars - min_stars) if max_stars is not None else None

            # If range width is 0 or we're already at minimum range, truly exhausted
            if current_range_width is None or current_range_width <= 1:
                print("\n⚠️  No new workflows collected, star-based search exhausted")
                print("    Switching to Phase 2 (language-filtered search)")
                break

            # Try subdividing the current range
            print(f"\n⚠️  No new workflows in {min_stars}-{max_stars} stars")
            print(f"    Attempting range subdivision (width: {current_range_width})")

            # Bisect the range
            mid_point = min_stars + (max_stars - min_stars) // 2
            next_min_stars = min_stars
            next_max_stars = mid_point

            # Validate subdivision didn't create invalid range
            if next_max_stars < next_min_stars:
                print(f"\n⚠️  Subdivision would create invalid range: {next_min_stars}-{next_max_stars}")
                print("    Star-based search exhausted, switching to Phase 2")
                break

            print(f"    Next attempt: {next_min_stars}-{next_max_stars} stars")

        # Handle special case: lowest_stars = 0 (cannot go below 0)
        elif stats['lowest_stars'] == 0:
            current_range_width = max_stars - min_stars if max_stars is not None else 0

            if current_range_width <= 1:
                # Already at minimum range with lowest_stars=0, fully exhausted
                print("\n✅ Reached minimum star range (0-1 stars)")
                print("    Star-based search exhausted, switching to Phase 2")
                break

            # Subdivide the 0-max_stars range to search more thoroughly
            print(f"\n⚠️  Reached lowest_stars=0 in range {min_stars}-{max_stars}")
            print(f"    Subdividing range to search more thoroughly")

            # Bisect the range to find more workflows
            mid_point = min_stars + (max_stars - min_stars) // 2
            next_min_stars = min_stars
            next_max_stars = mid_point

            # Validate subdivision didn't create invalid range
            if next_max_stars < next_min_stars:
                print(f"\n⚠️  Subdivision would create invalid range: {next_min_stars}-{next_max_stars}")
                print("    Star-based search exhausted, switching to Phase 2")
                break

            print(f"    Next sub-range: {next_min_stars}-{next_max_stars} stars")

        # Normal case: Continue adaptive subdivision
        else:
            # Standard adaptive strategy: next upper bound = lowest_stars - 1
            next_max_stars = stats['lowest_stars'] - 1
            next_min_stars = 0

            # Validate that next_max_stars is non-negative
            if next_max_stars < 0:
                print(f"\n⚠️  Calculated next_max_stars={next_max_stars} (invalid)")
                print("    Star-based search exhausted, switching to Phase 2")
                break

            print(f"\n✅ Continue adaptive search: 0-{next_max_stars} stars")

        # Validate next range before saving checkpoint
        if next_max_stars is not None and next_max_stars < 0:
            print(f"\n❌ ERROR: Cannot save checkpoint with negative max_stars={next_max_stars}")
            print("    Star-based search exhausted, switching to Phase 2")
            break

        if next_max_stars is not None and next_min_stars > next_max_stars:
            print(f"\n❌ ERROR: Cannot save checkpoint with inverted range: {next_min_stars}-{next_max_stars}")
            print("    Star-based search exhausted, switching to Phase 2")
            break

        # Save checkpoint before next pass
        save_checkpoint(
            checkpoint_file=checkpoint_file,
            pass_num=pass_num,
            total_workflows=current_count,
            target=args.target,
            next_min=next_min_stars,
            next_max=next_max_stars,
            last_pass_stats={
                'collected': collected_this_pass,
                'star_range': [stats['highest_stars'], stats['lowest_stars']],
                'unique_repos': stats['unique_repos']
            }
        )

        # Update for next iteration
        min_stars = next_min_stars
        max_stars = next_max_stars

        pass_num += 1

        # Safety limit
        if pass_num > 200:  # Increased limit to allow more subdivision
            print("\n⚠️  Reached maximum pass limit (200)")
            break

    # If target not reached, try language-based searches
    if current_count < args.target:
        print("\n" + "=" * 70)
        print("🌐 Phase 2: Language-Filtered Search")
        print("=" * 70)
        print(f"Current: {current_count} / Target: {args.target}")
        print(f"Still need: {args.target - current_count} workflows")

        languages = ["Python", "JavaScript", "TypeScript", "Java", "Go", "Rust", "C", "C++", "Ruby", "PHP"]

        for language in languages:
            if current_count >= args.target:
                break

            pass_num += 1
            print(f"\n{'=' * 70}")
            print(f"📊 Pass {pass_num} - Language: {language}")
            print(f"{'=' * 70}")

            # Run language-filtered collection
            cmd = [
                "python3", "scripts/data/collect_workflows.py",
                "--limit", str(args.workflows_per_pass),
                "--search-mode", "language",
                "--language", language,
                "--output", str(args.output),
                "--schema", str(args.schema),
                "--resume"
            ]

            print(f"🚀 Running collection: language={language}")
            print(f"   Command: {' '.join(cmd)}\n")

            result = subprocess.run(cmd, capture_output=False, text=True)

            if result.returncode != 0:
                print(f"⚠️  Language search for {language} failed")
                continue

            # Update count from metadata
            with open(metadata_file) as f:
                new_count = sum(1 for line in f if line.strip())
            collected_this_pass = new_count - current_count
            current_count = new_count

            print(f"\n✅ Pass {pass_num} complete ({language}):")
            print(f"   Workflows collected this pass: {collected_this_pass}")
            print(f"   Total workflows now: {current_count}")
            print(f"   Progress: {current_count}/{args.target} ({current_count/args.target*100:.1f}%)")

            if current_count >= args.target:
                print(f"\n🎉 Target reached! ({current_count} >= {args.target})")
                break

    # Final statistics
    print("\n" + "=" * 70)
    print("📊 Final Collection Statistics")
    print("=" * 70)
    print(f"Total workflows collected: {current_count}")
    print(f"Total passes executed: {pass_num}")

    if current_count >= args.target:
        print(f"✅ SUCCESS: Target reached! ({current_count} >= {args.target})")
        return 0
    else:
        print(f"⚠️  Target not reached: {current_count} / {args.target}")
        print(f"   All available search strategies exhausted")
        return 1


if __name__ == "__main__":
    sys.exit(main())
