#!/usr/bin/env python3
"""
GitHub Workflow Data Collection Script

Collects GitHub Actions workflow files from public repositories for TCT validation
and compression analysis.

Usage:
    # Collect 100 workflows (dry run)
    GITHUB_TOKEN=ghp_... python3 collect_workflows.py --limit 100

    # Collect 50K workflows (full collection)
    GITHUB_TOKEN=ghp_... python3 collect_workflows.py --limit 50000

    # Resume interrupted collection
    GITHUB_TOKEN=ghp_... python3 collect_workflows.py --limit 50000 --resume

Requirements:
    pip install requests pyyaml jsonschema
"""

import argparse
import hashlib
import json
import os
import subprocess
import sys
import tempfile
import time
from pathlib import Path
from typing import Dict, List, Optional, Tuple
import urllib.parse

try:
    import requests
except ImportError:
    print("Error: requests library required. Install with: pip install requests")
    sys.exit(1)

try:
    import yaml
except ImportError:
    print("Error: PyYAML is required. Install with: pip install pyyaml")
    sys.exit(1)

try:
    import jsonschema
except ImportError:
    print("Error: jsonschema is required. Install with: pip install jsonschema")
    sys.exit(1)


class GitHubWorkflowCollector:
    """Collects GitHub Actions workflows from public repositories."""

    def __init__(self, token: str, base_dir: Path, schema_path: Path, collect_all: bool = False, search_mode: str = "auto", language: str = None, query: str = None):
        self.token = token
        self.base_dir = base_dir
        self.schema_path = schema_path
        self.collect_all = collect_all  # If True, collect ALL workflows from repos (not just specific filenames)
        self.search_mode = search_mode  # Search strategy: auto, repo, code, language
        self.language = language  # Language filter for language mode
        self.query = query  # Content search query for code mode

        # Setup directories
        self.raw_dir = base_dir / "raw"
        self.json_dir = base_dir / "json"
        self.failures_dir = base_dir / "failures"
        self.metadata_file = base_dir / "metadata.jsonl"
        self.stats_file = base_dir / "statistics.json"

        for directory in [self.raw_dir, self.json_dir, self.failures_dir]:
            directory.mkdir(parents=True, exist_ok=True)

        # Load schema for validation
        with open(schema_path) as f:
            self.schema = json.load(f)

        # Start validation server (compiles schema once, reuses for all validations)
        print("🚀 Starting TCT validation server (compiling schema once)...")
        self.validator_process = subprocess.Popen(
            ['target/release/tct-validate-server',
             '--schema', str(schema_path)],
            stdin=subprocess.PIPE,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            text=True,
            bufsize=1  # Line buffered
        )

        # Wait for server to be ready (read stderr until we see "Ready")
        import select
        import time as time_module
        ready = False
        start_time = time_module.time()
        while time_module.time() - start_time < 300:  # 300 second (5 minute) timeout for complex schemas
            # Check if stderr has data
            if self.validator_process.stderr:
                line = self.validator_process.stderr.readline()
                if line:
                    print(f"  {line.rstrip()}")
                    if "Ready to validate" in line:
                        ready = True
                        break

        if not ready:
            raise RuntimeError("Validation server failed to start")

        # Statistics
        self.stats = {
            "total_searched": 0,
            "downloaded": 0,
            "yaml_valid": 0,
            "json_valid": 0,
            "schema_valid": 0,
            "encoding_valid": 0,
            "duplicates": 0,
            "errors": {
                "download_failed": 0,
                "yaml_parse_failed": 0,
                "json_conversion_failed": 0,
                "schema_validation_failed": 0,
                "encoding_failed": 0
            }
        }

        # Track seen workflows to avoid duplicates (by path and content)
        self.seen_workflows = set()
        self.seen_content_hashes = set()

        # Load existing metadata if resuming
        if self.metadata_file.exists():
            with open(self.metadata_file) as f:
                for line in f:
                    if line.strip():
                        entry = json.loads(line)
                        workflow_id = f"{entry['repo']}:{entry['workflow_path']}"
                        self.seen_workflows.add(workflow_id)
                        # Load content hash if available
                        if 'content_hash' in entry:
                            self.seen_content_hashes.add(entry['content_hash'])
            print(f"📂 Resuming: found {len(self.seen_workflows)} existing workflows")
            print(f"📂 Loaded {len(self.seen_content_hashes)} content hashes for deduplication")

    def _api_request(self, url: str, params: Optional[Dict] = None) -> Optional[Dict]:
        """Make authenticated GitHub API request with rate limit handling."""
        headers = {
            "Authorization": f"token {self.token}",
            "Accept": "application/vnd.github.v3+json"
        }

        try:
            response = requests.get(url, headers=headers, params=params)

            # Check rate limit
            remaining = int(response.headers.get('X-RateLimit-Remaining', 0))
            if remaining < 10:
                reset_time = int(response.headers.get('X-RateLimit-Reset', 0))
                sleep_time = max(reset_time - time.time(), 0) + 10
                print(f"⏳ Rate limit approaching, sleeping {sleep_time:.0f}s...")
                time.sleep(sleep_time)

            if response.status_code == 403:
                print(f"🚫 Rate limited. Waiting...")
                time.sleep(60)
                return self._api_request(url, params)

            response.raise_for_status()
            return response.json()

        except requests.exceptions.RequestException as e:
            print(f"❌ API request failed: {e}")
            return None

    def _download_workflow(self, repo: str, path: str, download_url: str) -> Optional[Tuple[str, str]]:
        """Download workflow YAML content."""
        try:
            response = requests.get(download_url)
            response.raise_for_status()
            yaml_content = response.text

            # Save raw YAML
            filename = f"{repo.replace('/', '_')}_{Path(path).stem}.yml"
            raw_path = self.raw_dir / filename

            with open(raw_path, 'w', encoding='utf-8') as f:
                f.write(yaml_content)

            return yaml_content, filename

        except requests.exceptions.RequestException as e:
            print(f"❌ Download failed for {repo}/{path}: {e}")
            self.stats["errors"]["download_failed"] += 1
            return None

    def _calculate_content_hash(self, yaml_content: str) -> str:
        """
        Calculate content hash for duplicate detection.

        Normalizes workflow content to detect duplicates even with minor differences:
        - Removes comments
        - Normalizes whitespace
        - Removes trailing whitespace from lines

        Returns: 16-character hex hash
        """
        lines = []
        for line in yaml_content.split('\n'):
            # Remove inline comments (but preserve '#' in strings)
            if '#' in line:
                # Simple heuristic: only remove comments not in quotes
                parts = line.split('#')
                if len(parts) > 1:
                    # Keep first part (before comment)
                    line = parts[0]
            # Remove trailing whitespace
            line = line.rstrip()
            if line:  # Skip empty lines
                lines.append(line)

        normalized = '\n'.join(lines)
        return hashlib.sha256(normalized.encode('utf-8')).hexdigest()[:16]

    def _preprocess_yaml(self, yaml_content: str) -> str:
        """
        Preprocess YAML to fix common issues.

        Fixes:
        - Inline comments after colons before nested structures (PyYAML bug)
        - "on" keyword (YAML boolean) → quoted "'on'" for GitHub Actions
        """
        lines = yaml_content.split('\n')
        processed = []

        for i, line in enumerate(lines):
            # Fix YAML boolean keywords used as keys (on, off, yes, no, true, false)
            # GitHub Actions uses "on:" which YAML parses as True:
            stripped = line.lstrip()
            if stripped.startswith('on:'):
                # Replace "on:" with "'on':"
                indent = line[:len(line) - len(stripped)]
                rest = stripped[3:]  # Everything after "on:"
                line = f"{indent}'on':{rest}"

            # Check for pattern: "key: # comment" followed by indented content
            if ':' in line and '#' in line:
                before_hash = line.split('#')[0]
                # If line ends with ": #" or ": # comment", it's problematic
                if before_hash.rstrip().endswith(':'):
                    # Remove the inline comment
                    processed.append(before_hash.rstrip())
                    continue

            processed.append(line)

        return '\n'.join(processed)

    def _convert_to_json(self, yaml_content: str, filename: str) -> Optional[Dict]:
        """Convert YAML workflow to JSON."""
        try:
            # Preprocess YAML to fix common issues
            yaml_content = self._preprocess_yaml(yaml_content)

            # Parse YAML
            data = yaml.safe_load(yaml_content)
            if data is None:
                return None

            # Save JSON
            json_filename = filename.replace('.yml', '.json').replace('.yaml', '.json')
            json_path = self.json_dir / json_filename

            with open(json_path, 'w', encoding='utf-8') as f:
                json.dump(data, f, indent=2, sort_keys=False)

            return data

        except yaml.YAMLError as e:
            print(f"❌ YAML parse failed for {filename}: {e}")
            self.stats["errors"]["yaml_parse_failed"] += 1
            return None
        except Exception as e:
            print(f"❌ JSON conversion failed for {filename}: {e}")
            self.stats["errors"]["json_conversion_failed"] += 1
            return None

    def _validate_schema(self, data: Dict, filename: str) -> bool:
        """Validate JSON against github-workflow schema."""
        try:
            jsonschema.validate(instance=data, schema=self.schema)
            return True
        except jsonschema.ValidationError as e:
            print(f"⚠️  Schema validation failed for {filename}: {e.message[:100]}")
            self.stats["errors"]["schema_validation_failed"] += 1

            # Save failure case for analysis
            failure_path = self.failures_dir / filename
            with open(failure_path, 'w', encoding='utf-8') as f:
                json.dump({
                    "data": data,
                    "error": str(e),
                    "error_path": list(e.path)
                }, f, indent=2)

            return False

    def _validate_encoding(self, data: Dict, filename: str) -> bool:
        """
        Validate that workflow can be encoded with TCT.

        Uses the validation server (schema compiled once at startup).
        Returns True if encodable, False otherwise.
        """
        try:
            # Write JSON to temp file
            with tempfile.NamedTemporaryFile(mode='w', suffix='.json', delete=False) as tmp:
                json.dump(data, tmp)
                tmp_path = tmp.name

            try:
                # Send path to validation server
                self.validator_process.stdin.write(tmp_path + '\n')
                self.validator_process.stdin.flush()

                # Read response
                response = self.validator_process.stdout.readline().strip()

                if response.startswith('OK'):
                    return True
                elif response.startswith('ERROR'):
                    # Extract error message
                    parts = response.split(' ', 2)
                    error_msg = parts[2] if len(parts) > 2 else "Unknown error"
                    print(f"⚠️  Encoding failed for {filename}: {error_msg[:100]}")
                    self.stats["errors"]["encoding_failed"] += 1
                    return False
                else:
                    print(f"⚠️  Unexpected validator response: {response}")
                    self.stats["errors"]["encoding_failed"] += 1
                    return False

            finally:
                # Clean up temp file
                Path(tmp_path).unlink(missing_ok=True)

        except Exception as e:
            print(f"⚠️  Encoding validation error for {filename}: {str(e)[:100]}")
            self.stats["errors"]["encoding_failed"] += 1
            return False

    def _save_metadata(self, entry: Dict):
        """Append metadata entry to JSONL file."""
        with open(self.metadata_file, 'a', encoding='utf-8') as f:
            f.write(json.dumps(entry) + '\n')

    def _save_statistics(self):
        """Save collection statistics."""
        with open(self.stats_file, 'w', encoding='utf-8') as f:
            json.dump(self.stats, f, indent=2)

    def search_workflows(self, limit: int, diversity_strategy: str = "balanced", min_stars: int = 0, max_stars: int = None):
        """
        Search for GitHub workflows.

        Args:
            limit: Maximum number of workflows to collect
            diversity_strategy: ignored (kept for compatibility)
            min_stars: Minimum star count for repositories (default: 0)
            max_stars: Maximum star count for repositories (default: None = no limit)
        """
        if max_stars:
            star_filter = f" (repos with {min_stars}-{max_stars} stars)"
        elif min_stars > 0:
            star_filter = f" (repos with {min_stars}+ stars)"
        else:
            star_filter = ""

        # Determine search mode
        mode = self.search_mode
        if mode == "auto":
            # Auto-select based on parameters
            if self.language:
                mode = "language"
            elif min_stars > 0 or max_stars is not None:
                mode = "repo"
            else:
                mode = "code"

        print(f"🔍 Searching for GitHub workflows (limit: {limit}{star_filter}, mode: {mode})")

        # Execute appropriate search strategy
        if mode == "language":
            if not self.language:
                raise ValueError("Language mode requires --language parameter")
            self._search_workflows_by_language(limit, self.language)
        elif mode == "repo":
            # Use repository search when filtering by stars
            self._search_repos_then_workflows(limit, min_stars, max_stars)
        elif mode == "code":
            # Use code search for filename-based collection
            self._search_workflows_simple(limit, min_stars)
        else:
            raise ValueError(f"Unknown search mode: {mode}")

        print(f"\n✅ Collection complete!")
        self._print_statistics()

        # Shutdown validation server
        self._shutdown_validator()

    def _search_repos_then_workflows(self, limit: int, min_stars: int, max_stars: int = None):
        """
        Efficient search: Find repos by stars first, then get their workflows.

        Uses Repository Search API with stars filter for better efficiency.
        """
        if max_stars:
            print(f"\n📊 Searching repositories with {min_stars}-{max_stars} stars...")
        else:
            print(f"\n📊 Searching repositories with {min_stars}+ stars...")

        collected = 0
        page = 1
        max_pages = 10  # GitHub API limits search to 1000 results (10 pages of 100)

        while collected < limit and page <= max_pages:
            # Search for repositories with star filter
            if max_stars:
                query = f"stars:{min_stars}..{max_stars} fork:false"
            else:
                query = f"stars:>={min_stars} fork:false"
            search_url = "https://api.github.com/search/repositories"
            params = {
                "q": query,
                "sort": "stars",
                "order": "desc",
                "per_page": 100,
                "page": page
            }

            result = self._api_request(search_url, params)
            if not result or "items" not in result:
                break

            repositories = result["items"]
            if not repositories:
                break

            print(f"  Page {page}: Found {len(repositories)} repositories")

            # Process each repository
            for repo_data in repositories:
                if collected >= limit:
                    break

                repo = repo_data["full_name"]
                stars = repo_data["stargazers_count"]

                # Get all workflows from this repository
                workflows = self._get_repo_workflows(repo)

                if not workflows:
                    continue

                print(f"  📦 {repo} ({stars} ⭐): {len(workflows)} workflows")

                # Process each workflow from this repo
                for workflow in workflows:
                    if collected >= limit:
                        break

                    workflow_id = f"{repo}:{workflow['path']}"

                    # Skip duplicates
                    if workflow_id in self.seen_workflows:
                        self.stats["duplicates"] += 1
                        continue

                    self.seen_workflows.add(workflow_id)
                    self.stats["total_searched"] += 1

                    # Process workflow
                    if self._process_workflow(repo, stars, workflow):
                        collected += 1
                        self.stats["downloaded"] += 1

                        if collected % 10 == 0:
                            print(f"  ✓ Collected {collected}/{limit} "
                                  f"(schema: {self.stats['schema_valid']}, "
                                  f"encodable: {self.stats['encoding_valid']})")

                    # Small delay to be nice to GitHub
                    time.sleep(0.05)

            page += 1

        print(f"\n  Completed: {collected} workflows collected from high-star repositories")

    def _search_workflows_simple(self, limit: int, min_stars: int = 0):
        """Search for workflow files using GitHub code search."""

        # If query is provided, use content-based search
        if self.query:
            print(f"\n🔍 Searching workflows by content: '{self.query}'...")
            return self._search_workflows_by_content(limit, min_stars)

        print(f"\n📊 Searching workflow files...")

        # Expanded workflow filename patterns for better discovery
        workflow_names = [
            # Original patterns (keep these)
            "ci.yml", "build.yml", "test.yml", "main.yml", "workflow.yml",

            # Deployment & Release workflows
            "deploy.yml", "deployment.yml", "release.yml", "publish.yml",

            # Code quality & linting
            "lint.yml", "linter.yml", "check.yml", "checks.yml", "quality.yml",

            # Testing variants
            "integration.yml", "unit-test.yml", "e2e.yml", "tests.yml",

            # Pull request workflows
            "pr.yml", "pull-request.yml", "pr-check.yml",

            # Documentation
            "docs.yml", "documentation.yml", "pages.yml",

            # Scheduled/Cron jobs
            "cron.yml", "schedule.yml", "nightly.yml", "daily.yml",

            # Performance & benchmarks
            "benchmark.yml", "perf.yml", "performance.yml"
        ]

        collected = 0
        skipped_low_stars = 0
        repo_star_cache = {}  # Cache star counts to avoid duplicate API calls

        for workflow_name in workflow_names:
            if collected >= limit:
                break

            page = 1
            while collected < limit and self.stats["downloaded"] < limit:
                # Search for specific workflow filename
                query = f"filename:{workflow_name} path:.github/workflows"
                search_url = "https://api.github.com/search/code"
                params = {
                    "q": query,
                    "per_page": 100,
                    "page": page
                }

                result = self._api_request(search_url, params)
                if not result or "items" not in result:
                    break

                workflow_files = result["items"]
                if not workflow_files:
                    break

                # Process each workflow file
                for file_data in workflow_files:
                    if collected >= limit or self.stats["downloaded"] >= limit:
                        break

                    repo = file_data["repository"]["full_name"]
                    workflow_path = file_data["path"]

                    workflow_id = f"{repo}:{workflow_path}"

                    # Skip duplicates
                    if workflow_id in self.seen_workflows:
                        self.stats["duplicates"] += 1
                        continue

                    self.seen_workflows.add(workflow_id)
                    self.stats["total_searched"] += 1

                    # Get repository details for star count (with caching)
                    if repo not in repo_star_cache:
                        repo_details = self._get_repo_details(repo)
                        if not repo_details:
                            continue
                        repo_star_cache[repo] = repo_details.get("stargazers_count", 0)

                    stars = repo_star_cache[repo]

                    # Filter by minimum stars
                    if stars < min_stars:
                        skipped_low_stars += 1
                        if skipped_low_stars % 50 == 0:
                            print(f"  ⏭️  Skipped {skipped_low_stars} workflows from low-star repos")
                        continue

                    # Get download URL (code search uses html_url, need to convert to raw)
                    html_url = file_data["html_url"]
                    # Convert: https://github.com/owner/repo/blob/branch/.github/workflows/ci.yml
                    # To: https://raw.githubusercontent.com/owner/repo/branch/.github/workflows/ci.yml
                    download_url = html_url.replace("github.com", "raw.githubusercontent.com").replace("/blob/", "/")

                    workflow = {
                        "path": workflow_path,
                        "name": file_data["name"],
                        "download_url": download_url,
                        "size": file_data.get("size", 0),
                        "type": "file"
                    }

                    # Process workflow with star count
                    if self._process_workflow(repo, stars, workflow):
                        collected += 1
                        self.stats["downloaded"] += 1

                        if collected % 10 == 0:
                            print(f"  ✓ Collected {collected}/{limit} "
                                  f"(schema: {self.stats['schema_valid']}, "
                                  f"encodable: {self.stats['encoding_valid']})")

                    time.sleep(0.05)

                page += 1
                # GitHub API limits search to 1000 results (10 pages of 100)
                if page > 10:
                    break

            print(f"  Completed searching for {workflow_name}: {collected} total")

    def _search_workflows_by_content(self, limit: int, min_stars: int = 0):
        """Search for workflow files by content using GitHub Code Search."""
        collected = 0
        skipped_low_stars = 0
        repo_star_cache = {}  # Cache star counts to avoid duplicate API calls

        page = 1
        while collected < limit and self.stats["downloaded"] < limit:
            # Search for content in workflow files
            query = f"{self.query} path:.github/workflows"
            search_url = "https://api.github.com/search/code"
            params = {
                "q": query,
                "per_page": 100,
                "page": page
            }

            result = self._api_request(search_url, params)
            if not result or "items" not in result:
                break

            workflow_files = result["items"]
            if not workflow_files:
                break

            # Process each workflow file
            for file_data in workflow_files:
                if collected >= limit or self.stats["downloaded"] >= limit:
                    break

                repo = file_data["repository"]["full_name"]
                workflow_path = file_data["path"]

                workflow_id = f"{repo}:{workflow_path}"

                # Skip duplicates
                if workflow_id in self.seen_workflows:
                    self.stats["duplicates"] += 1
                    continue

                self.seen_workflows.add(workflow_id)
                self.stats["total_searched"] += 1

                # Get repository details for star count (with caching)
                if repo not in repo_star_cache:
                    repo_details = self._get_repo_details(repo)
                    if not repo_details:
                        continue
                    repo_star_cache[repo] = repo_details.get("stargazers_count", 0)

                stars = repo_star_cache[repo]

                # Filter by minimum stars
                if stars < min_stars:
                    skipped_low_stars += 1
                    if skipped_low_stars % 50 == 0:
                        print(f"  ⏭️  Skipped {skipped_low_stars} workflows from low-star repos")
                    continue

                # Get download URL (code search uses html_url, need to convert to raw)
                html_url = file_data["html_url"]
                # Convert: https://github.com/owner/repo/blob/branch/.github/workflows/ci.yml
                # To: https://raw.githubusercontent.com/owner/repo/branch/.github/workflows/ci.yml
                download_url = html_url.replace("github.com", "raw.githubusercontent.com").replace("/blob/", "/")

                workflow = {
                    "path": workflow_path,
                    "name": file_data["name"],
                    "download_url": download_url,
                    "size": file_data.get("size", 0),
                    "type": "file"
                }

                # Process workflow with star count
                if self._process_workflow(repo, stars, workflow):
                    collected += 1
                    self.stats["downloaded"] += 1

                    if collected % 10 == 0:
                        print(f"  ✓ Collected {collected}/{limit} "
                              f"(schema: {self.stats['schema_valid']}, "
                              f"encodable: {self.stats['encoding_valid']})")

                time.sleep(0.05)

            page += 1
            # GitHub API limits search to 1000 results (10 pages of 100)
            if page > 10:
                break

        print(f"  Completed content search: {collected} total")

    def _search_workflows_by_language(self, limit: int, language: str):
        """Search for workflow files filtered by programming language."""
        print(f"\n📊 Searching workflow files for {language} projects...")

        # Expanded workflow filename patterns
        workflow_names = [
            "ci.yml", "build.yml", "test.yml", "main.yml", "workflow.yml",
            "deploy.yml", "deployment.yml", "release.yml", "publish.yml",
            "lint.yml", "linter.yml", "check.yml", "checks.yml", "quality.yml",
            "integration.yml", "unit-test.yml", "e2e.yml", "tests.yml",
            "pr.yml", "pull-request.yml", "pr-check.yml",
            "docs.yml", "documentation.yml", "pages.yml",
            "cron.yml", "schedule.yml", "nightly.yml", "daily.yml",
            "benchmark.yml", "perf.yml", "performance.yml"
        ]

        collected = 0
        repo_star_cache = {}

        for workflow_name in workflow_names:
            if collected >= limit:
                break

            page = 1
            while collected < limit:
                # Search for workflow file with language filter
                query = f"language:{language} filename:{workflow_name} path:.github/workflows"
                search_url = "https://api.github.com/search/code"
                params = {
                    "q": query,
                    "per_page": 100,
                    "page": page
                }

                result = self._api_request(search_url, params)
                if not result or "items" not in result:
                    break

                workflow_files = result["items"]
                if not workflow_files:
                    break

                # Process each workflow file
                for file_data in workflow_files:
                    if collected >= limit:
                        break

                    repo = file_data["repository"]["full_name"]
                    workflow_path = file_data["path"]
                    workflow_id = f"{repo}:{workflow_path}"

                    # Skip duplicates
                    if workflow_id in self.seen_workflows:
                        self.stats["duplicates"] += 1
                        continue

                    self.seen_workflows.add(workflow_id)
                    self.stats["total_searched"] += 1

                    # Get repository star count (with caching)
                    if repo not in repo_star_cache:
                        repo_details = self._get_repo_details(repo)
                        if not repo_details:
                            continue
                        repo_star_cache[repo] = repo_details.get("stargazers_count", 0)

                    stars = repo_star_cache[repo]

                    # Get download URL
                    html_url = file_data["html_url"]
                    download_url = html_url.replace("github.com", "raw.githubusercontent.com").replace("/blob/", "/")

                    workflow = {
                        "path": workflow_path,
                        "name": file_data["name"],
                        "download_url": download_url,
                        "size": file_data.get("size", 0),
                        "type": "file"
                    }

                    # Process workflow
                    if self._process_workflow(repo, stars, workflow):
                        collected += 1
                        self.stats["downloaded"] += 1

                        if collected % 10 == 0:
                            print(f"  ✓ Collected {collected}/{limit} "
                                  f"(schema: {self.stats['schema_valid']}, "
                                  f"encodable: {self.stats['encoding_valid']})")

                    time.sleep(0.05)

                page += 1
                # GitHub API limits search to 1000 results (10 pages of 100)
                if page > 10:
                    break

            print(f"  Completed searching for {language}/{workflow_name}: {collected} total")

    def _get_repo_details(self, repo: str) -> Optional[Dict]:
        """Get repository details including star count."""
        url = f"https://api.github.com/repos/{repo}"
        result = self._api_request(url)
        return result

    def _get_repo_workflows(self, repo: str) -> List[Dict]:
        """
        Get workflow files from a repository.

        If self.collect_all is True, returns ALL .yml/.yaml files.
        Otherwise, returns only workflows matching expanded pattern list.
        """
        url = f"https://api.github.com/repos/{repo}/contents/.github/workflows"
        result = self._api_request(url)

        if not result or not isinstance(result, list):
            return []

        # Filter for YAML files
        workflows = [
            item for item in result
            if item["type"] == "file" and (
                item["name"].endswith('.yml') or item["name"].endswith('.yaml')
            )
        ]

        # If collect_all is False, filter by known workflow patterns
        if not self.collect_all:
            # Expanded pattern list for filename matching
            known_patterns = {
                "ci.yml", "build.yml", "test.yml", "main.yml", "workflow.yml",
                "deploy.yml", "deployment.yml", "release.yml", "publish.yml",
                "lint.yml", "linter.yml", "check.yml", "checks.yml", "quality.yml",
                "integration.yml", "unit-test.yml", "e2e.yml", "tests.yml",
                "pr.yml", "pull-request.yml", "pr-check.yml",
                "docs.yml", "documentation.yml", "pages.yml",
                "cron.yml", "schedule.yml", "nightly.yml", "daily.yml",
                "benchmark.yml", "perf.yml", "performance.yml"
            }
            workflows = [w for w in workflows if w["name"] in known_patterns]

        return workflows

    def _process_workflow(self, repo: str, stars: int, workflow: Dict) -> bool:
        """Download, convert, validate, and save a workflow."""
        path = workflow["path"]
        download_url = workflow["download_url"]

        # Download
        download_result = self._download_workflow(repo, path, download_url)
        if not download_result:
            return False

        yaml_content, filename = download_result

        # Check content hash for duplicates (before expensive validation)
        content_hash = self._calculate_content_hash(yaml_content)
        if content_hash in self.seen_content_hashes:
            self.stats["duplicates"] += 1
            print(f"  ⏭️  Skipping duplicate content: {filename}")
            return False

        # Convert to JSON
        json_data = self._convert_to_json(yaml_content, filename)
        if not json_data:
            return False

        self.stats["yaml_valid"] += 1
        self.stats["json_valid"] += 1

        # Validate schema
        schema_valid = self._validate_schema(json_data, filename)
        if not schema_valid:
            # Don't save non-schema-valid workflows
            return False

        self.stats["schema_valid"] += 1

        # Validate encoding (only for schema-valid workflows)
        encoding_valid = self._validate_encoding(json_data, filename)
        if not encoding_valid:
            # Don't save non-encodable workflows
            return False

        self.stats["encoding_valid"] += 1

        # Save metadata (only for workflows that passed both validations)
        metadata = {
            "repo": repo,
            "stars": stars,
            "workflow_path": path,
            "workflow_name": workflow["name"],
            "size_bytes": workflow["size"],
            "filename": filename,
            "content_hash": content_hash,
            "yaml_valid": True,
            "json_valid": True,
            "schema_valid": True,
            "encodable": True
        }

        self._save_metadata(metadata)
        self._save_statistics()

        # Mark content as seen (only after successful save)
        self.seen_content_hashes.add(content_hash)

        return True

    def _shutdown_validator(self):
        """Shutdown the validation server gracefully."""
        try:
            # Send empty line to signal shutdown
            self.validator_process.stdin.write('\n')
            self.validator_process.stdin.flush()
            self.validator_process.stdin.close()

            # Wait for process to exit
            self.validator_process.wait(timeout=5)
            print("✅ Validation server shutdown")
        except Exception as e:
            print(f"⚠️  Error shutting down validator: {e}")
            self.validator_process.kill()

    def _print_statistics(self):
        """Print collection statistics."""
        print(f"\n{'='*70}")
        print(f"📊 Collection Statistics")
        print(f"{'='*70}")
        print(f"Total searched:          {self.stats['total_searched']}")
        print(f"Downloaded:              {self.stats['downloaded']}")
        print(f"Duplicates skipped:      {self.stats['duplicates']}")
        print(f"YAML valid:              {self.stats['yaml_valid']}")
        print(f"JSON valid:              {self.stats['json_valid']}")
        print(f"Schema valid:            {self.stats['schema_valid']} ({self.stats['schema_valid']/max(1,self.stats['downloaded'])*100:.1f}%)")
        print(f"TCT encodable:           {self.stats['encoding_valid']} ({self.stats['encoding_valid']/max(1,self.stats['schema_valid'])*100:.1f}% of schema-valid)")
        print(f"\nErrors:")
        for error_type, count in self.stats['errors'].items():
            print(f"  {error_type:30s} {count}")
        print(f"{'='*70}")


def main():
    parser = argparse.ArgumentParser(
        description="Collect GitHub Actions workflows for TCT validation"
    )
    parser.add_argument(
        "--limit",
        type=int,
        default=100,
        help="Maximum number of workflows to collect (default: 100)"
    )
    parser.add_argument(
        "--min-stars",
        type=int,
        default=0,
        help="Minimum repository star count (default: 0, recommended: 1000 for quality)"
    )
    parser.add_argument(
        "--max-stars",
        type=int,
        default=None,
        help="Maximum repository star count (default: None = no limit)"
    )
    parser.add_argument(
        "--diversity",
        choices=["popular", "balanced", "random"],
        default="balanced",
        help="Diversity strategy for repository selection (default: balanced)"
    )
    parser.add_argument(
        "--resume",
        action="store_true",
        help="Resume from existing metadata file"
    )
    parser.add_argument(
        "--collect-all-workflows",
        action="store_true",
        default=True,
        help="Collect ALL workflows from repos (default: True). Use --no-collect-all-workflows to disable."
    )
    parser.add_argument(
        "--no-collect-all-workflows",
        dest="collect_all_workflows",
        action="store_false",
        help="Only collect workflows with specific filenames (ci.yml, build.yml, etc.)"
    )
    parser.add_argument(
        "--search-mode",
        choices=["auto", "repo", "code", "language"],
        default="auto",
        help="Search strategy: auto (default), repo (repository search), code (filename search), language (language-filtered)"
    )
    parser.add_argument(
        "--language",
        type=str,
        help="Programming language filter for language mode (e.g., Python, JavaScript, TypeScript)"
    )
    parser.add_argument(
        "--query",
        type=str,
        help="Content search query for code mode (e.g., 'uses: actions/checkout', 'runs-on: ubuntu-latest')"
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

    args = parser.parse_args()

    # Check for GitHub token
    token = os.getenv("GITHUB_TOKEN")
    if not token:
        print("❌ Error: GITHUB_TOKEN environment variable not set")
        print("Create a token at: https://github.com/settings/tokens")
        print("Required scopes: public_repo (read-only)")
        print("\nUsage: GITHUB_TOKEN=ghp_... python3 collect_workflows.py")
        return 1

    # Validate schema exists
    if not args.schema.exists():
        print(f"❌ Error: Schema file not found: {args.schema}")
        return 1

    # Create collector
    collector = GitHubWorkflowCollector(token, args.output, args.schema, args.collect_all_workflows, args.search_mode, args.language, args.query)

    # Run collection
    try:
        collector.search_workflows(args.limit, args.diversity, args.min_stars, args.max_stars)
        return 0
    except KeyboardInterrupt:
        print("\n\n⚠️  Collection interrupted by user")
        print("Resume with: --resume flag")
        collector._save_statistics()
        return 130
    except Exception as e:
        print(f"\n❌ Unexpected error: {e}")
        import traceback
        traceback.print_exc()
        return 1


if __name__ == "__main__":
    sys.exit(main())
