#!/usr/bin/env python3
"""Validate structure-only BPE merge tables against role bitmaps.

Measures compression of structure-only BPE against base (no-BPE) TCT token
streams and unconstrained full BPE at the same target vocabulary, and checks:

(a) no structure-only merge application spans a value / value-end role;
(c) digit-digit and ASCII letter pairs do not appear in the structure-only
    merge table (value payload fingerprint stays 1 token/char like base).

This is a compression / merge-policy measurement only. It does not train or
evaluate models and is not paper-grade by itself.
"""

from __future__ import annotations

import argparse
import json
import sys
from collections import Counter
from pathlib import Path
from typing import Any


TOOL_NAME = "structure_only_bpe_measurement"
ARTIFACT_KIND = "tct_structure_only_bpe_measurement_v1"

ROLE_STRUCTURE = 0
ROLE_VALUE = 1
ROLE_VALUE_END = 2
ROLE_DISCRIMINATOR = 3
MERGEABLE_ROLES = {ROLE_STRUCTURE, ROLE_DISCRIMINATOR}


def load_json(path: Path) -> Any:
    return json.loads(path.read_text(encoding="utf-8"))


def load_jsonl(path: Path) -> list[Any]:
    rows: list[Any] = []
    with path.open(encoding="utf-8") as handle:
        for line in handle:
            line = line.strip()
            if line:
                rows.append(json.loads(line))
    return rows


def as_sequence(row: Any) -> list[int]:
    if isinstance(row, list):
        return [int(x) for x in row]
    if isinstance(row, dict):
        for key in ("tokens", "roles", "ids", "sequence"):
            if key in row and isinstance(row[key], list):
                return [int(x) for x in row[key]]
        if len(row) == 1:
            only = next(iter(row.values()))
            if isinstance(only, list):
                return [int(x) for x in only]
    raise SystemExit(f"error: expected token/role sequence list, got {type(row)!r}")


def merge_new_token(merge: dict[str, Any], fallback_id: int) -> int:
    for key in ("new_token", "id", "token_id"):
        if key in merge:
            return int(merge[key])
    return fallback_id


def merge_pair(merge: dict[str, Any]) -> tuple[int, int]:
    pair = merge["pair"]
    return int(pair[0]), int(pair[1])


def ordered_encode_len(seq: list[int], merges: list[dict[str, Any]], base_vocab: int) -> int:
    ids = list(seq)
    for index, merge in enumerate(merges):
        a, b = merge_pair(merge)
        new_id = merge_new_token(merge, base_vocab + index)
        out: list[int] = []
        i = 0
        while i < len(ids):
            if i + 1 < len(ids) and ids[i] == a and ids[i + 1] == b:
                out.append(new_id)
                i += 2
            else:
                out.append(ids[i])
                i += 1
        ids = out
    return len(ids)


def validate_no_value_span(
    token_seqs: list[list[int]],
    role_seqs: list[list[int]],
    merges: list[dict[str, Any]],
    base_vocab: int,
) -> dict[str, int]:
    violations = 0
    checked = 0
    for tokens, roles in zip(token_seqs, role_seqs):
        ids = list(tokens)
        roles_cur = list(roles)
        for index, merge in enumerate(merges):
            a, b = merge_pair(merge)
            new_id = merge_new_token(merge, base_vocab + index)
            out_ids: list[int] = []
            out_roles: list[int] = []
            i = 0
            while i < len(ids):
                if i + 1 < len(ids) and ids[i] == a and ids[i + 1] == b:
                    ra, rb = roles_cur[i], roles_cur[i + 1]
                    checked += 1
                    if ra not in MERGEABLE_ROLES or rb not in MERGEABLE_ROLES:
                        violations += 1
                    new_role = (
                        ROLE_STRUCTURE
                        if ra in MERGEABLE_ROLES and rb in MERGEABLE_ROLES
                        else ROLE_VALUE
                    )
                    out_ids.append(new_id)
                    out_roles.append(new_role)
                    i += 2
                else:
                    out_ids.append(ids[i])
                    out_roles.append(roles_cur[i])
                    i += 1
            ids, roles_cur = out_ids, out_roles
    return {"merge_applications_checked": checked, "value_span_violations": violations}


def value_fingerprint_stats(
    token_seqs: list[list[int]],
    role_seqs: list[list[int]],
    structure_only_merges: list[dict[str, Any]],
    full_merges: list[dict[str, Any]],
) -> dict[str, int]:
    digit_digit_value_pairs = 0
    for tokens, roles in zip(token_seqs, role_seqs):
        for i in range(len(tokens) - 1):
            if (
                roles[i] == ROLE_VALUE
                and roles[i + 1] == ROLE_VALUE
                and 48 <= tokens[i] <= 57
                and 48 <= tokens[i + 1] <= 57
            ):
                digit_digit_value_pairs += 1

    so_pairs = {merge_pair(m) for m in structure_only_merges}
    full_pairs = {merge_pair(m) for m in full_merges}
    digit_digit_in_so = sum(
        1 for a in range(48, 58) for b in range(48, 58) if (a, b) in so_pairs
    )
    ascii_letter_in_so = sum(
        1
        for a, b in so_pairs
        if 65 <= a <= 122 and 65 <= b <= 122 and chr(a).isalpha() and chr(b).isalpha()
    )
    ascii_letter_in_full = sum(
        1
        for a, b in full_pairs
        if 65 <= a <= 122 and 65 <= b <= 122 and chr(a).isalpha() and chr(b).isalpha()
    )
    return {
        "digit_digit_adjacent_value_pairs_in_corpus": digit_digit_value_pairs,
        "digit_digit_pairs_in_structure_only_table": digit_digit_in_so,
        "ascii_letter_pairs_in_structure_only_table": ascii_letter_in_so,
        "ascii_letter_pairs_in_full_table": ascii_letter_in_full,
    }


def measure(
    tokens_path: Path,
    roles_path: Path,
    structure_only_merges_path: Path,
    full_merges_path: Path,
    schema: str | None = None,
) -> dict[str, Any]:
    token_rows = load_jsonl(tokens_path)
    role_rows = load_jsonl(roles_path)
    if len(token_rows) != len(role_rows):
        raise SystemExit(
            f"error: token/role count mismatch: {len(token_rows)} vs {len(role_rows)}"
        )
    token_seqs = [as_sequence(row) for row in token_rows]
    role_seqs = [as_sequence(row) for row in role_rows]
    for index, (tok, role) in enumerate(zip(token_seqs, role_seqs)):
        if len(tok) != len(role):
            raise SystemExit(
                f"error: sequence {index} token/role length mismatch: "
                f"{len(tok)} vs {len(role)}"
            )

    so_table = load_json(structure_only_merges_path)
    full_table = load_json(full_merges_path)
    so_merges = list(so_table["merges"])
    full_merges = list(full_table["merges"])
    base_vocab = int(so_table.get("base_vocab_size") or full_table.get("base_vocab_size") or 257)

    role_hist = Counter(role for seq in role_seqs for role in seq)
    value_positions = role_hist[ROLE_VALUE] + role_hist[ROLE_VALUE_END]
    mergeable_positions = role_hist[ROLE_STRUCTURE] + role_hist[ROLE_DISCRIMINATOR]
    total_positions = value_positions + mergeable_positions

    span = validate_no_value_span(token_seqs, role_seqs, so_merges, base_vocab)
    fingerprint = value_fingerprint_stats(
        token_seqs, role_seqs, so_merges, full_merges
    )

    n = len(token_seqs)
    base_total = sum(len(seq) for seq in token_seqs)
    so_total = sum(ordered_encode_len(seq, so_merges, base_vocab) for seq in token_seqs)
    full_total = sum(
        ordered_encode_len(seq, full_merges, int(full_table.get("base_vocab_size", base_vocab)))
        for seq in token_seqs
    )

    acceptance = {
        "a_no_value_span_merges": span["value_span_violations"] == 0,
        "c_value_fingerprint_preserved": (
            fingerprint["digit_digit_pairs_in_structure_only_table"] == 0
            and fingerprint["ascii_letter_pairs_in_structure_only_table"] == 0
        ),
        "b_structure_only_between_base_and_full": base_total >= so_total >= full_total,
    }
    acceptance["all_passed"] = all(acceptance.values())

    return {
        "artifact_kind": ARTIFACT_KIND,
        "tool": TOOL_NAME,
        "schema": schema,
        "instances": n,
        "base_vocab_size": base_vocab,
        "structure_only_merges": len(so_merges),
        "full_merges": len(full_merges),
        "inputs": {
            "tokens": str(tokens_path),
            "roles": str(roles_path),
            "structure_only_merges": str(structure_only_merges_path),
            "full_merges": str(full_merges_path),
        },
        "role_histogram": {str(k): int(v) for k, v in sorted(role_hist.items())},
        "value_position_share": (
            value_positions / total_positions if total_positions else None
        ),
        "token_totals": {
            "base": base_total,
            "structure_only": so_total,
            "full_bpe": full_total,
        },
        "avg_tokens": {
            "base": base_total / n if n else None,
            "structure_only": so_total / n if n else None,
            "full_bpe": full_total / n if n else None,
        },
        "ratios": {
            "structure_only_over_base": so_total / base_total if base_total else None,
            "full_over_base": full_total / base_total if base_total else None,
            "structure_only_over_full": so_total / full_total if full_total else None,
        },
        "span_check": span,
        "value_fingerprint": fingerprint,
        "acceptance": acceptance,
        "claim_boundary": (
            "Structure-only BPE merge-policy and compression measurement over "
            "TCT base-token sequences with parallel role bitmaps. Not model "
            "training/evaluation, not host-tokenizer parity, not paper-grade "
            "by itself."
        ),
    }


def render_markdown(report: dict[str, Any]) -> str:
    acc = report["acceptance"]
    avg = report["avg_tokens"]
    ratios = report["ratios"]
    lines = [
        "# Structure-Only BPE Compression Measurement",
        "",
        f"Tool: `{report['tool']}` (`{report['artifact_kind']}`).",
        "",
        "This is a repo-local compression measurement. It is not paper evidence",
        "and does not update downstream model or paper claims.",
        "",
        "## Corpus",
        "",
        f"- Schema: `{report.get('schema')}`",
        f"- Instances: {report['instances']}",
        f"- Base vocab: {report['base_vocab_size']}",
        f"- Structure-only merges: {report['structure_only_merges']}",
        f"- Full BPE merges: {report['full_merges']}",
        f"- Value-position share: {report['value_position_share']!r}",
        "",
        "## Acceptance",
        "",
        f"- (a) no value-span merges: **{acc['a_no_value_span_merges']}** "
        f"({report['span_check']['value_span_violations']} violations / "
        f"{report['span_check']['merge_applications_checked']} applications)",
        f"- (b) structure-only length between base and full: "
        f"**{acc['b_structure_only_between_base_and_full']}**",
        f"- (c) value fingerprint preserved (no digit-digit or ASCII letter "
        f"pairs in structure-only table): **{acc['c_value_fingerprint_preserved']}**",
        f"- All passed: **{acc['all_passed']}**",
        "",
        "## Compression",
        "",
        f"| Mode | Avg tokens | Total | Ratio vs base |",
        f"| --- | ---: | ---: | ---: |",
        f"| Base (no BPE) | {avg['base']:.2f} | {report['token_totals']['base']} | 1.000 |",
        f"| Structure-only BPE | {avg['structure_only']:.2f} | "
        f"{report['token_totals']['structure_only']} | "
        f"{ratios['structure_only_over_base']:.3f} |",
        f"| Full BPE | {avg['full_bpe']:.2f} | {report['token_totals']['full_bpe']} | "
        f"{ratios['full_over_base']:.3f} |",
        "",
        f"Structure-only / full = {ratios['structure_only_over_full']:.3f}.",
        "",
        "## Claim boundary",
        "",
        report["claim_boundary"],
        "",
    ]
    return "\n".join(lines)


def self_test() -> None:
    # Two sequences: structure pair (10,11) is mergeable; value pair (1,2) is not.
    tokens = [
        [10, 11, 1, 2],
        [1, 2, 1, 2],
        [10, 11, 10, 11],
    ]
    roles = [
        [ROLE_STRUCTURE, ROLE_STRUCTURE, ROLE_VALUE, ROLE_VALUE],
        [ROLE_VALUE, ROLE_VALUE, ROLE_VALUE, ROLE_VALUE],
        [ROLE_STRUCTURE, ROLE_STRUCTURE, ROLE_STRUCTURE, ROLE_STRUCTURE],
    ]
    structure_only = {
        "base_vocab_size": 20,
        "merges": [{"pair": [10, 11], "new_token": 20, "frequency": 3}],
    }
    full = {
        "base_vocab_size": 20,
        "merges": [
            {"pair": [1, 2], "new_token": 20, "frequency": 3},
            {"pair": [10, 11], "new_token": 21, "frequency": 3},
        ],
    }
    tmp = Path("/tmp/tct-structure-only-self-test")
    tmp.mkdir(parents=True, exist_ok=True)
    tokens_path = tmp / "all.jsonl"
    roles_path = tmp / "all.roles.jsonl"
    so_path = tmp / "so.json"
    full_path = tmp / "full.json"
    tokens_path.write_text("\n".join(json.dumps(s) for s in tokens) + "\n", encoding="utf-8")
    roles_path.write_text("\n".join(json.dumps(s) for s in roles) + "\n", encoding="utf-8")
    so_path.write_text(json.dumps(structure_only), encoding="utf-8")
    full_path.write_text(json.dumps(full), encoding="utf-8")

    report = measure(tokens_path, roles_path, so_path, full_path, schema="self-test")
    assert report["acceptance"]["all_passed"], report
    assert report["token_totals"]["base"] == 12
    assert report["token_totals"]["structure_only"] == 9  # three (10,11) pairs collapsed
    assert report["token_totals"]["full_bpe"] == 6  # all three (1,2) and three (10,11)
    assert report["span_check"]["value_span_violations"] == 0
    assert report["value_fingerprint"]["ascii_letter_pairs_in_structure_only_table"] == 0
    print("self-test passed")


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--self-test", action="store_true")
    parser.add_argument("--tokens", type=Path, help="all.jsonl from tct-prepare-data")
    parser.add_argument("--roles", type=Path, help="all.roles.jsonl from --export-roles")
    parser.add_argument(
        "--structure-only-merges",
        type=Path,
        help="merge table trained with --structure-only-merges",
    )
    parser.add_argument(
        "--full-merges",
        type=Path,
        help="unconstrained merge table at the same target vocab",
    )
    parser.add_argument("--schema", type=str, default=None)
    parser.add_argument("--output", type=Path, default=None, help="JSON report path")
    parser.add_argument(
        "--markdown-output", type=Path, default=None, help="Markdown report path"
    )
    args = parser.parse_args(argv)

    if args.self_test:
        self_test()
        return 0

    missing = [
        name
        for name, value in (
            ("--tokens", args.tokens),
            ("--roles", args.roles),
            ("--structure-only-merges", args.structure_only_merges),
            ("--full-merges", args.full_merges),
        )
        if value is None
    ]
    if missing:
        parser.error("required arguments: " + ", ".join(missing))

    report = measure(
        args.tokens,
        args.roles,
        args.structure_only_merges,
        args.full_merges,
        schema=args.schema,
    )
    text = json.dumps(report, indent=2, sort_keys=False) + "\n"
    if args.output:
        args.output.parent.mkdir(parents=True, exist_ok=True)
        args.output.write_text(text, encoding="utf-8")
    else:
        sys.stdout.write(text)

    if args.markdown_output:
        args.markdown_output.parent.mkdir(parents=True, exist_ok=True)
        args.markdown_output.write_text(render_markdown(report) + "\n", encoding="utf-8")

    if not report["acceptance"]["all_passed"]:
        print("acceptance failed", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
