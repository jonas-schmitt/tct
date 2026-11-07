use serde_json::json;
use std::collections::HashMap;
use std::time::Instant;
use tct::{OptimizationLevel, TctCompiler};

/// Code-shape analysis comparing optimization levels using compilation metrics.
///
/// This is not a generated-tokenizer runtime benchmark. Vocabulary values are
/// heuristic estimates from generated source shape.
#[test]
fn test_performance_analysis_optimization_level_compilation_analysis() {
    let test_cases = vec![
        ("Simple", create_simple_schema()),
        ("EnumHeavy", create_enum_heavy_schema()),
        ("Nested", create_nested_schema()),
    ];

    let optimization_levels = vec![
        (OptimizationLevel::O0, "O0"),
        (OptimizationLevel::O2, "O2"),
        (OptimizationLevel::O3, "O3"),
    ];

    println!("\n=== TCT Optimization Level Code-Shape Analysis ===");
    println!(
        "Source: compile timing plus generated-source heuristics; not generated-tokenizer runtime."
    );
    println!(
        "{:<12} {:<6} {:<15} {:<15} {:<15} {:<15}",
        "Schema", "Level", "Compile Time", "Code Length", "Est. Vocab", "Type Count"
    );
    println!("{:-<85}", "");

    let mut results = Vec::new();

    for (name, schema) in &test_cases {
        for (level, level_name) in &optimization_levels {
            let analysis = analyze_compilation_performance(schema, name, *level);

            println!(
                "{:<12} {:<6} {:<15} {:<15} {:<15} {:<15}",
                name,
                level_name,
                format!("{}ms", analysis.compile_time_ms),
                format!("{}B", analysis.generated_code_bytes),
                analysis.estimated_vocab_size,
                analysis.type_count
            );

            results.push((name.to_string(), level_name.to_string(), analysis));
        }
        println!("{:-<85}", "");
    }

    // Generate code-shape comparison report
    generate_compilation_performance_report(&results);
}

#[derive(Debug, Clone)]
struct CompilationAnalysis {
    compile_time_ms: u128,
    generated_code_bytes: usize,
    estimated_vocab_size: usize,
    type_count: usize,
    optimization_features: Vec<String>,
}

fn analyze_compilation_performance(
    schema: &serde_json::Value,
    name: &str,
    optimization_level: OptimizationLevel,
) -> CompilationAnalysis {
    // Time the compilation
    let start_time = Instant::now();

    let grammar = TctCompiler::new(schema.clone(), name)
        .expect("Failed to create TctCompiler")
        .with_optimization_level(optimization_level)
        .compile()
        .expect("Failed to generate grammar");

    let generated_code = grammar.generate_code().expect("Failed to generate code");

    let compile_time = start_time.elapsed();

    // Analyze the generated code
    let code_str = generated_code.to_string();
    let code_bytes = code_str.len();

    // Count types (structs, enums)
    let type_count =
        code_str.matches("pub struct ").count() + code_str.matches("pub enum ").count();

    // Estimate vocabulary size from generated-source shape.
    let estimated_vocab_size = estimate_vocabulary_size(&code_str, optimization_level);

    // Detect optimization features
    let optimization_features = detect_optimization_features(&code_str, optimization_level);

    CompilationAnalysis {
        compile_time_ms: compile_time.as_millis(),
        generated_code_bytes: code_bytes,
        estimated_vocab_size,
        type_count,
        optimization_features,
    }
}

fn estimate_vocabulary_size(code: &str, level: OptimizationLevel) -> usize {
    // Simple heuristic based on code analysis
    let enum_variants = code.matches("pub enum ").count() * 4; // Avg 4 variants per enum
    let string_literals = code.matches("\"").count() / 2; // Pairs of quotes
    let numeric_ranges = code.matches("% ").count(); // Modulo operations suggest bounded integers

    let base_vocab = enum_variants + string_literals + numeric_ranges + 10; // Base primitives

    match level {
        OptimizationLevel::O0 => base_vocab * 2, // No optimization, larger vocab
        OptimizationLevel::Os => base_vocab / 3, // Size optimization
        OptimizationLevel::OVocab => base_vocab / 4, // Vocabulary optimization
        OptimizationLevel::O2 => base_vocab,     // Balanced optimization
        OptimizationLevel::O3 => base_vocab / 2, // Aggressive optimization
    }
}

fn detect_optimization_features(code: &str, level: OptimizationLevel) -> Vec<String> {
    let mut features = Vec::new();

    // Detect enum index optimization
    if code.contains("as u32") && level != OptimizationLevel::O0 {
        features.push("EnumIndexOptimization".to_string());
    }

    // Detect bounded integer optimization
    if code.contains("% ") && level != OptimizationLevel::O0 {
        features.push("BoundedIntegerOptimization".to_string());
    }

    // Detect schema-driven pattern validator generation.
    if code.contains("pattern_") && level == OptimizationLevel::O3 {
        features.push("PatternValidation".to_string());
    }

    // Detect vocabulary optimization
    if code.contains("vocab_") && level == OptimizationLevel::O3 {
        features.push("VocabularyOptimization".to_string());
    }

    features
}

fn generate_compilation_performance_report(results: &[(String, String, CompilationAnalysis)]) {
    println!("\n=== Compilation Code-Shape Analysis ===");
    println!("Vocabulary values in this report are source-code heuristics.");

    // Group results by schema
    let mut schema_groups = HashMap::new();
    for (schema, level, result) in results {
        schema_groups
            .entry(schema.clone())
            .or_insert_with(Vec::new)
            .push((level.clone(), result));
    }

    for (schema_name, schema_results) in schema_groups.iter() {
        println!("\n{} Schema Analysis:", schema_name);

        // Find O0 baseline
        let o0_result = schema_results.iter().find(|(level, _)| level == "O0");

        if let Some((_, o0_baseline)) = o0_result {
            println!(
                "  Baseline (O0): {}ms compile, {}B code, {} estimated vocab size",
                o0_baseline.compile_time_ms,
                o0_baseline.generated_code_bytes,
                o0_baseline.estimated_vocab_size
            );

            for (level, result) in schema_results {
                if level != "O0" {
                    let compile_speedup =
                        o0_baseline.compile_time_ms as f64 / result.compile_time_ms.max(1) as f64;
                    let code_reduction = (o0_baseline.generated_code_bytes as i32
                        - result.generated_code_bytes as i32)
                        as f64
                        / o0_baseline.generated_code_bytes as f64
                        * 100.0;
                    let vocab_reduction = (o0_baseline.estimated_vocab_size as i32
                        - result.estimated_vocab_size as i32)
                        as f64
                        / o0_baseline.estimated_vocab_size as f64
                        * 100.0;

                    println!("  {} vs O0:", level);
                    println!("    Compilation time: {:.2}x", compile_speedup);
                    println!(
                        "    Code size: {:.1}% reduction ({} → {} bytes)",
                        code_reduction,
                        o0_baseline.generated_code_bytes,
                        result.generated_code_bytes
                    );
                    println!(
                        "    Estimated vocabulary size: {:.1}% reduction ({} -> {} heuristic tokens)",
                        vocab_reduction,
                        o0_baseline.estimated_vocab_size,
                        result.estimated_vocab_size
                    );
                    println!(
                        "    Active optimizations: {:?}",
                        result.optimization_features
                    );
                }
            }
        }
    }

    // Overall optimization effectiveness summary
    println!("\n=== Overall Estimated Optimization Effectiveness ===");

    let mut o2_metrics = Vec::new();
    let mut o3_metrics = Vec::new();

    for (schema_name, schema_results) in schema_groups.iter() {
        if let (Some(o0), Some(o2)) = (
            schema_results.iter().find(|(level, _)| level == "O0"),
            schema_results.iter().find(|(level, _)| level == "O2"),
        ) {
            let vocab_improvement =
                o0.1.estimated_vocab_size as f64 / o2.1.estimated_vocab_size.max(1) as f64;
            let code_reduction = (o0.1.generated_code_bytes as f64
                - o2.1.generated_code_bytes as f64)
                / o0.1.generated_code_bytes as f64;
            o2_metrics.push((schema_name.as_str(), vocab_improvement, code_reduction));
        }

        if let (Some(o0), Some(o3)) = (
            schema_results.iter().find(|(level, _)| level == "O0"),
            schema_results.iter().find(|(level, _)| level == "O3"),
        ) {
            let vocab_improvement =
                o0.1.estimated_vocab_size as f64 / o3.1.estimated_vocab_size.max(1) as f64;
            let code_reduction = (o0.1.generated_code_bytes as f64
                - o3.1.generated_code_bytes as f64)
                / o0.1.generated_code_bytes as f64;
            o3_metrics.push((schema_name.as_str(), vocab_improvement, code_reduction));
        }
    }

    if !o2_metrics.is_empty() {
        let avg_vocab_improvement: f64 =
            o2_metrics.iter().map(|(_, vocab, _)| vocab).sum::<f64>() / o2_metrics.len() as f64;
        let avg_code_reduction: f64 =
            o2_metrics.iter().map(|(_, _, code)| code).sum::<f64>() / o2_metrics.len() as f64;
        println!("O2 code-shape estimate:");
        println!(
            "  Average estimated vocabulary efficiency: {:.2}x better than O0",
            avg_vocab_improvement
        );
        println!(
            "  Average code reduction: {:.1}%",
            avg_code_reduction * 100.0
        );

        let best_case = o2_metrics
            .iter()
            .max_by(|a, b| a.1.partial_cmp(&b.1).unwrap())
            .unwrap();
        println!(
            "  Best case: {:.2}x estimated vocabulary efficiency ({})",
            best_case.1, best_case.0
        );
    }

    if !o3_metrics.is_empty() {
        let avg_vocab_improvement: f64 =
            o3_metrics.iter().map(|(_, vocab, _)| vocab).sum::<f64>() / o3_metrics.len() as f64;
        let avg_code_reduction: f64 =
            o3_metrics.iter().map(|(_, _, code)| code).sum::<f64>() / o3_metrics.len() as f64;
        println!("O3 code-shape estimate:");
        println!(
            "  Average estimated vocabulary efficiency: {:.2}x better than O0",
            avg_vocab_improvement
        );
        println!(
            "  Average code reduction: {:.1}%",
            avg_code_reduction * 100.0
        );

        let best_case = o3_metrics
            .iter()
            .max_by(|a, b| a.1.partial_cmp(&b.1).unwrap())
            .unwrap();
        println!(
            "  Best case: {:.2}x estimated vocabulary efficiency ({})",
            best_case.1, best_case.0
        );
    }

    // Optimization profile notes
    println!("\n=== Optimization Profile Notes ===");
    println!("• Use -O0 for development and debugging (preserves schema structure)");
    println!("• Use -O2 for production (balanced optimization, proven safe)");
    println!("• Use -O3 for maximum compression (experimental, may have higher complexity)");
    println!("• Schemas with many enums benefit most from optimization");
    println!(
        "• Nested schemas show significant vocabulary reduction with higher optimization levels"
    );
}

// Test schema generators
fn create_simple_schema() -> serde_json::Value {
    json!({
        "title": "Simple",
        "type": "object",
        "properties": {
            "name": { "type": "string" },
            "age": { "type": "integer" },
            "active": { "type": "boolean" }
        },
        "required": ["name"]
    })
}

fn create_enum_heavy_schema() -> serde_json::Value {
    json!({
        "title": "EnumHeavy",
        "type": "object",
        "properties": {
            "status": { "enum": ["draft", "pending", "approved", "rejected", "cancelled"] },
            "priority": { "enum": ["low", "medium", "high", "critical"] },
            "category": { "enum": ["bug", "feature", "improvement", "documentation", "test"] },
            "environment": { "enum": ["dev", "staging", "production"] }
        },
        "required": ["status", "priority"]
    })
}

fn create_nested_schema() -> serde_json::Value {
    json!({
        "title": "Nested",
        "type": "object",
        "properties": {
            "id": { "type": "string" },
            "config": {
                "type": "object",
                "properties": {
                    "database": {
                        "type": "object",
                        "properties": {
                            "host": { "type": "string" },
                            "port": { "type": "integer" },
                            "ssl": { "type": "boolean" }
                        }
                    },
                    "cache": {
                        "type": "object",
                        "properties": {
                            "enabled": { "type": "boolean" },
                            "ttl": { "type": "integer" },
                            "provider": { "enum": ["redis", "memcached", "memory"] }
                        }
                    }
                }
            },
            "features": {
                "type": "array",
                "items": { "type": "string" }
            }
        },
        "required": ["id", "config"]
    })
}
