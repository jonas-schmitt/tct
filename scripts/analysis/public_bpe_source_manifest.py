#!/usr/bin/env python3
"""Verify public BPE artifacts and a Git-independent, complete source snapshot.

The snapshot excludes only this manifest, its twelve separately hashed BPE
files, root Git/build/environment caches, and Python cache directories. Git
ignore rules never select source. Original training commit IDs are archival;
verification and retraining use the current content-addressed source tree.
"""
from __future__ import annotations

import argparse
import glob
import hashlib
import json
import os
from pathlib import Path, PurePosixPath
import re
import stat

MANIFEST_PATH = "third_party/clearance/bpe-artifacts.json"
SPEC_PATH = "publication/bpe-training-spec.json"
SNAPSHOT_FORMAT = "tct-public-bpe-source-v1"
ROOT_CACHE_DIRS = {".git", "target", ".venv", "venv", ".pytest_cache", ".mypy_cache", ".ruff_cache"}
DATASET_COUNTS = {"package": 2, "eslintrc": 3, "kubernetes": 4, "tsconfig": 1, "github-workflow": 2}
EXPECTED_OUTPUTS = {
    f"schemas/bpe/{name}-tct-bpe-{label}.json": (name, budget)
    for name in DATASET_COUNTS
    for label, budget in ([("500", 500), ("1k", 1000)] if name == "tsconfig" else [("500", 500)])
}


def require(condition, message):
    if not condition:
        raise ValueError(message)


def sha256(data):
    return hashlib.sha256(data).hexdigest()


def safe_path(value):
    require(isinstance(value, str) and bool(value), "invalid relative path")
    require(not PurePosixPath(value).is_absolute() and "\\" not in value,
            "unsafe relative path")
    require(not any(c in value for c in ("\0", "\n", "\r", ":")), "unsafe relative path")
    require(all(p not in ("", ".", "..", ".git") for p in value.split("/")),
            "unsafe relative path")
    return value


def file_record(root, relative):
    relative = safe_path(relative)
    path = root / relative
    require(not any((root / Path(*Path(relative).parts[:i])).is_symlink()
                    for i in range(1, len(Path(relative).parts) + 1)),
            "symlink in input or output path")
    require(path.is_file(), "missing regular file: " + relative)
    data = path.read_bytes()
    return {"path": relative, "bytes": len(data), "sha256": sha256(data)}


def unique_object(pairs):
    value = {}
    for key, item in pairs:
        require(key not in value, "duplicate JSON object key")
        value[key] = item
    return value


def read_json(path):
    try:
        return json.loads(path.read_bytes(), object_pairs_hook=unique_object)
    except (ValueError, UnicodeError):
        raise ValueError("invalid JSON: " + path.name) from None


def load_spec(source):
    spec = read_json(source / SPEC_PATH)
    require(isinstance(spec, dict) and set(spec) == {"version", "purpose", "datasets", "artifacts"}, "invalid training spec")
    require(type(spec["version"]) is int and spec["version"] == 1, "unknown training spec version")
    require(isinstance(spec["datasets"], dict) and set(spec["datasets"]) == set(DATASET_COUNTS), "unexpected datasets")
    require(isinstance(spec["artifacts"], list) and len(spec["artifacts"]) == 6, "expected six artifacts")
    file_record(source, "Cargo.lock")
    for name, dataset in spec["datasets"].items():
        require(isinstance(dataset, dict) and set(dataset) == {"schema", "instances_pattern", "fixtures"}, "invalid dataset spec")
        require(dataset["schema"] == f"schemas/popular/{name}.json", "unexpected schema path")
        pattern = f"tests/data/instances/{name}/synthetic/public_*.json"
        require(dataset["instances_pattern"] == pattern, "unexpected fixture glob")
        fixtures = dataset["fixtures"]
        require(isinstance(fixtures, list) and len(fixtures) == DATASET_COUNTS[name], "unexpected fixture count")
        declared = []
        for item in fixtures:
            require(isinstance(item, dict) and set(item) == {"path", "sha256"}, "invalid fixture pin")
            path = safe_path(item["path"])
            require(re.fullmatch(r"[0-9a-f]{64}", item["sha256"]) is not None, "invalid fixture hash")
            require(file_record(source, path)["sha256"] == item["sha256"], "fixture hash mismatch")
            declared.append(path)
        observed = sorted(Path(p).relative_to(source).as_posix() for p in glob.glob(str(source / pattern)))
        require(sorted(declared) == observed and len(set(declared)) == len(declared), "fixture glob differs from exact authored allowlist")
        file_record(source, dataset["schema"])
    seen = set()
    for job in spec["artifacts"]:
        require(isinstance(job, dict) and set(job) == {"dataset", "target_vocab", "optimization", "output"}, "invalid artifact spec")
        output = safe_path(job["output"])
        require(output in EXPECTED_OUTPUTS and output not in seen, "unexpected or duplicate output path")
        require((job["dataset"], job["target_vocab"]) == EXPECTED_OUTPUTS[output] and type(job["target_vocab"]) is int,
                "artifact budget or dataset mismatch")
        require(job["optimization"] == "O3", "unexpected optimization")
        seen.add(output)
    return spec


def sidecar_path(output):
    return str(PurePosixPath(output).with_suffix(".provenance.json"))


def input_fingerprint(records):
    data = b"tct-bpe-artifact-inputs-v1\n"
    for item in records:
        data += f"{item['path']}\0{item['bytes']}\0{item['sha256']}\n".encode()
    return sha256(data)


def validate_outputs(clone, job, dataset, expected, training_date):
    output = file_record(clone, job["output"])
    sidecar = file_record(clone, sidecar_path(job["output"]))
    table = read_json(clone / job["output"])
    provenance = read_json(clone / sidecar["path"])
    require(isinstance(table, dict) and set(table) == {"schema", "base_vocab_size", "merges", "metadata"}
            and isinstance(provenance, dict), "unexpected output JSON structure")
    fixture_records = sorted([file_record(clone, x["path"]) for x in dataset["fixtures"]], key=lambda x: x["path"])
    for actual, pin in zip(fixture_records, sorted(dataset["fixtures"], key=lambda x: x["path"])):
        require(actual["sha256"] == pin["sha256"], "fixture changed during training")
    schema_record = file_record(clone, dataset["schema"])
    inputs = sorted([schema_record, *fixture_records], key=lambda x: x["path"])
    expected_fields = {
        "schema_name": job["dataset"], "instances_pattern": dataset["instances_pattern"],
        "schema_file": schema_record, "input_files": inputs, "input_corpus_fingerprint": input_fingerprint(inputs),
        "output_files": [output], "repo_commit": expected, "repo_dirty": False,
        "optimization_level": job["optimization"], "target_vocab": job["target_vocab"],
        "base_vocab_source": "schema", "schema_ids": [], "reserved_special_tokens": 2,
        "trained_on_instances": len(fixture_records), "provenance_path": sidecar["path"],
        "encoded_input": None, "encoded_input_file": None, "prepare_data_metadata_file": None,
        "cache_path": None, "cache_file": None, "target_avg_length": None, "sample_size": None,
        "max_merges_deprecated": None, "checkpoints": None,
    }
    for key, value in expected_fields.items():
        require(key in provenance and type(provenance[key]) is type(value) and provenance[key] == value,
                "provenance mismatch: " + key)
    require(provenance["repo_dirty"] is False, "dirty trainer source rejected")
    corpus = provenance.get("token_corpus")
    require(isinstance(corpus, dict), "missing token corpus provenance")
    expected_corpus = {
        "source": "instance_encoding", "sequence_count": len(fixture_records),
        "sample_size": None, "sample_seed": None, "cache_path": None,
        "cache_hit": False, "cache_written": False, "parallel_early_stop": False,
        "truncated_to_sample_size": False, "selected_instance_files": [], "rejected_instance_files": [],
    }
    for key, value in expected_corpus.items():
        require(key in corpus and type(corpus[key]) is type(value) and corpus[key] == value,
                "token corpus mismatch: " + key)
    require(sorted(corpus.get("accepted_instance_files", []), key=lambda x: x["path"]) == fixture_records,
            "not every authored fixture was encoded")
    require(isinstance(corpus.get("sequence_fingerprint"), str) and re.fullmatch(r"[0-9a-f]{64}", corpus["sequence_fingerprint"]),
            "invalid token corpus fingerprint")
    base, merges = table.get("base_vocab_size"), table.get("merges")
    require(type(base) is int and 0 < base < 2**32 and isinstance(merges, list), "invalid merge table")
    require(table.get("schema") == job["dataset"] and type(provenance.get("base_vocab_size")) is int
            and provenance["base_vocab_size"] == base,
            "merge table schema or base mismatch")
    require(len(merges) <= max(0, job["target_vocab"] - base - 3), "merge budget exceeded")
    for index, merge in enumerate(merges):
        require(isinstance(merge, dict) and set(merge) == {"pair", "new_token", "frequency"}, "invalid merge record")
        require(type(merge["new_token"]) is int and merge["new_token"] == base + index, "nonsequential BPE token ID")
        require(type(merge["frequency"]) is int and 0 < merge["frequency"] < 2**64, "invalid merge frequency")
        pair = merge.get("pair")
        require(isinstance(pair, list) and len(pair) == 2 and all(type(x) is int and 0 <= x < base + index for x in pair),
                "invalid BPE pair references")
    metadata = table.get("metadata")
    require(isinstance(metadata, dict) and set(metadata) == {"trained_on_instances", "training_date", "max_merges", "dict_size"}
            and metadata.get("training_date") == training_date,
            "training date mismatch")
    require(type(metadata["max_merges"]) is int and metadata["max_merges"] == len(merges)
            and type(metadata["trained_on_instances"]) is int and metadata["trained_on_instances"] == len(fixture_records)
            and metadata["dict_size"] is None and type(provenance.get("generated_merges")) is int
            and provenance["generated_merges"] == len(merges),
            "merge metadata mismatch")
    return {"output": output, "provenance": sidecar, "original_training_commit": expected,
            "input_count": len(fixture_records), "requested_vocab_budget": job["target_vocab"],
            "base_vocab_size": base, "actual_merges": len(merges),
            "actual_raw_vocab_size_with_pad": base + len(merges) + 1,
            "actual_training_vocab_with_pad_bos_eos": base + len(merges) + 3,
            "actual_single_schema_sequence_vocab": base + len(merges) + 4,
            "below_requested_budget": base + len(merges) + 3 < job["target_vocab"],
            "token_corpus_fingerprint": corpus["sequence_fingerprint"]}


def artifact_paths():
    return set(EXPECTED_OUTPUTS) | {sidecar_path(path) for path in EXPECTED_OUTPUTS}


def snapshot_digest(records):
    payload = json.dumps(records, sort_keys=True, separators=(",", ":"), ensure_ascii=True).encode()
    return sha256(SNAPSHOT_FORMAT.encode() + b"\n" + payload + b"\n")


def source_snapshot(root):
    root = Path(root).resolve()
    excluded = artifact_paths() | {MANIFEST_PATH}
    records = []
    def walk_error(error):
        raise error

    for directory, dirs, files in os.walk(root, followlinks=False, onerror=walk_error):
        relative_dir = Path(directory).relative_to(root)
        # Reject symlinks even when they use an otherwise excluded cache name.
        for name in dirs + files:
            require(not (Path(directory) / name).is_symlink(), "source symlink rejected: " + str(relative_dir / name))
        dirs[:] = sorted(name for name in dirs if name not in {"__pycache__", ".pytest_cache"}
                         and not (relative_dir == Path(".") and name in ROOT_CACHE_DIRS))
        for name in sorted(files):
            relative = (relative_dir / name).as_posix()
            if relative in excluded or (relative_dir == Path(".") and name == ".git"):
                continue
            path = root / relative
            mode = path.stat().st_mode
            require(stat.S_ISREG(mode), "source is not a regular file: " + relative)
            records.append({**file_record(root, relative), "mode": "100755" if mode & 0o111 else "100644"})
    records.sort(key=lambda row: row["path"])
    return {"format": SNAPSHOT_FORMAT, "sha256": snapshot_digest(records), "files": records}


def verify_snapshot(root, snapshot):
    require(isinstance(snapshot, dict) and set(snapshot) == {"format", "sha256", "files"}, "invalid source snapshot")
    require(snapshot["format"] == SNAPSHOT_FORMAT and isinstance(snapshot["files"], list), "unknown source snapshot format")
    seen = []
    for row in snapshot["files"]:
        require(isinstance(row, dict) and set(row) == {"path", "mode", "bytes", "sha256"}, "invalid source snapshot record")
        seen.append(safe_path(row["path"]))
        require(row["mode"] in ("100644", "100755") and type(row["bytes"]) is int and row["bytes"] >= 0
                and re.fullmatch(r"[0-9a-f]{64}", row["sha256"]), "invalid source snapshot metadata")
    require(seen == sorted(set(seen)), "duplicate or unsorted source snapshot paths")
    require(snapshot["sha256"] == snapshot_digest(snapshot["files"]), "source snapshot digest differs")
    current = source_snapshot(root)
    if current != snapshot:
        expected = {row["path"]: row for row in snapshot["files"]}
        actual = {row["path"]: row for row in current["files"]}
        changed = sorted(p for p in expected.keys() | actual.keys() if expected.get(p) != actual.get(p))
        raise ValueError("source snapshot differs: " + ", ".join(changed[:20]))
    return current


def validate_manifest(root, *, check_source=True):
    root = Path(root).resolve()
    manifest = read_json(root / MANIFEST_PATH)
    require(manifest.get("version") == 2, "unsupported public BPE manifest version")
    original = manifest.get("original_training", {})
    require(original.get("role") == "archival_training_record", "original training must be marked archival")
    for key in ("commit", "tree"):
        require(re.fullmatch(r"[0-9a-f]{40}", original.get(key, "")), "invalid original training " + key)
    require(re.fullmatch(r"[0-9a-f]{64}", original.get("receipt_sha256", "")), "invalid archival receipt hash")
    spec = load_spec(root)
    require(file_record(root, SPEC_PATH)["sha256"] == manifest.get("training_spec_sha256"), "training spec hash differs")
    expected_paths = artifact_paths()
    require({p.relative_to(root).as_posix() for p in (root / "schemas/bpe").glob("*.json")} == expected_paths,
            "public BPE directory must contain exactly six tables and six sidecars")
    files = manifest.get("files")
    require(isinstance(files, list) and len(files) == 12, "expected twelve public BPE file records")
    require({row.get("path") for row in files} == expected_paths, "public BPE manifest path coverage differs")
    for row in files:
        require(row == file_record(root, row["path"]), "public BPE file hash mismatch: " + row["path"])
    expected_artifacts = [validate_outputs(root, job, spec["datasets"][job["dataset"]], original["commit"],
                                           original["training_date"]) for job in spec["artifacts"]]
    require(manifest.get("artifacts") == expected_artifacts, "public BPE artifact review differs")
    if check_source:
        verify_snapshot(root, manifest.get("source_snapshot"))
    return manifest


def write_snapshot(root):
    root = Path(root).resolve()
    manifest = validate_manifest(root, check_source=False)
    manifest["source_snapshot"] = source_snapshot(root)
    (root / MANIFEST_PATH).write_text(json.dumps(manifest, indent=2) + "\n", encoding="utf-8")
    verify_snapshot(root, manifest["source_snapshot"])
    return manifest["source_snapshot"]


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--root", type=Path, default=Path(__file__).resolve().parents[2])
    parser.add_argument("--write", action="store_true", help="Refresh the source snapshot after reviewed source edits; then rerun all six recipes.")
    args = parser.parse_args()
    try:
        snapshot = write_snapshot(args.root) if args.write else validate_manifest(args.root)["source_snapshot"]
    except (ValueError, OSError, KeyError, TypeError) as error:
        parser.exit(1, f"public BPE source manifest: {error}\n")
    print(json.dumps({"source_sha256": snapshot["sha256"], "source_files": len(snapshot["files"]),
                      "artifacts": 6, "snapshot_refreshed": args.write}, sort_keys=True))


if __name__ == "__main__":
    main()
