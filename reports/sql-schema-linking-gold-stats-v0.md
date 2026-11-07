# SQL schema-linking gold stats (T11)

- artifact: `tct_sql_schema_linking_gold_stats_v0`
- claim_boundary: `compiler_sql_schema_linking_only`
- no_training_claims: true
- db_id: `tct_fixture_join`
- layout_id: `sql-schema-linking-token-layout-v1`
- queries: 13 (parse_ok=13, parse_fail=0, round_trip_ok=13)
- schema_ref sites: 56 menu tokens vs 502 baseline tokens (ratio ≈ 8.96×)

Parse failures are listed explicitly (not filtered out).

| id | parse | round_trip | menu sites | baseline tokens | menu tokens |
| --- | --- | --- | ---: | ---: | ---: |
| select_one_table | ok | ok | 3 | 23 | 3 |
| join_two_tables | ok | ok | 6 | 59 | 6 |
| star_structural | ok | ok | 1 | 10 | 1 |
| three_table_join | ok | ok | 10 | 100 | 10 |
| where_eq_string | ok | ok | 3 | 23 | 3 |
| where_cmp_and_or | ok | ok | 6 | 48 | 6 |
| join_with_where | ok | ok | 7 | 67 | 7 |
| group_order_limit | ok | ok | 4 | 29 | 4 |
| where_group_having_order_limit | ok | ok | 6 | 48 | 6 |
| count_star | ok | ok | 1 | 11 | 1 |
| sum_aggregate | ok | ok | 2 | 16 | 2 |
| select_distinct | ok | ok | 2 | 17 | 2 |
| scalar_subquery | ok | ok | 5 | 51 | 5 |
