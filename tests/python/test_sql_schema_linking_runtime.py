"""Smoke tests for the SQL schema-linking stage-1 runtime (PyO3).

Skipped when the `tct` extension is not importable (ordinary pytest without
a maturin install) or was built without the `experimental` feature. Focused
Rust coverage lives in `src/sql_schema_linking/runtime/`.

Module-level skip uses `unittest.SkipTest` so both
`python3 -m pytest tests/python` and
`python3 -m unittest discover tests/python` treat absence of the extension as
a skip (not a collection error from pytest's module-level skip exception).
"""

from __future__ import annotations

import hashlib
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


FIXTURE_DIR = (
    Path(__file__).resolve().parents[1] / "fixtures" / "sql_schema_linking"
)
TABLES_JSON = FIXTURE_DIR / "tables.json"
FIXTURE_DB_ID = "tct_fixture_join"


def _fixture_runtime():
    if hasattr(tct.SqlSchemaLinkingRuntime, "fixture"):
        return tct.SqlSchemaLinkingRuntime.fixture()
    raw = TABLES_JSON.read_text(encoding="utf-8")
    sha = hashlib.sha256(raw.encode("utf-8")).hexdigest()
    return tct.SqlSchemaLinkingRuntime(raw, sha, FIXTURE_DB_ID)


class TestSqlSchemaLinkingRuntime(unittest.TestCase):
    def test_encode_decode_round_trip(self):
        rt = _fixture_runtime()
        sql = "SELECT name, city FROM customers"
        tokens = rt.encode(sql)
        self.assertTrue(all(t < rt.pad_token for t in tokens))
        self.assertEqual(rt.decode(tokens), 'SELECT "name", "city" FROM "customers"')
        self.assertEqual(rt.encode(rt.decode(tokens)), tokens)

    def test_incremental_tct_decode(self):
        rt = _fixture_runtime()
        tokens = rt.encode("SELECT * FROM products")
        st = rt.begin_tct_decode()
        for token in tokens:
            self.assertIn(token, st.allowed_token_ids())
            st.apply(token)
        self.assertTrue(st.is_complete())
        self.assertEqual(st.render_sql(), 'SELECT * FROM "products"')
        self.assertEqual(st.consumed_tokens(), len(tokens))

    def test_active_menu_names_table_site(self):
        rt = _fixture_runtime()
        tokens = rt.encode_elided("SELECT name FROM customers")
        st = rt.begin_elided_tct_decode()
        self.assertTrue(hasattr(st, "active_menu"))
        kinds = []
        menus_by_kind = {}
        for token in tokens:
            menu = st.active_menu()
            kinds.append(menu["kind"])
            if menu["kind"] != "none" and menu["kind"] not in menus_by_kind:
                menus_by_kind[menu["kind"]] = menu
            st.apply(token)
        self.assertIn("table", kinds)
        table_menu = menus_by_kind["table"]
        self.assertIn("customers", [r["name"] for r in table_menu["referents"]])
        # Width-1 sites resolve names at digit 0; wider sites defer (Rust
        # coverage pins the wide-radix behavior).
        self.assertEqual(table_menu["width"], 1)
        self.assertTrue(table_menu["resolved"])
        # The default flat addressing exposes the column menu with names too.
        self.assertIn("column_flat", kinds)
        column_menu = menus_by_kind["column_flat"]
        self.assertTrue(column_menu["resolved"])
        self.assertIn("name", [r["name"] for r in column_menu["referents"]])

    def test_capability_report_and_text_constraint_fail_closed(self):
        rt = _fixture_runtime()
        caps = rt.capability_report()
        self.assertEqual(caps["sql_text_contract"], "sqlite_quoted_identifiers_no_nul_v1")
        self.assertTrue(caps["a5_encode_decode"])
        self.assertTrue(caps["a5_incremental_tct_decode"])
        self.assertFalse(caps["a1_text_structural_constraint"])
        self.assertFalse(caps["a2_text_dynamic_scope_constraint"])
        blocker = caps["text_constraint_blocker"]
        self.assertEqual(blocker["code"], tct.SQL_TEXT_CONSTRAINT_BLOCKER_CODE)
        self.assertTrue(blocker["reusable"])

        pieces = {1: "SELECT", 2: " "}
        tok_hash = "a" * 64
        with self.assertRaises(Exception) as a1:
            rt.begin_text_constraint(pieces, tok_hash, dynamic_scope=False)
        self.assertIn(tct.SQL_TEXT_CONSTRAINT_BLOCKER_CODE, str(a1.exception))
        with self.assertRaises(Exception) as a2:
            rt.begin_text_constraint(pieces, tok_hash, dynamic_scope=True)
        self.assertIn(tct.SQL_TEXT_CONSTRAINT_BLOCKER_CODE, str(a2.exception))

    def test_wrong_catalog_sha_rejected(self):
        raw = TABLES_JSON.read_text(encoding="utf-8")
        bad = "0" * 64
        with self.assertRaises(Exception) as exc:
            tct.SqlSchemaLinkingRuntime(raw, bad, FIXTURE_DB_ID)
        self.assertIn("sha256", str(exc.exception).lower())


if __name__ == "__main__":
    unittest.main()
