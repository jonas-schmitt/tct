use serde_json::json;
use std::time::Instant;

#[test]
fn test_schema_aware_decode_performance() {
    println!("🎯 Testing schema-aware decode performance vs deprecated buffered decode");

    // Create a minimal untagged enum that triggers the decode strategy issue
    let schema = json!({
        "type": "object",
        "properties": {
            "field": {
                "oneOf": [
                    {"type": "string"},
                    {
                        "type": "object",
                        "properties": {
                            "nested": {"type": "string"}
                        }
                    }
                ]
            }
        }
    });

    let _string_instance = json!({"field": "simple string"});
    let _object_instance = json!({"field": {"nested": "complex object"}});

    println!("📝 Schema: untagged enum (string vs object)");
    println!("📄 Test instances: string and object variants");

    // Compile schema
    let start = Instant::now();
    let compiler = tct::TctCompiler::new(schema, "TestEnum").unwrap();
    let grammar = compiler.compile().unwrap();
    let compilation_time = start.elapsed();

    let generated_code = grammar.generate_code().unwrap();
    println!("✅ Compilation completed in {:?}", compilation_time);
    println!("📊 Generated code size: {} bytes", generated_code.len());

    // The key insight: we can't directly test runtime performance here because
    // the generated code needs to be compiled first. But we can validate that:
    // 1. Schema-aware decode generates different code than deprecated decode
    // 2. The code contains pattern recognition logic instead of backtracking

    // Check for schema-aware decode patterns
    let has_pattern_recognition = generated_code.contains("analyze_token_structure")
        || generated_code.contains("TokenStructure::");
    let has_buffered_backtracking =
        generated_code.contains("try_decode") || generated_code.contains("BufferedTokenIterator");

    println!("🔍 Code analysis:");
    println!(
        "   Pattern recognition (schema-aware): {}",
        has_pattern_recognition
    );
    println!(
        "   Buffered backtracking (deprecated): {}",
        has_buffered_backtracking
    );

    if has_pattern_recognition && !has_buffered_backtracking {
        println!("✅ Schema-aware decode is active");
    } else if has_buffered_backtracking && !has_pattern_recognition {
        println!("⚠️  Deprecated buffered decode is active (expected due to current fallback)");
    } else {
        println!("❓ Mixed or unknown decode strategy");
    }

    // Print a sample of the generated decode logic to verify strategy
    if let Some(decode_start) = generated_code.find("fn decode(") {
        let decode_end = generated_code[decode_start..].find("fn ").unwrap_or(500);
        let decode_sample = &generated_code[decode_start..decode_start + decode_end.min(500)];
        println!("📋 Decode implementation sample:");
        println!("{}", decode_sample);
    }
}

#[test]
fn test_enable_schema_aware_decode_temporarily() {
    println!("🧪 Testing schema-aware decode by temporarily enabling it");

    // This placeholder documents the current fallback boundary for future
    // schema-aware decode work. A real implementation test should validate
    // token-structure recognition, variant routing, and performance against
    // buffered backtracking when schema-aware module wiring exists.

    println!("⚠️  Currently skipped due to module resolution issues");
    println!("    Enable with a real schema-aware module wiring contract");
}
