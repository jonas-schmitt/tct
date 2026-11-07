"""Gold-stream replay for the SQL elided-BPE admissible mask PyO3 binding.

Uses the corpus-validated merge table and train JSONL when absolute artifact
paths are present. Falls back to a self-contained fixture path (identity BPE
over the in-repo join fixture) when those paths are missing so the test never
silently skips the contract. Full non-empty merge training lives in the Rust
binding test (`sql_elided_bpe_tct_decode_state_fixture_gold_path_and_pad`) via
`train_elided_bpe_table`.

Requires an importable `tct` extension built with the `experimental` feature
(`--features python,experimental` / maturin wheel with `-F experimental`).
"""

from __future__ import annotations

import hashlib
import json
import os
import tempfile
import unittest
from pathlib import Path

try:
    import tct
except ImportError as exc:  # pragma: no cover - exercised without extension
    raise unittest.SkipTest(
        "tct extension not importable (build with --features python,experimental / maturin)"
    ) from exc

if not hasattr(tct, "SqlSchemaLinkingRuntime"):  # pragma: no cover - core-only build
    raise unittest.SkipTest(
        "tct extension built without the experimental feature (no SQL runtime bindings)"
    )


# Optional real-data paths; override with TCT_SQL_ELIDED_BPE_DIR and
# TCT_SPIDER_TABLES. The tests below skip when the files are absent.
_SQL_DATA_DIR = Path(os.environ.get("TCT_SQL_ELIDED_BPE_DIR", "data/sql-elided-bpe"))
REAL_CORPUS = _SQL_DATA_DIR / "train-elided-bpe-m191.jsonl"
REAL_MERGES = _SQL_DATA_DIR / "sql-elided-bpe-m191.json"
REAL_CATALOG = Path(os.environ.get("TCT_SPIDER_TABLES", "data/spider-official/tables.json"))
REAL_SOURCE_CORPUS = _SQL_DATA_DIR / "stage1_corpus_indexed.json"

FIXTURE_DIR = Path(__file__).resolve().parents[1] / "fixtures" / "sql_schema_linking"
SAMPLE_LIMIT = 40
# Stride through the train JSONL so the sample is not just the first db.
SAMPLE_STRIDE = 150


def _real_artifacts_available() -> bool:
    return REAL_CORPUS.is_file() and REAL_MERGES.is_file() and REAL_CATALOG.is_file()


def _load_rendered_sql_by_id(limit_ids: set[str]) -> dict[str, str]:
    """Optional gold SQL map from the indexed stage-1 corpus (when present)."""
    if not REAL_SOURCE_CORPUS.is_file() or not limit_ids:
        return {}
    with REAL_SOURCE_CORPUS.open(encoding="utf-8") as fh:
        data = json.load(fh)
    out: dict[str, str] = {}
    for ex in data.get("examples", []):
        eid = ex.get("example_id")
        if eid in limit_ids:
            sql = ex.get("rendered_sql") or ex.get("gold_sql")
            if sql:
                out[eid] = sql
        if len(out) >= len(limit_ids):
            break
    return out


def _sample_rows(path: Path, limit: int, stride: int) -> list[dict]:
    rows: list[dict] = []
    with path.open(encoding="utf-8") as fh:
        for i, line in enumerate(fh):
            if i % stride != 0:
                continue
            rows.append(json.loads(line))
            if len(rows) >= limit:
                break
    return rows


def _runtime_for_db(catalog_json: str, catalog_sha: str, db_id: str, cache: dict):
    if db_id not in cache:
        cache[db_id] = tct.SqlSchemaLinkingRuntime(catalog_json, catalog_sha, db_id)
    return cache[db_id]


class TestSqlElidedBpeMaskBinding(unittest.TestCase):
    def test_elided_state_clone_independent(self):
        rt = tct.SqlSchemaLinkingRuntime.fixture()
        elided = rt.encode_elided("SELECT name FROM customers")
        self.assertGreaterEqual(len(elided), 2)
        base = rt.begin_elided_tct_decode()
        base.apply(elided[0])
        probe = base.clone()
        probe.apply(elided[1])
        self.assertEqual(base.consumed_elided_tokens(), 1)
        self.assertEqual(probe.consumed_elided_tokens(), 2)
        self.assertNotEqual(base.expanded_tokens(), probe.expanded_tokens())

    def test_gold_stream_replay_or_fixture_fallback(self):
        if _real_artifacts_available():
            self._run_real_corpus_gold_stream()
        else:
            self._run_fixture_identity_bpe_fallback()

    def test_bpe_active_menu_delegates_to_base_walker(self):
        # Self-contained on every host: the identity-BPE fixture path pins
        # active_menu parity between the BPE walker and the base elided walker.
        self._run_fixture_identity_bpe_fallback()

    def _run_real_corpus_gold_stream(self):
        catalog_json = REAL_CATALOG.read_text(encoding="utf-8")
        catalog_sha = hashlib.sha256(catalog_json.encode("utf-8")).hexdigest()
        rows = _sample_rows(REAL_CORPUS, SAMPLE_LIMIT, SAMPLE_STRIDE)
        self.assertGreaterEqual(len(rows), 1, "expected sample rows from gold JSONL")
        sql_by_id = _load_rendered_sql_by_id({r["example_id"] for r in rows})
        runtimes: dict = {}
        pad = 320
        for row in rows:
            self.assertEqual(row["pad_token"], pad)
            self.assertEqual(row["base_vocab_size"], 321)
            self.assertEqual(row["merge_count"], 191)
            self.assertEqual(row["total_vocab_size"], 512)

            rt = _runtime_for_db(catalog_json, catalog_sha, row["db_id"], runtimes)
            st = rt.begin_elided_bpe_tct_decode(str(REAL_MERGES), row["merge_count"])
            self.assertEqual(st.base_vocab_size, row["base_vocab_size"])
            self.assertEqual(st.pad_token, pad)
            self.assertEqual(st.total_vocab_size, row["total_vocab_size"])
            self.assertEqual(st.bpe_start, row["base_vocab_size"])

            accumulated: list[int] = []
            for bpe_id in row["bpe_token_ids"]:
                allowed = st.allowed_token_ids()
                self.assertIn(
                    bpe_id,
                    allowed,
                    f"{row['example_id']}: gold BPE {bpe_id} not in allowed",
                )
                self.assertNotIn(pad, allowed, f"{row['example_id']}: PAD in allowed")
                expansion = st.apply(bpe_id)
                accumulated.extend(expansion)

            self.assertTrue(st.is_complete(), row["example_id"])
            self.assertEqual(
                accumulated,
                row["elided_token_ids"],
                f"{row['example_id']}: base expansion mismatch",
            )
            self.assertEqual(st.consumed_bpe_tokens(), len(row["bpe_token_ids"]))
            rendered = st.render_sql()
            # Semantic gold: re-encode of rendered SQL must match elided stream.
            again = rt.encode_elided(rendered)
            self.assertEqual(
                again,
                row["elided_token_ids"],
                f"{row['example_id']}: render_sql re-encode fixpoint failed",
            )
            gold_sql = sql_by_id.get(row["example_id"])
            if gold_sql is not None:
                self.assertEqual(
                    rt.encode(rendered),
                    rt.encode(gold_sql),
                    f"{row['example_id']}: rendered and corpus SQL denote different token trees",
                )

    def _run_fixture_identity_bpe_fallback(self):
        """Self-contained path when host SQL artifacts are absent.

        Uses a zero-merge table (identity BPE) written to a temp file so the
        binding loader path is still exercised. Non-empty merge training is
        covered by the Rust test that calls `train_elided_bpe_table`.
        """
        rt = tct.SqlSchemaLinkingRuntime.fixture()
        sqls = [
            "SELECT name, city FROM customers",
            "SELECT * FROM products",
            "SELECT name FROM customers",
        ]
        # Getters, not methods: the fallback had bitrotted unnoticed because
        # hosts with real artifacts never executed it.
        base_vocab = rt.vocab_size
        pad = rt.pad_token
        table = {
            "schema": "sql-fixture-elided-identity-fallback",
            "base_vocab_size": base_vocab,
            "merges": [],
            "metadata": {
                "trained_on_instances": len(sqls),
                "training_date": "sql-elided-bpe-fixture-fallback",
                "max_merges": 0,
                "dict_size": None,
            },
        }
        with tempfile.TemporaryDirectory() as tmp:
            merge_path = Path(tmp) / "identity-merges.json"
            merge_path.write_text(json.dumps(table), encoding="utf-8")
            for sql in sqls:
                elided = rt.encode_elided(sql)
                st = rt.begin_elided_bpe_tct_decode(str(merge_path), 0)
                self.assertEqual(st.pad_token, pad)
                self.assertEqual(st.base_vocab_size, base_vocab)
                self.assertEqual(st.total_vocab_size, base_vocab)
                accumulated: list[int] = []
                # Identity BPE aligns 1:1 with the base elided walker, so the
                # BPE state's active_menu delegation must agree at every step.
                base_walker = rt.begin_elided_tct_decode()
                self.assertEqual(st.active_menu(), base_walker.active_menu())
                for tok in elided:
                    allowed = st.allowed_token_ids()
                    self.assertIn(tok, allowed)
                    self.assertNotIn(pad, allowed)
                    accumulated.extend(st.apply(tok))
                    base_walker.apply(tok)
                    self.assertEqual(st.active_menu(), base_walker.active_menu())
                self.assertTrue(st.is_complete())
                self.assertEqual(accumulated, elided)
                self.assertEqual(rt.encode(st.render_sql()), rt.encode(sql))


if __name__ == "__main__":
    unittest.main()
