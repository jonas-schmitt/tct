"""Qualified SQL menu identities through the real opt-in PyO3 API.

Requires an extension built from the source under test. Absence skips this
module; an installed extension missing the method fails and must not be
counted as validation of the new API. No model or host corpus is loaded.
"""

from __future__ import annotations

import hashlib
import json
import tempfile
import unittest
from pathlib import Path

try:
    import tct
except ImportError as exc:  # pragma: no cover - no native extension installed
    raise unittest.SkipTest("tct extension not importable; qualified API untested") from exc

if not hasattr(tct, "SqlSchemaLinkingRuntime"):  # pragma: no cover - core-only build
    raise unittest.SkipTest(
        "tct extension built without the experimental feature (no SQL runtime bindings)"
    )


CATALOG_PATH = (
    Path(__file__).resolve().parents[1]
    / "fixtures/sql_schema_linking/qualified_menu/tables.json"
)
MENU_KEYS = {"kind", "referents", "digits_seen", "width", "resolved", "addressing"}
BINDING_KEYS = {
    "token_id", "ordinal", "table_ordinal", "table_name", "column_ordinal", "column_name"
}
# sql-schema-linking-token-layout-v1: select the payload band only after the
# native cursor identifies a resolved menu site; a band alone has no role.
PAYLOAD_START, PAYLOAD_END = 64, 320


def fixture_runtime(db_id="qualified_menu", addressing="flat"):
    raw = CATALOG_PATH.read_text(encoding="utf-8")
    return tct.SqlSchemaLinkingRuntime(
        raw,
        hashlib.sha256(raw.encode()).hexdigest(),
        db_id,
        column_addressing=addressing,
    )


class TestSqlActiveMenuBindings(unittest.TestCase):
    def assert_menu_contract(self, state):
        historical = state.active_menu()
        self.assertEqual(set(historical), MENU_KEYS)
        self.assertTrue(all(set(r) == {"ordinal", "name"} for r in historical["referents"]))
        qualified = state.active_menu_bindings()
        self.assertEqual(set(qualified), MENU_KEYS | {"bindings"})
        self.assertEqual({k: qualified[k] for k in MENU_KEYS}, historical)
        self.assertEqual(len(qualified["bindings"]), len(historical["referents"]))
        if qualified["resolved"]:
            binding_ids = {binding["token_id"] for binding in qualified["bindings"]}
            payload_mask = {
                token for token in state.allowed_token_ids() if PAYLOAD_START <= token < PAYLOAD_END
            }
            self.assertEqual(binding_ids, payload_mask, "every admissible candidate needs a binding")
            self.assertEqual(len(binding_ids), len(qualified["bindings"]), "one binding per token")
        else:
            self.assertEqual(qualified["bindings"], [])
        for binding, referent in zip(qualified["bindings"], historical["referents"]):
            self.assertEqual(set(binding), BINDING_KEYS)
            self.assertEqual(binding["ordinal"], referent["ordinal"])
            self.assertIn(binding["token_id"], state.allowed_token_ids())
            if binding["column_ordinal"] is None:
                self.assertIsNone(binding["column_name"])
                self.assertEqual(binding["table_name"], referent["name"])
            else:
                self.assertEqual(binding["column_name"], referent["name"])
        return qualified

    def test_duplicate_columns_follow_catalog_identity_on_all_base_walkers(self):
        sql = "SELECT Alpha.Name, Beta.Name FROM Alpha JOIN Beta ON Alpha.Id = Beta.Id"
        for db_id, alpha_ordinal in [("qualified_menu", 0), ("qualified_menu_permuted", 2)]:
            for addressing in ["flat", "factored"]:
                rt = fixture_runtime(db_id, addressing)
                for elided in [False, True]:
                    with self.subTest(db_id=db_id, addressing=addressing, elided=elided):
                        state = rt.begin_elided_tct_decode() if elided else rt.begin_tct_decode()
                        tokens = rt.encode_elided(sql) if elided else rt.encode(sql)
                        chosen = []
                        for token in tokens:
                            qualified = self.assert_menu_contract(state)
                            for row in qualified["bindings"]:
                                if row["column_ordinal"] is not None:
                                    self.assertIn(row["table_name"], ["Alpha", "Beta"])
                                    self.assertEqual(
                                        row["table_ordinal"],
                                        alpha_ordinal if row["table_name"] == "Alpha" else 1,
                                    )
                                    if row["token_id"] == token:
                                        chosen.append((row["table_name"], row["column_name"]))
                            state.apply(token)
                        self.assertEqual(chosen, [
                            ("Alpha", "Id"), ("Beta", "Id"), ("Alpha", "Name"), ("Beta", "Name")
                        ])
                        self.assertTrue(state.is_complete())
                        self.assertEqual(self.assert_menu_contract(state)["bindings"], [])

    def test_forced_factored_parent_and_clone_have_independent_future_choices(self):
        rt = fixture_runtime("qualified_menu_permuted", "factored")
        tokens = rt.encode_elided("SELECT Name FROM Alpha")
        state = rt.begin_elided_tct_decode()
        for token in tokens:
            menu = self.assert_menu_contract(state)
            self.assertNotEqual(menu["kind"], "column_table")
            if menu["kind"] == "column":
                self.assertEqual(menu["digits_seen"], 1)
                self.assertEqual([b["table_ordinal"] for b in menu["bindings"]], [2, 2])
                self.assertEqual([b["table_name"] for b in menu["bindings"]], ["Alpha", "Alpha"])
                probe = state.clone()
                alternate = next(b["token_id"] for b in menu["bindings"] if b["token_id"] != token)
                probe.apply(alternate)
                self.assertEqual(state.active_menu_bindings(), menu)
                self.assertNotEqual(probe.expanded_tokens(), state.expanded_tokens())
                return
            state.apply(token)
        self.fail("never reached the column digit after implicit table elision")

    def test_bpe_merge_crosses_parent_choice_and_names_next_column(self):
        rt = fixture_runtime("qualified_menu_permuted", "factored")
        sql = "SELECT Beta.Name FROM Alpha JOIN Beta ON Alpha.Id = Beta.Id"
        tokens = rt.encode_elided(sql)
        probe = rt.begin_elided_tct_decode()
        parent_position = None
        for position, token in enumerate(tokens):
            if probe.active_menu()["kind"] == "column_table":
                parent_position = position
                break
            probe.apply(token)
        self.assertIsNotNone(parent_position, "the join must expose a free parent choice")
        self.assertGreater(parent_position, 0)
        # The fixed query's first column is ON-lhs Alpha.Id, and the permuted
        # catalog puts Alpha at ordinal 2. Merge its preceding structural
        # choice with the parent digit; the next unconsumed choice is Id/Name.
        pair = tokens[parent_position - 1:parent_position + 1]
        self.assertLess(pair[0], PAYLOAD_START)
        self.assertEqual(pair[1], PAYLOAD_START + 2)
        merge_id = rt.vocab_size
        table = {
            "schema": "qualified-menu-parent-crossing-bpe",
            "base_vocab_size": rt.vocab_size,
            "merges": [{"pair": pair, "new_token": merge_id, "frequency": 1}],
            "metadata": {
                "trained_on_instances": 1,
                "training_date": "qualified-menu-fixture",
                "max_merges": 1,
                "dict_size": None,
            },
        }
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "parent-crossing.json"
            path.write_text(json.dumps(table), encoding="utf-8")
            bpe = rt.begin_elided_bpe_tct_decode(str(path), 1)
            for token in tokens[:parent_position - 1]:
                bpe.apply(token)
            self.assertIn(merge_id, bpe.allowed_token_ids())
            self.assertEqual(bpe.apply(merge_id), pair, "the nonempty merge must actually execute")
            self.assertEqual(bpe.consumed_bpe_tokens(), parent_position)
            menu = self.assert_menu_contract(bpe)
            self.assertEqual(menu["kind"], "column")
            self.assertEqual((menu["digits_seen"], menu["width"], menu["resolved"]), (1, 2, True))
            self.assertEqual(menu["bindings"], [
                {"token_id": PAYLOAD_START, "ordinal": 0, "table_ordinal": 2,
                 "table_name": "Alpha", "column_ordinal": 0, "column_name": "Id"},
                {"token_id": PAYLOAD_START + 1, "ordinal": 1, "table_ordinal": 2,
                 "table_name": "Alpha", "column_ordinal": 1, "column_name": "Name"},
            ])
            for token in tokens[parent_position + 1:]:
                bpe.apply(token)
            self.assertTrue(bpe.is_complete())
            self.assertEqual(bpe.active_menu_bindings()["bindings"], [])

    def test_wide_flat_final_ordinal_and_factored_local_column(self):
        columns = [[-1, "*"]] + [
            [table, f"c{column}"] for table in range(2) for column in range(150)
        ]
        raw = json.dumps([{
            "db_id": "wide_qualified",
            "table_names": ["alpha", "beta"],
            "table_names_original": ["Alpha", "Beta"],
            "column_names": columns,
            "column_names_original": columns,
            "column_types": ["number"] * 301,
            "primary_keys": [],
            "foreign_keys": [],
        }])
        sql = "SELECT Beta.c149 FROM Alpha JOIN Beta ON Alpha.c0 = Beta.c0"
        for addressing in ["flat", "factored"]:
            rt = tct.SqlSchemaLinkingRuntime(
                raw, hashlib.sha256(raw.encode()).hexdigest(), "wide_qualified",
                column_addressing=addressing,
            )
            state = rt.begin_elided_tct_decode()
            selected = []
            saw_unresolved = False
            for token in rt.encode_elided(sql):
                menu = self.assert_menu_contract(state)
                if menu["kind"] == "column_flat" and menu["digits_seen"] == 0:
                    self.assertFalse(menu["resolved"])
                    self.assertEqual(menu["width"], 2)
                    saw_unresolved = True
                for binding in menu["bindings"]:
                    if (binding["token_id"] == token
                            and binding["table_name"] == "Beta"
                            and binding["column_name"] == "c149"):
                        selected.append(binding)
                state.apply(token)
            self.assertTrue(state.is_complete())
            self.assertEqual(len(selected), 1)
            self.assertEqual(selected[0]["table_ordinal"], 1)
            self.assertEqual(selected[0]["column_ordinal"], 149)
            self.assertEqual(selected[0]["ordinal"], 299 if addressing == "flat" else 149)
            self.assertEqual(saw_unresolved, addressing == "flat")


if __name__ == "__main__":
    unittest.main()
