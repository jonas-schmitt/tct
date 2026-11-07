#!/usr/bin/env python3
"""Verify instance/notice hashes; never modify fixtures or inspect credentials."""
import argparse
import hashlib
import json
from pathlib import Path
import subprocess


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--snapshot', help='Verify every original fixture against a Git snapshot')
    parser.add_argument('--require-exclusions', action='store_true')
    args = parser.parse_args()
    root = Path(__file__).resolve().parents[3]
    ledger_dir = Path(__file__).resolve().parent
    ledger = json.loads((ledger_dir / 'ledger.json').read_text())
    failures = []
    checked = 0
    for record in ledger['records']:
        path = root / record['path']
        if args.snapshot:
            result = subprocess.run(['git', 'show', args.snapshot + ':' + record['path']], cwd=root, capture_output=True)
            if result.returncode:
                failures.append(record['path'] + ': missing from snapshot')
                continue
            data = result.stdout
        elif record['decision'] == 'exclude_from_public_export':
            if args.require_exclusions and path.exists():
                failures.append(record['path'] + ': excluded original still present')
            continue
        elif record['decision'] == 'retain_repository_documentation_subject_to_update':
            continue
        else:
            if not path.is_file():
                failures.append(record['path'] + ': retained file missing')
                continue
            data = path.read_bytes()
        if hashlib.sha256(data).hexdigest() != record['sha256']:
            failures.append(record['path'] + ': hash mismatch')
        checked += 1
    for notice in json.loads((ledger_dir / 'notices.json').read_text())['notices']:
        path = root / notice['path']
        if not path.is_file() or hashlib.sha256(path.read_bytes()).hexdigest() != notice['sha256']:
            failures.append(notice['path'] + ': notice missing or hash mismatch')
        checked += 1
    additions_path = ledger_dir / 'authored-additions.json'
    if additions_path.exists() and not args.snapshot:
        for record in json.loads(additions_path.read_text())['records']:
            path = root / record['path']
            if not path.is_file() or hashlib.sha256(path.read_bytes()).hexdigest() != record['sha256']:
                failures.append(record['path'] + ': authored addition missing or hash mismatch')
            checked += 1
    print(json.dumps({'checked_records': checked, 'failures': failures}, indent=2))
    return bool(failures)


if __name__ == '__main__':
    raise SystemExit(main())
