use serde_json::json;
use std::time::Instant;
use tct::{OptimizationLevel, TctCompiler};

#[test]
fn test_direct_package_performance() {
    // Load the package.json schema
    let schema_path = "schemas/popular/package.json";
    let schema_content =
        std::fs::read_to_string(schema_path).expect("Failed to read package.json schema");
    let schema: serde_json::Value =
        serde_json::from_str(&schema_content).expect("Failed to parse schema");

    let test_instance: serde_json::Value = serde_json::from_str(include_str!(
        "data/instances/package/synthetic/public_cli.json"
    ))
    .expect("Authored package fixture should be valid JSON");

    println!("🚀 Starting direct performance test within TCT crate");
    println!(
        "📄 Test instance size: {} bytes",
        serde_json::to_string(&test_instance).unwrap().len()
    );

    // Time schema compilation
    let start = Instant::now();
    let compiler = TctCompiler::new(schema, "PackageTest")
        .expect("Failed to create compiler")
        .with_optimization_level(OptimizationLevel::O2);
    let grammar = compiler.compile().expect("Failed to compile schema");
    let compilation_time = start.elapsed();
    println!("⏱️  Schema compilation: {:?}", compilation_time);

    // Time code generation
    let start = Instant::now();
    let generated_code = grammar.generate_code().expect("Failed to generate code");
    let generation_time = start.elapsed();
    println!("⏱️  Code generation: {:?}", generation_time);
    println!("📊 Generated code size: {} bytes", generated_code.len());

    // For direct encode/decode testing, we'd need to actually compile and run the generated code
    // This test establishes baseline for compilation overhead vs actual execution

    println!("✅ Direct test completed - no external crate compilation overhead");
    println!();
    println!("🔍 Analysis:");
    println!("   - If round-trip test takes 7-11s for same operations → framework overhead");
    println!("   - If this test shows similar times → algorithmic issues");
    println!(
        "   - Generated code size: {}KB",
        generated_code.len() / 1024
    );
}

#[test]
fn test_minimal_enum_performance() {
    // Test with minimal untagged enum to isolate decode strategy performance
    let minimal_schema = json!({
        "type": "object",
        "properties": {
            "value": {
                "oneOf": [
                    {"type": "string"},
                    {"type": "object", "properties": {"nested": {"type": "string"}}}
                ]
            }
        }
    });

    let _test_instance = json!({
        "value": "simple string"
    });

    println!("🧪 Testing minimal untagged enum performance");

    let start = Instant::now();
    let compiler = TctCompiler::new(minimal_schema, "MinimalTest")
        .expect("Failed to create compiler")
        .with_optimization_level(OptimizationLevel::O2);
    let grammar = compiler.compile().expect("Failed to compile schema");
    let compilation_time = start.elapsed();

    let start = Instant::now();
    let generated_code = grammar.generate_code().expect("Failed to generate code");
    let generation_time = start.elapsed();

    println!("⏱️  Minimal schema compilation: {:?}", compilation_time);
    println!("⏱️  Minimal code generation: {:?}", generation_time);
    println!(
        "📊 Minimal generated code size: {} bytes",
        generated_code.len()
    );

    // This gives us baseline performance for simple cases
    println!("✅ Minimal test completed");
}
