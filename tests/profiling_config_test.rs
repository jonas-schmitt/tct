use serde_json::json;
use tct::{ProfilingConfig, ProfilingLevel, TctCompiler};

#[test]
fn test_profiling_config_api() -> Result<(), Box<dyn std::error::Error>> {
    let schema = json!({"type": "string"});

    // Test default (no profiling)
    let compiler = TctCompiler::new(schema.clone(), "TestType")?;
    let grammar = compiler.compile()?;
    assert!(grammar.profiling_code.is_none());

    // Test basic profiling configuration
    let compiler =
        TctCompiler::new(schema.clone(), "TestType")?.with_profiling(ProfilingConfig::basic());
    let grammar = compiler.compile()?;
    // Now we actually generate profiling code
    assert!(grammar.profiling_code.is_some());
    let profiling = grammar.profiling_code.as_ref().unwrap();
    assert!(!profiling.module_code.is_empty());
    assert!(profiling.benchmark_file.is_none()); // Basic doesn't generate Criterion

    // Test detailed profiling configuration
    let compiler =
        TctCompiler::new(schema.clone(), "TestType")?.with_profiling(ProfilingConfig::detailed());
    let grammar = compiler.compile()?;
    assert!(grammar.profiling_code.is_some());
    let profiling = grammar.profiling_code.as_ref().unwrap();
    assert!(!profiling.module_code.is_empty());
    assert!(profiling.benchmark_file.is_none()); // Detailed doesn't generate Criterion

    // Test benchmarking configuration
    let compiler = TctCompiler::new(schema, "TestType")?.with_profiling(
        ProfilingConfig::benchmarking().with_criterion_sample("example", json!("example")),
    );
    let grammar = compiler.compile()?;
    assert!(grammar.profiling_code.is_some());
    let profiling = grammar.profiling_code.as_ref().unwrap();
    assert!(!profiling.module_code.is_empty());
    assert!(profiling.benchmark_file.is_some()); // Benchmarking generates Criterion

    Ok(())
}

#[test]
fn test_profiling_config_builder() {
    // Test basic config
    let config = ProfilingConfig::basic();
    assert_eq!(config.level, ProfilingLevel::Basic);
    assert_eq!(config.feature_gate, "profiling");
    assert!(!config.generate_criterion);
    assert!(!config.track_allocations);
    assert!(config.criterion_samples.is_empty());

    // Test detailed config
    let config = ProfilingConfig::detailed();
    assert_eq!(config.level, ProfilingLevel::Detailed);
    assert!(config.track_allocations);

    // Test benchmarking config
    let config = ProfilingConfig::benchmarking();
    assert!(config.generate_criterion);
    assert!(config.criterion_samples.is_empty());

    // Test builder pattern
    let config = ProfilingConfig::basic()
        .with_feature_gate("custom-profiling")
        .with_criterion(true)
        .with_criterion_sample("example", json!("example"))
        .with_allocation_tracking(true);

    assert_eq!(config.feature_gate, "custom-profiling");
    assert!(config.generate_criterion);
    assert!(config.track_allocations);
    assert_eq!(config.criterion_samples.len(), 1);
}

#[test]
fn test_profiling_level_checks() {
    let basic = ProfilingConfig::basic();
    assert!(basic.is_basic());
    assert!(!basic.is_detailed());
    assert!(!basic.is_disabled());

    let detailed = ProfilingConfig::detailed();
    assert!(!detailed.is_basic());
    assert!(detailed.is_detailed());
    assert!(!detailed.is_disabled());

    let disabled = ProfilingConfig::default();
    assert!(!disabled.is_basic());
    assert!(!disabled.is_detailed());
    assert!(disabled.is_disabled());
}
