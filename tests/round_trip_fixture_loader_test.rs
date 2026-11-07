//! Focused checks for shared round-trip fixture discovery.

#[allow(dead_code)]
mod round_trip_helpers;

use round_trip_helpers::{
    InstanceFixtureScope, instance_search_patterns_for_schema, load_instances_for_schema,
};
use std::collections::HashSet;

#[test]
fn helper_loads_schema_suffix_mixed_fixtures() {
    let instances = load_instances_for_schema("tsconfig", "popular");
    let instance_names = instances
        .iter()
        .map(|(name, _)| name.as_str())
        .collect::<HashSet<_>>();

    assert!(instance_names.contains("tsconfig/synthetic/public_strict"));
    assert!(instance_names.contains("tsconfig/apollographql_apolloserver_tsconfig"));
    assert!(instance_names.contains("tsconfig/nestjs_nest_tsconfig"));
    assert!(instance_names.contains("tsconfig/prisma_prisma_tsconfig"));
    assert!(instance_names.contains("tsconfig/trpc_trpc_tsconfig"));
}

#[test]
fn helper_large_scale_discovery_is_explicit() {
    let standard_patterns = instance_search_patterns_for_schema(
        "github-workflow",
        "popular",
        InstanceFixtureScope::Standard,
    );
    assert!(
        !standard_patterns
            .iter()
            .any(|(path, _)| path.ends_with("/large-scale")),
        "Standard helper discovery should not enumerate large-scale fixture directories"
    );

    let large_scale_patterns = instance_search_patterns_for_schema(
        "github-workflow",
        "popular",
        InstanceFixtureScope::IncludeLargeScale,
    );
    assert!(
        large_scale_patterns.iter().any(|(path, instance_type)| {
            path == "tests/data/instances/github-workflow/large-scale"
                && *instance_type == "large-scale"
        }),
        "Explicit large-scale discovery should include github-workflow large-scale fixtures"
    );
}
