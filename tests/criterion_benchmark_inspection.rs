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

fn has_string_literal(tokens: &[String], expected: &str) -> bool {
    tokens.iter().any(|token| {
        syn::parse_str::<syn::LitStr>(token).is_ok_and(|literal| literal.value() == expected)
    })
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
fn inspect_generated_criterion_benchmark() -> Result<(), Box<dyn std::error::Error>> {
    // Simple schema for testing benchmark generation
    let schema = json!({
        "type": "string",
        "enum": ["red", "green", "blue"]
    });

    // Compile with benchmarking profiling
    let compiler = TctCompiler::new(schema, "Color")?.with_profiling(
        ProfilingConfig::benchmarking()
            .with_criterion_sample("red", json!("red"))
            .with_criterion_sample("green", json!("green")),
    );

    let grammar = compiler.compile()?;

    // Check if benchmark file was generated
    if let Some(profiling) = &grammar.profiling_code {
        if let Some(benchmark_content) = &profiling.benchmark_file {
            println!("📊 Generated Criterion benchmark file:");
            println!("{}", benchmark_content);
            let generated_file = parse_generated_file(benchmark_content);
            let tokens = generated_token_texts(&generated_file);

            // Verify basic structure
            assert!(benchmark_content.contains("criterion_group!"));
            assert!(benchmark_content.contains("criterion_main!"));
            assert!(benchmark_content.contains("Color"));
            assert!(benchmark_content.contains("benchmark_encode"));
            assert!(benchmark_content.contains("benchmark_decode"));
            assert!(benchmark_content.contains("benchmark_round_trip"));
            assert!(contains_ordered_tokens(
                &tokens,
                &["serde_json", "from_value", "Color"]
            ));
            assert!(has_string_literal(&tokens, "red"));
            assert!(has_string_literal(&tokens, "\"red\""));
            assert!(contains_ordered_tokens(
                &tokens,
                &["serde_json", "from_str"]
            ));
            assert!(!benchmark_content.contains("json!(\"example\")"));

            println!("✅ Basic benchmark structure verified");
        } else {
            panic!("❌ No benchmark file generated");
        }
    } else {
        panic!("❌ No profiling code generated");
    }

    Ok(())
}
