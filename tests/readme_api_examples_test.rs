//! Test to validate README.md API examples compile and work correctly

use serde_json::json;
use tct::{OptimizationLevel, TctCompiler, TctError};

#[test]
fn test_readme_api_readme_basic_api_example() {
    let schema = json!({
        "type": "object",
        "properties": {
            "role": { "enum": ["admin", "user", "guest"] },
            "priority": { "type": "integer", "minimum": 1, "maximum": 10 }
        },
        "required": ["role", "priority"]
    });

    let grammar = TctCompiler::new(schema, "UserRequest")
        .unwrap()
        .with_optimization_level(OptimizationLevel::O2)
        .compile()
        .unwrap();

    let generated_code = grammar.generate_code().unwrap();

    // Verify the generated code contains expected optimizations
    assert!(generated_code.contains("0u32")); // Enum optimization applied

    // Debug: print what bounded integer type is actually generated
    println!("=== Searching for UserRequestPriority type ===");
    let mut found_type = false;
    for (i, line) in generated_code.lines().enumerate() {
        if line.contains("UserRequestPriority") || (found_type && i < 50) {
            println!("{}", line);
            if line.contains("UserRequestPriority") {
                found_type = true;
            }
            if found_type && line.contains("}") {
                break;
            }
        }
    }

    // Integer optimization: priority is bounded to [1, 10]
    // TCT generates a newtype wrapper for integers with bounds
    assert!(
        generated_code.contains("UserRequestPriority(pub i64)"),
        "Integer type wrapper not found"
    );

    println!("✅ Basic API example works correctly");
}

#[test]
fn test_readme_api_readme_optimization_pass_examples() {
    let schema = json!({
        "type": "object",
        "properties": {
            "role": { "enum": ["admin", "user", "guest"] }
        }
    });

    // Test individual passes
    let compiler1 = TctCompiler::new(schema.clone(), "UserRequest")
        .unwrap()
        .with_optimization_pass("enum_optimization".to_string())
        .with_optimization_pass("integer_optimization".to_string())
        .compile()
        .unwrap();

    let code1 = compiler1.generate_code().unwrap();
    assert!(code1.contains("0u32")); // Enum optimization applied

    // Test no optimization
    let compiler2 = TctCompiler::new(schema.clone(), "UserRequest")
        .unwrap()
        .with_no_optimization()
        .compile()
        .unwrap();

    let code2 = compiler2.generate_code().unwrap();
    assert!(code2.contains("String::from")); // String-based encoding

    // Test all optimizations
    let compiler3 = TctCompiler::new(schema.clone(), "UserRequest")
        .unwrap()
        .with_all_optimizations()
        .compile()
        .unwrap();

    let code3 = compiler3.generate_code().unwrap();
    assert!(code3.contains("0u32")); // Enum optimization applied

    println!("✅ Optimization pass examples work correctly");
}

#[test]
fn test_readme_api_readme_error_handling_example() {
    // Test error handling with invalid schema
    let invalid_schema = json!({ "invalid": "schema" });

    let result = TctCompiler::new(invalid_schema, "TestType");

    match result {
        Ok(_compiler) => {
            panic!("Expected error for invalid schema");
        }
        Err(TctError::SchemaValidation { message, .. }) => {
            assert!(message.contains("schema keyword"));
            println!("✅ Error handling example works correctly");
        }
        Err(e) => {
            panic!("Unexpected error type: {}", e);
        }
    }
}

#[test]
fn test_readme_api_readme_builder_methods() {
    let schema = json!({
        "type": "object",
        "properties": {
            "status": { "enum": ["active", "inactive"] }
        }
    });

    // Test all builder method examples from README

    // with_optimization_level
    let _result1 = TctCompiler::new(schema.clone(), "Test")
        .unwrap()
        .with_optimization_level(OptimizationLevel::O2)
        .compile()
        .unwrap();

    // with_optimization_pass
    let _result2 = TctCompiler::new(schema.clone(), "Test")
        .unwrap()
        .with_optimization_pass("enum_optimization".to_string())
        .compile()
        .unwrap();

    // with_no_optimization
    let _result3 = TctCompiler::new(schema.clone(), "Test")
        .unwrap()
        .with_no_optimization()
        .compile()
        .unwrap();

    // with_all_optimizations
    let _result4 = TctCompiler::new(schema.clone(), "Test")
        .unwrap()
        .with_all_optimizations()
        .compile()
        .unwrap();

    // with_stats (compiler should work)
    let _result5 = TctCompiler::new(schema.clone(), "Test")
        .unwrap()
        .with_stats()
        .compile()
        .unwrap();

    // basic compile (without stats)
    let _result6 = TctCompiler::new(schema.clone(), "Test")
        .unwrap()
        .compile()
        .unwrap();

    println!("✅ All builder methods work correctly");
}
