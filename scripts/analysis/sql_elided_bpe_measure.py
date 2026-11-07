#!/usr/bin/env python3
"""Measure BPE compression over SQL elided decision-token streams.

Trains (or reuses) a merge table at the max requested merge count, then reports
median/mean/p90 BPE token lengths at each merge budget by applying a truncated
merge table. Report-only; not a paper claim.
"""

from __future__ import annotations

import argparse
import json
import math
import subprocess
import sys
from pathlib import Path
from typing import Any


def percentile(sorted_vals: list[int], p: float) -> float:
    if not sorted_vals:
        return 0.0
    if len(sorted_vals) == 1:
        return float(sorted_vals[0])
    rank = p * (len(sorted_vals) - 1)
    lo = int(math.floor(rank))
    hi = int(math.ceil(rank))
    if lo == hi:
        return float(sorted_vals[lo])
    w = rank - lo
    return sorted_vals[lo] * (1.0 - w) + sorted_vals[hi] * w


def length_stats(lengths: list[int]) -> dict[str, Any]:
    if not lengths:
        return {
            "records": 0,
            "total_tokens": 0,
            "min_tokens": None,
            "max_tokens": None,
            "mean_tokens": None,
            "median_tokens": None,
            "p90_tokens": None,
        }
    ordered = sorted(lengths)
    total = sum(ordered)
    return {
        "records": len(ordered),
        "total_tokens": total,
        "min_tokens": ordered[0],
        "max_tokens": ordered[-1],
        "mean_tokens": total / len(ordered),
        "median_tokens": percentile(ordered, 0.50),
        "p90_tokens": percentile(ordered, 0.90),
    }


def load_sequences(path: Path) -> list[list[int]]:
    sequences: list[list[int]] = []
    with path.open() as f:
        for i, line in enumerate(f, 1):
            line = line.strip()
            if not line:
                continue
            tokens = json.loads(line)
            if not isinstance(tokens, list):
                raise SystemExit(f"{path}:{i}: expected JSON array of token ids")
            sequences.append([int(t) for t in tokens])
    return sequences


def write_truncated_merges(
    full_table: dict[str, Any], n_merges: int, out_path: Path
) -> dict[str, Any]:
    truncated = {
        "schema": full_table["schema"],
        "base_vocab_size": full_table["base_vocab_size"],
        "merges": full_table["merges"][:n_merges],
        "metadata": {
            **full_table.get("metadata", {}),
            "max_merges": n_merges,
            "truncated_from": full_table.get("metadata", {}).get("max_merges"),
        },
    }
    out_path.parent.mkdir(parents=True, exist_ok=True)
    out_path.write_text(json.dumps(truncated, indent=2) + "\n")
    return truncated


def main() -> int:
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument("--encoded", type=Path, required=True, help="Elided token JSONL")
    ap.add_argument(
        "--base-vocab",
        type=int,
        default=321,
        help="SQL VOCAB_SIZE / BPE base (default 321 after shared payload pool collapse)",
    )
    ap.add_argument(
        "--merges",
        type=str,
        default="256,512,1024",
        help="Comma-separated merge budgets to report",
    )
    ap.add_argument(
        "--train-bin",
        type=Path,
        default=Path("target/release/tct-train-bpe"),
        help="Path to tct-train-bpe binary",
    )
    ap.add_argument(
        "--length-stats-bin",
        type=Path,
        default=Path("target/release/tct-bpe-length-stats"),
        help="Path to tct-bpe-length-stats (SequentialBpeEncoder stats)",
    )
    ap.add_argument(
        "--work-dir",
        type=Path,
        default=Path("target/sql-elided-bpe"),
        help="Directory for merge tables and reports",
    )
    ap.add_argument(
        "--a0-median",
        type=float,
        default=17.0,
        help="A0 surface-SQL BPE median baseline",
    )
    ap.add_argument(
        "--elided-baseline-median",
        type=float,
        default=None,
        help="Optional override for elided baseline median (default: measured)",
    )
    ap.add_argument(
        "--skip-train",
        action="store_true",
        help="Reuse existing max-merge table in work-dir",
    )
    ap.add_argument(
        "--output",
        type=Path,
        default=None,
        help="Compression report JSON (default: work-dir/compression-report.json)",
    )
    args = ap.parse_args()

    merge_budgets = [int(x.strip()) for x in args.merges.split(",") if x.strip()]
    if not merge_budgets:
        raise SystemExit("no merge budgets")
    max_merges = max(merge_budgets)

    work = args.work_dir
    work.mkdir(parents=True, exist_ok=True)
    max_merge_path = work / f"sql-elided-bpe-m{max_merges}.json"
    report_path = args.output or (work / "compression-report.json")

    sequences = load_sequences(args.encoded)
    if not sequences:
        raise SystemExit(f"no sequences in {args.encoded}")
    elided_lengths = [len(s) for s in sequences]
    elided_stats = length_stats(elided_lengths)
    elided_median = (
        args.elided_baseline_median
        if args.elided_baseline_median is not None
        else elided_stats["median_tokens"]
    )

    if not args.skip_train:
        if not args.train_bin.is_file():
            raise SystemExit(f"missing train binary: {args.train_bin}")
        # Prefer exact merge count via deprecated --max-merges so budgets match the task.
        cmd = [
            str(args.train_bin),
            "--encoded",
            str(args.encoded),
            "--base-vocab",
            str(args.base_vocab),
            "--max-merges",
            str(max_merges),
            "--name",
            "sql-schema-linking-elided",
            "--output",
            str(max_merge_path),
            "--verbose",
        ]
        print("Training BPE:", " ".join(cmd), flush=True)
        subprocess.run(cmd, check=True)
    elif not max_merge_path.is_file():
        raise SystemExit(f"--skip-train but missing {max_merge_path}")

    full_table = json.loads(max_merge_path.read_text())
    trained_merges = len(full_table["merges"])
    if trained_merges < max_merges:
        print(
            f"WARNING: trainer produced only {trained_merges} merges (requested {max_merges})",
            file=sys.stderr,
        )

    if not args.length_stats_bin.is_file():
        raise SystemExit(f"missing length-stats binary: {args.length_stats_bin}")

    rows: list[dict[str, Any]] = []
    for n in merge_budgets:
        n_eff = min(n, trained_merges)
        trunc_path = work / f"sql-elided-bpe-m{n_eff}.json"
        write_truncated_merges(full_table, n_eff, trunc_path)

        mcmd = [
            str(args.length_stats_bin),
            "--encoded",
            str(args.encoded),
            "--bpe-merges",
            str(trunc_path),
        ]
        print("Measuring:", " ".join(mcmd), flush=True)
        proc = subprocess.run(mcmd, check=True, capture_output=True, text=True)
        mreport = json.loads(proc.stdout)
        (work / f"length-stats-m{n_eff}.json").write_text(
            json.dumps(mreport, indent=2) + "\n"
        )
        bpe_stats = mreport["token_length_stats"]
        median = bpe_stats["median_tokens"]
        row = {
            "requested_merges": n,
            "effective_merges": n_eff,
            "embedding_new_token_ids": n_eff,
            "merge_table": str(trunc_path),
            "bpe_length_stats": bpe_stats,
            "ratio_vs_elided_median": (
                median / elided_median if elided_median and median is not None else None
            ),
            "ratio_vs_a0_median": (
                median / args.a0_median if args.a0_median and median is not None else None
            ),
            "n_sequences": bpe_stats["records"],
        }
        rows.append(row)
        print(
            f"m={n_eff}: median={median} mean={bpe_stats['mean_tokens']:.3f} "
            f"p90={bpe_stats['p90_tokens']} "
            f"vs_elided={row['ratio_vs_elided_median']:.3f} "
            f"vs_a0={row['ratio_vs_a0_median']:.3f}",
            flush=True,
        )

    # Gate: 512 merges must be meaningfully below elided baseline.
    gate_row = next((r for r in rows if r["requested_merges"] == 512), None)
    gate = {
        "merge_budget": 512,
        "elided_baseline_median": elided_median,
        "a0_baseline_median": args.a0_median,
        "measured_median_at_512": None,
        "relative_reduction_vs_elided": None,
        "meaningfully_below_elided": False,
        "verdict": "no_512_row",
        "criterion": (
            "Pass if median BPE tokens at 512 merges is at least 10% below the "
            "measured elided baseline median (strictly < 0.90 * elided_median)."
        ),
    }
    if gate_row is not None and gate_row["bpe_length_stats"]["median_tokens"] is not None:
        m512 = float(gate_row["bpe_length_stats"]["median_tokens"])
        gate["measured_median_at_512"] = m512
        if elided_median and elided_median > 0:
            rel = (elided_median - m512) / elided_median
            gate["relative_reduction_vs_elided"] = rel
            ok = m512 < 0.90 * elided_median
            gate["meaningfully_below_elided"] = ok
            gate["verdict"] = "pass" if ok else "fail"
        else:
            gate["verdict"] = "no_elided_baseline"

    report = {
        "artifact_kind": "sql_elided_bpe_compression_report_v1",
        "claim_bearing": False,
        "note": (
            "Phase-1 measurement only. BPE IDs are a downstream artifact over the "
            "elided SQL stream contract; not a token-layout pin change."
        ),
        "encoded_input": str(args.encoded),
        "base_vocab_size": args.base_vocab,
        "max_merges_trained": trained_merges,
        "full_merge_table": str(max_merge_path),
        "elided_length_stats": elided_stats,
        "elided_baseline_median": elided_median,
        "a0_surface_sql_bpe_median": args.a0_median,
        "per_merge_budget": rows,
        "embedding_cost_note": (
            "Each merge introduces one new token id that needs an embedding row "
            "learned from scratch. On a ~5,715-example fine-tune this fixed cost "
            "can outweigh sequence-length gains if the model is data-limited."
        ),
        "gate": gate,
    }
    report_path.write_text(json.dumps(report, indent=2) + "\n")
    print(f"Wrote {report_path}", flush=True)
    print(f"GATE verdict: {gate['verdict']}", flush=True)
    return 0 if gate["verdict"] in ("pass", "no_512_row") else 0  # always 0; report carries verdict


if __name__ == "__main__":
    raise SystemExit(main())
