# SQL schema-linking frontend handoff (T12)

- artifact: `tct_sql_schema_linking_frontend_handoff_v0`
- claim_boundary: `compiler_sql_schema_linking_only`
- no_training_claims: true
- not_a_production_route: true
- tokenizer_module_id: `sql_schema_linking_sqlite_subset_v0`
- layout_id: `sql-schema-linking-token-layout-v1` (pin: `tests/fixtures/syntax_structural_model/sql-token-layout-pin-v0.json`)
- vocab_size / PAD: 321 / 320
- menu_band: role=`menu_index_payload` width=256 max_digits=2 capacity=65536
- schema pin db_id: `tct_fixture_join` tables_sha=4e009388a1cf347645b1370d716dedec1d6d9af9ff8b54adc8bef13b425c64c0 gold_sha=a4be7da20cad3ea1bc248153d3b531cc25db8111a3022e8dced847e88bd8a871
- gold_stats_summary: `reports/sql-schema-linking-gold-stats-v0.json`
- example streams: 13 gold ids with stream_sha256
- decode → executable SQL: decode tokens → AST → render_select → canonical SQL
- SharedAlphabet: supported (menus on u8 band + multi-digit capacity); PerGrammar remains pin default
- note: tokens per schema reference are scope-dependent (not always 1 digit)

This is compiler packaging for the downstream training repository. It does not
train, evaluate, or support a paper claim.
