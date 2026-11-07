#!/usr/bin/env python3
"""Check the exact vendored-source clearance inventory without changing files."""

import argparse
import hashlib
import json
from pathlib import Path
import tarfile


def digest(data):
    return hashlib.sha256(data).hexdigest()


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--upstream-cache", type=Path)
    args = parser.parse_args()
    root = Path(__file__).resolve().parents[3]
    inventory = json.loads(Path(__file__).with_name("inventory.json").read_text())
    entries = {entry["path"]: entry for entry in inventory["entries"]}
    errors = []
    if len(entries) != len(inventory["entries"]):
        errors.append("duplicate inventory paths")

    def check_file(entry):
        path = root / entry["path"]
        if not path.is_file():
            errors.append(f"missing: {entry['path']}")
            return
        data = path.read_bytes()
        if digest(data) != entry["sha256"] or len(data) != entry["bytes"]:
            errors.append(f"changed: {entry['path']}")

    for entry in inventory["entries"]:
        check_file(entry)
        for notice in entry["required_notices"]:
            if not (root / notice).is_file():
                errors.append(f"missing notice: {notice} for {entry['path']}")
        for dependency in entry.get("dependencies", []):
            if dependency not in entries:
                errors.append(f"unreviewed dependency: {dependency}")
        source = entry.get("recorded_source")
        if source and source["sha256"] != entries[source["path"]]["sha256"]:
            errors.append(f"source identity mismatch: {entry['path']}")
    for notice in inventory["required_notice_documents"]:
        check_file(notice)

    exclusions = set(inventory["inventory_root_exclusions"])
    for directory in inventory["inventory_roots"]:
        for path in (root / directory).rglob("*"):
            relative = path.relative_to(root).as_posix()
            if path.is_file() and relative not in entries and relative not in exclusions:
                errors.append(f"not inventoried: {relative}")

    upstream_checks = 0
    if args.upstream_cache:
        cache = args.upstream_cache
        receipts = json.loads((cache / "retrieval.json").read_text())
        by_url = {
            item["url"]: cache / item["file"]
            for item in receipts
            if item["status"] == "retrieved"
        }
        expected_downloads = {item["url"]: item for item in inventory["retrieval_evidence"]}
        for url, path in by_url.items():
            expected = expected_downloads.get(url)
            if expected is None or digest(path.read_bytes()) != expected["sha256"]:
                errors.append(f"download identity mismatch: {url}")
        with tarfile.open(cache / "go.tar.gz", "r:gz") as archive:
            prefix = archive.getmembers()[0].name + "/"

            def upstream_bytes(upstream):
                if upstream.get("repository") == "https://github.com/golang/go":
                    return archive.extractfile(prefix + upstream["path"]).read()
                return by_url[upstream["url"]].read_bytes()

            for entry in inventory["entries"]:
                if entry["kind"] == "upstream_source":
                    data = upstream_bytes(entry["upstream"])
                    if digest(data) != entry["sha256"]:
                        errors.append(f"upstream mismatch: {entry['path']}")
                    upstream_checks += 1
                for source in entry.get("upstream_sources", []):
                    if digest(upstream_bytes(source)) != source["sha256"]:
                        errors.append(f"upstream input mismatch: {source['url']}")
                    upstream_checks += 1

    result = {
        "status": "failed" if errors else "passed",
        "inventory_sha256": digest(Path(__file__).with_name("inventory.json").read_bytes()),
        "files_checked": len(entries),
        "notice_documents_checked": len(inventory["required_notice_documents"]),
        "upstream_byte_comparisons": upstream_checks,
        "errors": sorted(set(errors)),
    }
    print(json.dumps(result, indent=2))
    return int(bool(errors))


if __name__ == "__main__":
    raise SystemExit(main())
