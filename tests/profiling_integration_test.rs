use quote::ToTokens;
use serde_json::json;
use tct::{ProfilingConfig, TctCompiler};

fn parse_generated_file(code: &str) -> syn::File {
    syn::parse_file(code).unwrap_or_else(|error| panic!("generated Rust should parse: {error}"))
}

fn collect_token_texts(stream: proc_macro2::TokenStream, output: &mut Vec<String>) {
    for token in stream {
        match token {
            proc_macro2::TokenTree::Group(group) => collect_token_texts(group.stream(), output),
            proc_macro2::TokenTree::Ident(ident) => output.push(ident.to_string()),
            proc_macro2::TokenTree::Punct(punct) => output.push(punct.as_char().to_string()),
            proc_macro2::TokenTree::Literal(literal) => output.push(literal.to_string()),
        }
    }
}

fn generated_token_texts(file: &syn::File) -> Vec<String> {
    let mut output = Vec::new();
    collect_token_texts(file.to_token_stream(), &mut output);
    output
}

fn contains_ordered_tokens(tokens: &[String], expected: &[&str]) -> bool {
    let mut expected = expected.iter();
    let Some(mut next) = expected.next() else {
        return true;
    };
    for token in tokens {
        if token == next {
            let Some(after_match) = expected.next() else {
                return true;
            };
            next = after_match;
        }
    }
    false
}

#[test]
fn test_profiling_code_generation_integration() -> Result<(), Box<dyn std::error::Error>> {
    // Simple schema for testing profiling generation
    let schema = json!({
        "type": "string",
        "enum": ["option1", "option2", "option3"]
    });

    // Test with basic profiling
    let compiler =
        TctCompiler::new(schema.clone(), "TestEnum")?.with_profiling(ProfilingConfig::basic());

    let grammar = compiler.compile()?;

    // Verify profiling code was generated
    assert!(
        grammar.profiling_code.is_some(),
        "Profiling code should be generated"
    );

    let profiling = grammar.profiling_code.as_ref().unwrap();
    assert!(
        !profiling.module_code.is_empty(),
        "Profiling module code should not be empty"
    );
    assert!(
        profiling.benchmark_file.is_none(),
        "Basic profiling should not generate Criterion benchmarks"
    );

    // Generate the complete Rust code with profiling
    let generated_code = grammar.generate_code()?;

    // Verify profiling features are present
    assert!(
        generated_code.contains("#[cfg(feature = \"profiling\")]"),
        "Generated code should contain feature-gated profiling"
    );

    assert!(
        generated_code.contains("TimedEncode"),
        "Generated code should contain TimedEncode trait"
    );

    assert!(
        generated_code.contains("encode_with_timing"),
        "Generated code should contain encode_with_timing method"
    );

    assert!(
        generated_code.contains("decode_with_timing"),
        "Generated code should contain decode_with_timing method"
    );

    assert!(
        generated_code.contains("benchmark_round_trip"),
        "Generated code should contain benchmark helper function"
    );

    println!("✅ Basic profiling integration test passed");

    // Test with benchmarking profiling
    let compiler = TctCompiler::new(schema, "TestEnum")?.with_profiling(
        ProfilingConfig::benchmarking().with_criterion_sample("option1", json!("option1")),
    );

    let grammar = compiler.compile()?;
    let profiling = grammar.profiling_code.as_ref().unwrap();

    assert!(
        profiling.benchmark_file.is_some(),
        "Benchmarking profiling should generate Criterion benchmarks"
    );

    let benchmark_content = profiling.benchmark_file.as_ref().unwrap();
    let benchmark_file = parse_generated_file(benchmark_content);
    let benchmark_tokens = generated_token_texts(&benchmark_file);
    assert!(
        benchmark_content.contains("criterion_group!"),
        "Benchmark file should contain Criterion macros"
    );
    assert!(
        benchmark_content.contains("TestEnum"),
        "Benchmark file should reference the type name"
    );
    assert!(
        contains_ordered_tokens(&benchmark_tokens, &["serde_json", "from_value", "TestEnum"]),
        "Benchmark file should deserialize caller-provided sample data"
    );
    assert!(
        !benchmark_content.contains("json!(\"example\")"),
        "Benchmark file should not use placeholder sample data"
    );

    println!("✅ Benchmarking profiling integration test passed");

    Ok(())
}

#[test]
fn test_profiling_disabled_by_default() -> Result<(), Box<dyn std::error::Error>> {
    let schema = json!({"type": "string"});

    // Default compiler should not generate profiling code
    let compiler = TctCompiler::new(schema, "SimpleString")?;
    let grammar = compiler.compile()?;

    assert!(
        grammar.profiling_code.is_none(),
        "Default compiler should not generate profiling code"
    );

    let generated_code = grammar.generate_code()?;
    assert!(
        !generated_code.contains("TimedEncode"),
        "Default generated code should not contain profiling traits"
    );

    println!("✅ Profiling disabled by default test passed");

    Ok(())
}

#[test]
fn test_profiling_with_complex_schema() -> Result<(), Box<dyn std::error::Error>> {
    // More complex schema to test profiling generation
    let schema = json!({
        "type": "object",
        "properties": {
            "name": {"type": "string"},
            "age": {"type": "integer", "minimum": 0, "maximum": 150},
            "status": {"enum": ["active", "inactive", "pending"]}
        },
        "required": ["name", "status"]
    });

    let compiler = TctCompiler::new(schema, "Person")?.with_profiling(ProfilingConfig::detailed());

    let grammar = compiler.compile()?;

    // Should generate profiling code even for complex schemas
    assert!(
        grammar.profiling_code.is_some(),
        "Complex schema should generate profiling code"
    );

    let generated_code = grammar.generate_code()?;

    // Should contain both basic and detailed profiling features
    assert!(
        generated_code.contains("TimedEncode"),
        "Should contain basic profiling"
    );
    assert!(
        generated_code.contains("detailed"),
        "Should contain detailed profiling module"
    );
    assert!(
        generated_code.contains("DetailedProfiler"),
        "Should contain detailed profiling trait"
    );

    println!("✅ Complex schema profiling test passed");

    Ok(())
}
