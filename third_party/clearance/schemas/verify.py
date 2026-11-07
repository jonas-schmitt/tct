#!/usr/bin/env python3
"""Verify the hash-bound schema review; optionally rederive upstream matches."""

import argparse
import copy
import hashlib
import json
from pathlib import Path
import re
import subprocess
import sys
import tempfile

sys.path.insert(0, str(Path(__file__).resolve().parents[3] / "scripts/analysis"))
from public_bpe_source_manifest import validate_manifest


def sha256(data):
    return hashlib.sha256(data).hexdigest()


def require(condition, message):
    if not condition:
        raise ValueError(message)


def git_file(repository, revision, path):
    require(re.fullmatch(r"[0-9a-f]{40}", revision), f"Non-immutable pin: {revision}")
    return subprocess.check_output(["git", "-C", str(repository), "show", f"{revision}:{path}"])


def apply_changes(document, operations):
    document = copy.deepcopy(document)
    for operation in operations:
        parts = [p.replace("~1", "/").replace("~0", "~") for p in operation["path"].split("/")[1:]]
        parent = document
        for part in parts[:-1]:
            parent = parent[int(part)] if isinstance(parent, list) else parent[part]
        key = int(parts[-1]) if isinstance(parent, list) else parts[-1]
        if operation["op"] == "remove":
            del parent[key]
        elif operation["op"] in ("add", "replace"):
            parent[key] = operation["value"]
        else:
            raise ValueError(f"Unsupported operation: {operation['op']}")
    return document


def add_notice(data, notice):
    document = json.loads(data)
    old = notice["previous_comment"]
    require(document.get("$comment") == old, "Original comment differs")
    document["$comment"] = (old + "\n\n" if old else "") + notice["added_text"]
    result = (json.dumps(document, indent=2, ensure_ascii=False) + "\n").encode()
    require(sha256(result) == notice["after_sha256"], "Notice transformation hash differs")
    return result


def before_notice(document, notice):
    document = copy.deepcopy(document)
    if notice["previous_comment"] is None:
        del document["$comment"]
    else:
        document["$comment"] = notice["previous_comment"]
    return document


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--root", type=Path, default=Path(__file__).resolve().parents[3])
    parser.add_argument("--schemastore", type=Path)
    parser.add_argument("--extractor", type=Path)
    parser.add_argument("--research", type=Path, help="Optional preserved private history for authorship receipts")
    parser.add_argument("--kubernetes-definitions", type=Path, help="Pinned upstream Kubernetes 1.30 definitions JSON")
    parser.add_argument("--babel-website", type=Path)
    parser.add_argument("--cypress-schema", type=Path, help="Pinned original Cypress CLI schema")
    parser.add_argument("--gitlab-schema", type=Path, help="Pinned original GitLab CI schema")
    parser.add_argument("--public-scope", action="store_true", help="Require pending schemas absent; qualify fresh BPE files by their separate manifest")
    args = parser.parse_args()
    directory = args.root / "third_party/clearance/schemas"
    ledger = json.loads((directory / "embedded-schemas.json").read_text())
    catalog = json.loads((directory / "catalog-inventory.json").read_text())
    inputs = json.loads((directory / "eslint-plugin-inputs.json").read_text())
    owned = json.loads((directory / "owner-authored.json").read_text())
    derived = json.loads((directory / "derived-schemas.json").read_text())
    kubernetes = json.loads((directory / "kubernetes-schemas.json").read_text())
    catalog_review = json.loads((directory / "schemastore-catalog.json").read_text())
    additional = json.loads((directory / "catalog-additional-sources.json").read_text())
    pending = json.loads((directory / "pending-schemas.json").read_text())
    frameworks = json.loads((directory / "retained-framework-schemas.json").read_text())
    schemas = ledger["files"]
    expected = {row["local_path"] for row in schemas}
    resolver = (args.root / "src/generator/external_schemas.rs").read_text()
    included = set(re.findall(r'include_str!\("../../(schemas/[^\"]+)"\)', resolver))
    require(expected == included and len(expected) == 7, "Embedded resolver scope differs from seven-file ledger")
    actual_paths = {str(p.relative_to(args.root)) for p in (args.root / "schemas").rglob("*.json")}
    inventory_paths = {row["local_path"] for row in catalog["files"]}
    reviewed_files = schemas + owned["files"] + derived["files"] + kubernetes["files"] + catalog_review["files"] + frameworks["files"]
    reviewed_paths = [row["local_path"] for row in reviewed_files]
    pending_paths = {row["local_path"] for row in pending["files"]}
    require(len(reviewed_paths) == len(set(reviewed_paths)), "Overlapping clearance groups")
    require(not set(reviewed_paths) & pending_paths, "A path is both reviewed and pending")
    require(set(reviewed_paths) | pending_paths == inventory_paths, "Incomplete clearance classification")
    fresh_bpe_paths = set()
    if args.public_scope:
        pending_non_bpe = {path for path in pending_paths if not path.startswith("schemas/bpe/")}
        require(not actual_paths & pending_non_bpe, "Uncleared schemas remain in public scope: " + ", ".join(sorted(actual_paths & pending_non_bpe)))
        bpe_paths = {path for path in actual_paths if path.startswith("schemas/bpe/")}
        bpe_manifest_path = args.root / "third_party/clearance/bpe-artifacts.json"
        require(bpe_manifest_path.is_file(), "Public scope requires third_party/clearance/bpe-artifacts.json")
        bpe = validate_manifest(args.root)
        fresh_bpe_paths = {row["path"] for row in bpe["files"]}
        require(fresh_bpe_paths == bpe_paths, "Public BPE manifest does not exactly cover retained BPE JSON files")
        require(actual_paths == set(reviewed_paths) | fresh_bpe_paths, "Missing reviewed schema or unreviewed extra JSON in public scope")
    else:
        require(actual_paths == inventory_paths, "Schema catalog path inventory changed")
    for row in reviewed_files + ([] if args.public_scope else catalog["files"]):
        raw = (args.root / row["local_path"]).read_bytes()
        require(sha256(raw) == row["local_sha256"], f"Local hash mismatch: {row['local_path']}")
    notice_by_path = {row["local_path"]: row for row in ledger["notice_files"] + kubernetes["notice_files"] + additional["notice_files"] + frameworks["notice_files"]}
    for path, row in notice_by_path.items():
        require(sha256((args.root / path).read_bytes()) == row["sha256"], f"Notice hash mismatch: {path}")
    for row in reviewed_files:
        for path in row.get("retained_notices", []):
            require(path in notice_by_path, f"Unrecorded retained notice: {path}")
        if "change_notice" in row:
            require((args.root / row["change_notice"]).is_file(), "Missing modification notice")
    for row in schemas:
        if args.schemastore:
            upstream = git_file(args.schemastore, row["source_revision"], row["source_path"])
            require(sha256(upstream) == row["source_sha256"], f"Source hash mismatch: {row['source_path']}")
            reconstructed = upstream
            transformation = row.get("source_copy_transformation", row["transformation"])
            if transformation == "python-json-indent4-ascii-newline":
                reconstructed = (json.dumps(json.loads(upstream), indent=4, ensure_ascii=True) + "\n").encode("utf-8")
            else:
                require(transformation == "identity", "Unknown source transformation")
            if "publication_notice" in row:
                require(sha256(reconstructed) == row["before_publication_notice_sha256"], "Pre-notice transformation mismatch")
                reconstructed = add_notice(reconstructed, row["publication_notice"])
            require(sha256(reconstructed) == row["local_sha256"], f"Transformation mismatch: {row['local_path']}")
            for name in ("LICENSE", "NOTICE"):
                original = git_file(args.schemastore, row["source_revision"], name)
                require(original == (args.root / "third_party/licenses/schemastore" / name).read_bytes(), f"Historical notice mismatch: {name}")
    if args.schemastore:
        for revision in {row["source_revision"] for row in catalog_review["files"]}:
            for name in ("LICENSE", "NOTICE"):
                require(git_file(args.schemastore, revision, name) == (args.root / "third_party/licenses/schemastore" / name).read_bytes(), "Historical catalog notice mismatch")
        for row in catalog["files"]:
            if row.get("source_repository") != "https://github.com/SchemaStore/schemastore":
                continue
            if args.public_scope and row["local_path"] not in set(reviewed_paths):
                continue
            if row["match"] not in ("identical", "json_value_equal", "publication_notice_added") or "source_path" not in row:
                continue
            upstream = git_file(args.schemastore, row.get("source_revision", catalog["source_revision"]), row["source_path"])
            require(sha256(upstream) == row["source_sha256"], "Catalog upstream hash mismatch")
            local = (args.root / row["local_path"]).read_bytes()
            if "publication_notice" in row:
                require(before_notice(json.loads(local), row["publication_notice"]) == json.loads(upstream), "Catalog JSON before notice differs")
                profile = row["upstream_to_pre_notice_serialization"]
                if profile["kind"] == "identity":
                    before = upstream
                else:
                    require(profile["kind"] == "json-reserialize", "Unknown catalog source serialization")
                    before = (json.dumps(json.loads(upstream), indent=profile["indent"], ensure_ascii=profile["ensure_ascii"]) + ("\n" if profile["final_newline"] else "")).encode()
                require(sha256(before) == row["publication_notice"]["before_sha256"], "Catalog pre-notice byte reconstruction differs")
                require(add_notice(before, row["publication_notice"]) == local, "Catalog notice differs")
            else:
                require(local == upstream if row["match"] == "identical" else json.loads(local) == json.loads(upstream), "Catalog match differs")
        for row in derived["files"]:
            original = git_file(args.schemastore, row["source_revision"], row["source_path"])
            require(sha256(original) == row["source_sha256"], "Derived schema source hash differs")
            source = json.loads(original)
            transformation = row["transformation"]
            if transformation["kind"] == "selected-package-properties":
                reconstructed = {
                    key: ({name: source["properties"][name] for name in transformation["selected_properties"]}
                          if key == "properties" else transformation["title"] if key == "title" else source[key])
                    for key in transformation["top_level_keys"]
                }
                raw = json.dumps(reconstructed, indent=2).encode()
                require(sha256(raw) == row["before_publication_notice_sha256"], "Package subset reconstruction differs")
                require(sha256(add_notice(raw, row["publication_notice"])) == row["local_sha256"], "Package publication notice differs")
            else:
                require(transformation["kind"] == "owner-compatibility-changes", "Unknown derivation")
                patch = json.loads((args.root / transformation["json_patch"]).read_text())
                reconstructed = apply_changes(source, patch["operations"])
                local = json.loads((args.root / row["local_path"]).read_bytes())
                require(reconstructed == before_notice(local, row["publication_notice"]), "Workflow JSON patch differs")
                byte_patch = (args.root / transformation["byte_patch"]).read_bytes()
                require(sha256(byte_patch) == transformation["byte_patch_sha256"], "Workflow byte patch hash differs")
                with tempfile.TemporaryDirectory(prefix="tct-schema-patch-") as scratch:
                    target = Path(scratch) / "github-workflow.json"
                    target.write_bytes(original)
                    subprocess.run(["patch", "-p1", "--batch", "-i", str((args.root / transformation["byte_patch"]).resolve())], cwd=scratch, check=True, stdout=subprocess.DEVNULL)
                    require(sha256(target.read_bytes()) == row["before_publication_notice_sha256"], "Workflow byte patch differs")
                    require(sha256(add_notice(target.read_bytes(), row["publication_notice"])) == row["local_sha256"], "Workflow publication notice differs")
    if args.extractor:
        plugin = next(row for row in schemas if "source_chain" in row)
        chain = plugin["source_chain"]
        original = git_file(args.extractor, chain["extractor_revision"], chain["extractor_path"])
        require(sha256(original) == chain["extractor_schema_sha256"], "Extractor schema hash mismatch")
        changes = json.loads((directory / "eslint-schema-changes.json").read_text())
        reconstructed = apply_changes(json.loads(original), changes["operations"])
        local_document = json.loads((args.root / plugin["local_path"]).read_bytes())
        if "publication_notice" in plugin:
            local_document = before_notice(local_document, plugin["publication_notice"])
        require(reconstructed == local_document, "Extractor-to-local JSON transformation differs")
        original_license = git_file(args.extractor, chain["extractor_revision"], "LICENSE")
        require(original_license == (args.root / "third_party/licenses/jsonschema-extractor/LICENSE").read_bytes(), "Extractor license differs")
        lock = git_file(args.extractor, inputs["extractor_revision"], inputs["extractor_lock_path"])
        require(sha256(lock) == inputs["extractor_lock_sha256"], "Extractor lock hash differs")
        for package in inputs["packages"]:
            require(package["tarball_integrity"] in lock.decode(), f"Package integrity missing from extractor lock: {package['name']}")
        if args.schemastore:
            regenerated = git_file(args.schemastore, chain["schemastore_regeneration_revision"], plugin["source_path"])
            require(sha256(regenerated) == chain["regenerated_schema_sha256"], "Regenerated schema hash differs")
            require(json.loads(regenerated) == json.loads(original), "Extractor output differs from SchemaStore regeneration")
    if args.research:
        for row in owned["files"]:
            original = git_file(args.research, row["introduction_commit"], row["introduction_path"])
            require(sha256(original) == row["introduction_sha256"], "Owner-authorship source receipt differs")
    if args.babel_website:
        babel = next(row for row in frameworks["files"] if "original_documentation" in row)["original_documentation"]
        original = git_file(args.babel_website, babel["initial_document_revision"], babel["document_path"])
        require(sha256(original) == babel["document_sha256"], "Babel original documentation differs")
        package = git_file(args.babel_website, babel["initial_document_revision"], babel["package_path"])
        require(sha256(package) == babel["package_sha256"] and json.loads(package)["license"] == "ISC", "Babel original ISC declaration differs")
        later = git_file(args.babel_website, babel["later_document_revision"], babel["later_document_path"])
        require(sha256(later) == babel["later_document_sha256"], "Babel later documentation differs")
        for notice in frameworks["notice_files"]:
            if notice["source_repository"] == "https://github.com/babel/website":
                require(git_file(args.babel_website, notice["source_revision"], notice["source_path"]) == (args.root / notice["local_path"]).read_bytes(), "Babel historical notice differs")
    if args.cypress_schema:
        cypress = next(row for row in frameworks["files"] if row["local_path"].endswith("/cypress.json"))
        require(sha256(args.cypress_schema.read_bytes()) == cypress["source_sha256"], "Pinned Cypress source hash differs")
        require(args.cypress_schema.read_bytes() == (args.root / cypress["local_path"]).read_bytes(), "Cypress source byte match differs")
    if args.gitlab_schema:
        gitlab = next(row for row in frameworks["files"] if row["local_path"].endswith("/gitlab-ci.json"))
        require(sha256(args.gitlab_schema.read_bytes()) == gitlab["source_sha256"], "Pinned GitLab source hash differs")
        require(args.gitlab_schema.read_bytes() == (args.root / gitlab["local_path"]).read_bytes(), "GitLab source byte match differs")
    for script in kubernetes["scripts"]:
        require(sha256((args.root / script["local_path"]).read_bytes()) == script["sha256"], "Kubernetes transformation script differs")
    if args.kubernetes_definitions:
        original = args.kubernetes_definitions.read_bytes()
        require(sha256(original) == kubernetes["source_sha256"], "Kubernetes upstream definitions hash differs")
        with tempfile.TemporaryDirectory(prefix="tct-kubernetes-schema-") as scratch:
            schema_dir = Path(scratch) / "schemas/popular"
            schema_dir.mkdir(parents=True)
            (schema_dir / "kubernetes-definitions.json").write_bytes(original)
            scripts = [args.root / script["local_path"] for script in kubernetes["scripts"]]
            subprocess.run([sys.executable, str(scripts[0].resolve())], cwd=scratch, check=True, stdout=subprocess.DEVNULL)
            (schema_dir / "kubernetes-original.json").write_bytes((schema_dir / "kubernetes.json").read_bytes())
            subprocess.run([sys.executable, str(scripts[1].resolve())], cwd=scratch, check=True, stdout=subprocess.DEVNULL)
            for row in kubernetes["files"]:
                rebuilt = (Path(scratch) / row["local_path"]).read_bytes()
                require(sha256(rebuilt) == row["reproduced_before_publication_notice_sha256"], "Kubernetes original reconstruction differs")
                require(sha256(add_notice(rebuilt, row["publication_notice"])) == row["local_sha256"], "Kubernetes publication notice differs")
    print(f"Verified {len(schemas)} embedded schemas, {len(catalog['files'])} catalog hashes and {len(notice_by_path)} notice files.")
    print(f"Classification: {len(reviewed_paths)} reviewed paths; {len(pending_paths)} pending paths.")
    if args.public_scope:
        print(f"Public scope: reviewed schemas retained; pending schemas absent; {len(fresh_bpe_paths)} fresh BPE files hash-bound to separate manifest.")
    print("Upstream rederivation: " + ("SchemaStore " if args.schemastore else "") + ("extractor" if args.extractor else "") if args.schemastore or args.extractor else "Upstream objects not re-read; local integrity only.")


if __name__ == "__main__":
    main()
