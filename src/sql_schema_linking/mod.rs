//! SQL schema-linking frontend (claim-bounded, not a production route).
//!
//! Implements `docs/sql-schema-linking-frontend-contract.md`.
//! Catalog, parser, GrammarIR, menu binding, and generic-codec reuse land
//! incrementally via the T1–T12 tranche ladder. This is not JSON Schema TCT,
//! not a tree-sitter pin, and not a paper claim.

pub mod ast;
pub mod catalog;
pub mod codec;
pub mod elided_bpe_mask;
pub mod grammar;
pub mod layout;
pub mod menu_binding;
pub mod menu_trace;
pub mod parser;
pub mod render;
pub mod report;
pub mod runtime;
pub mod spider_coverage;
pub mod tree;

pub use ast::{
    AggFunc, AggregateArg, BoolOp, CmpOp, ColumnRef, Expression, FromClause, GroupByClause, InRhs,
    JoinClause, JoinKind, OrderByClause, OrderDirection, OrderItem, OrderKey, SelectItem,
    SelectItems, SelectList, SelectStatement, TableRef,
};
pub use catalog::{
    FixturePin, FixturePinPaths, ForeignKeyRef, SQL_SCHEMA_LINKING_CLAIM_BOUNDARY,
    SQL_SCHEMA_LINKING_FIXTURE_DB_ID, SQL_SCHEMA_LINKING_FIXTURE_PIN_ARTIFACT,
    SQL_SCHEMA_LINKING_WIDE_FIXTURE_DB_ID, SqlCatalog, SqlCatalogError, SqlColumn,
    SqlFixtureBundle, SqlTable, fixture_dir, load_fixture_bundle, load_fixture_catalog,
    load_wide_menu_capacity_catalog, normalize_sha256_hex, resolve_repo_relative_path,
    validate_relative_fixture_path, verify_gold_queries_bytes,
};
// Unpinned multi-db load lives on SqlCatalog::{load_unpinned_tables_json,
// parse_tables_json_unpinned, list_db_ids} for host Spider / reconstructed
// catalogs (not claim-bearing official pins).
pub use codec::{
    SQL_CODEC_CLAIM_BOUNDARY, SQL_CODEC_SUBSET_ID, SqlCodecError, SqlRoundTrip,
    SqlSchemaLinkingCodec, count_menu_tokens, count_menu_tokens_by_role_replay,
};
pub use elided_bpe_mask::{
    ARTIFACT_KIND as SQL_ELIDED_BPE_MASK_ARTIFACT_KIND,
    CLAIM_BOUNDARY as SQL_ELIDED_BPE_MASK_CLAIM_BOUNDARY, STATUS as SQL_ELIDED_BPE_MASK_STATUS,
    SqlBpeTokenDecision, SqlElidedBpeMaskReport, SqlElidedBpePrefixMask,
    TOOL as SQL_ELIDED_BPE_MASK_TOOL, VERSION as SQL_ELIDED_BPE_MASK_VERSION, bpe_codec_for_merges,
    bpe_prefix_masks_for_gold_path, bpe_token_is_admissible, build_sql_elided_bpe_mask_report,
    expand_bpe_token_to_elided_base, raw_elided_exact_mask_cases_for_stream,
    round_trip_sql_via_elided_bpe, train_elided_bpe_table, try_apply_bpe_token,
};
pub use grammar::{
    COLUMN_MENU_OWNER_FIELD, COLUMN_MENU_OWNER_RULE, COLUMN_REF_RULE, FROM_CLAUSE_RULE,
    SELECT_STATEMENT_FIELD_ORDER, SELECT_STATEMENT_RULE, SQL_GRAMMAR_ID, SQL_GRAMMAR_SOURCE_KIND,
    SQL_GRAMMAR_SOURCE_PATH, STAR_RULE, TABLE_MENU_OWNER_FIELD, TABLE_MENU_OWNER_RULE,
    TABLE_REF_RULE, lower_sql_schema_linking_grammar, select_statement_field_index,
    select_statement_field_names, sql_schema_linking_grammar_ir,
};
pub use layout::{
    SQL_MENU_BAND_WIDTH, SQL_MENU_INDEX_PAYLOAD_ROLE, SQL_SYNTAX_LAYOUT_PAD_TOKEN,
    SQL_SYNTAX_LAYOUT_VOCAB_SIZE, SQL_SYNTAX_TOKEN_LAYOUT_ID, SQL_SYNTAX_TOKEN_LAYOUT_VERSION,
    SQL_TOKEN_LAYOUT_PIN_FIXTURE_PATH, menu_band_disjoint_from_u8_integer, menu_band_range,
    payload_roles_share_pool, sql_syntax_token_layout, u8_integer_band_range, utf8_band_range,
};
pub use menu_binding::{
    MenuBindingError, SQL_MAX_MENU_DIGITS, SqlColumnAddressing, SqlJoinOnPolicy,
    column_menu_cardinality_for_table, menu_bindings_for_single_table_scope,
    menu_bindings_from_catalog, menu_bindings_from_catalog_with_join_policy,
    menu_bindings_from_catalog_with_policies, sql_menu_ordinal_capacity,
    table_menu_bindings_from_catalog,
};
pub use menu_trace::{MenuTraceError, MenuTraceSite, menu_trace_from_select};
pub use parser::{SqlParseError, parse_m0_select, parse_select};
pub use render::{
    RenderError, SQL_SUBSET_KEYWORDS, SQL_TEXT_CONTRACT, is_sql_subset_keyword, render_select,
};
pub use report::{
    GoldQueryStat, SQL_GOLD_STATS_ARTIFACT_KIND, SQL_GOLD_STATS_CLAIM_BOUNDARY,
    SQL_GOLD_STATS_VERSION, SQL_HANDOFF_ARTIFACT_KIND, SQL_HANDOFF_VERSION, SqlFrontendHandoff,
    SqlGoldStatsReport, fixture_gold_stats_report, fixture_sql_frontend_handoff,
    gold_stats_markdown, gold_stats_report_from_bundle, handoff_markdown,
};
pub use runtime::{
    ELIDED_FORCED_RUN_LIMIT, SQL_RUNTIME_CLAIM_BOUNDARY, SQL_RUNTIME_SURFACE_ID,
    SQL_TEXT_CONSTRAINT_BLOCKER_CODE, SqlActiveMenu, SqlActiveMenuReferent,
    SqlElidedTctDecodeState, SqlRuntimeCapabilityReport, SqlRuntimeError, SqlSchemaLinkingRuntime,
    SqlTctActiveSite, SqlTctDecodeState, SqlTextConstraintBlocker, SqlTextConstraintState,
    SqlTextScopeMetadata, TextTokenPieceMap, singleton_allowed_token_id,
};
pub use spider_coverage::{
    KeywordColumnCensus, KeywordColumnCensusRow, SQL_SPIDER_PARSE_COVERAGE_ARTIFACT,
    SQL_SPIDER_PARSE_COVERAGE_CLAIM_BOUNDARY, SpiderEncodedExample, SpiderParseCoverageReport,
    UnloadableCensusDatabase, census_keyword_named_columns, encode_spider_examples,
    measure_spider_parse_coverage,
};
pub use tree::{TreeError, select_from_decoded_node, select_to_decoded_node};
