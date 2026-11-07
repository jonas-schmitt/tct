# TCT Real-World Schema Benchmarks

This directory contains the maintained baseline and regression helper for TCT's
real-world schema compatibility checks. Generated benchmark outputs are written
under `target/tct-benchmarks/` by default so opt-in benchmark runs do not modify
tracked files.

## Files

- `baseline.json` - Baseline metrics for regression detection (created manually)
- `regression_check.sh` - Local regression check against the baseline

## Running Benchmarks

### Basic Schema Tests (Fast)
```bash
# Run representative schemas
cargo test --test schema_compilation_test test_basic_real_world_schemas
```

### Extended Schema Tests (Comprehensive)
```bash
# Run full corpus validation
cargo test --features extended-benchmarks --test schema_compilation_test test_extended_real_world_schemas -- --ignored --exact --nocapture
```

The latest generated report is written to
`target/tct-benchmarks/latest_extended_results.json`. Override the directory
with `TCT_BENCHMARK_OUTPUT_DIR=/path/to/output` when needed.

### Category-Specific Tests
```bash
# Test configuration schemas only
cargo test --test schema_compilation_test test_configuration_schemas

# Test API schemas only
cargo test --test schema_compilation_test test_api_schemas
```

## Current Baseline

The curated baseline is `baseline.json`. Do not copy volatile generated counts,
timestamps, or success percentages into maintained docs; preserve a new baseline
only when the expected compatibility contract intentionally changes.

## Regression Detection

The `regression_check.sh` script compares current results with baseline:
- Fails if success rate drops by >5%
- Fails if average compilation time increases by >50%
- Generates detailed diff report

## Adding New Schemas

1. Add schema files to `schemas/{category}/`
2. Run extended tests to verify
3. Update baseline only if the changed schema set is an intentional compatibility
   contract change.

## Performance Optimization Tracking

The benchmark system tracks optimization effectiveness:
- Schemas with optimizations applied
- Types of optimizations used (enum indices, bounded integers, etc.)
- Compression ratios and performance improvements
