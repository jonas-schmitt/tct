//! Test streaming wheel generation and decode_prefix functionality

use proc_macro2::{TokenStream, TokenTree};
use quote::ToTokens;

fn parse_generated_lib_rs(code: &str) -> syn::File {
    syn::parse_file(code).unwrap_or_else(|error| panic!("generated lib.rs should parse: {error}"))
}

fn generated_file_has_function(file: &syn::File, name: &str) -> bool {
    file.items.iter().any(|item| match item {
        syn::Item::Fn(function) => function.sig.ident == name,
        _ => false,
    })
}

fn generated_file_has_struct(file: &syn::File, name: &str) -> bool {
    file.items.iter().any(|item| match item {
        syn::Item::Struct(item_struct) => item_struct.ident == name,
        _ => false,
    })
}

fn collect_token_texts(stream: TokenStream, tokens: &mut Vec<String>) {
    for token in stream {
        match token {
            TokenTree::Group(group) => collect_token_texts(group.stream(), tokens),
            TokenTree::Ident(ident) => tokens.push(ident.to_string()),
            TokenTree::Punct(punct) => tokens.push(punct.as_char().to_string()),
            TokenTree::Literal(literal) => tokens.push(literal.to_string()),
        }
    }
}

fn generated_token_texts(file: &syn::File) -> Vec<String> {
    let mut tokens = Vec::new();
    collect_token_texts(file.to_token_stream(), &mut tokens);
    tokens
}

fn contains_ordered_tokens(tokens: &[String], expected: &[&str]) -> bool {
    let mut position = 0;
    for token in tokens {
        if token == expected[position] {
            position += 1;
            if position == expected.len() {
                return true;
            }
        }
    }
    false
}

#[test]
#[ignore] // Run manually: cargo test --test streaming_wheel_test -- --ignored --nocapture
fn test_streaming_wheel_generation() {
    use std::fs;
    use tct::wheel_builder::WheelConfig;

    eprintln!();
    eprintln!("🔧 Testing wheel builder with streaming mode...");
    eprintln!();

    // Clean up previous test
    let output_dir = "/tmp/streaming_wheel_test";
    let _ = fs::remove_dir_all(output_dir);

    let config = WheelConfig::new(
        "schemas/popular/tsconfig.json",
        "TsConfig",
        "tct-tsconfig-streaming",
    )
    .with_version("0.1.0")
    .with_streaming_encode(true)
    .with_output_dir(output_dir);

    let wheel_path = tct::wheel_builder::build_wheel(&config).expect("Wheel build failed");

    eprintln!("✅ Wheel built: {}", wheel_path.display());

    let lib_rs_path = format!("{}/tct-tsconfig-streaming/src/lib.rs", output_dir);
    let lib_rs = fs::read_to_string(&lib_rs_path).expect("Failed to read lib.rs");
    let generated_file = parse_generated_lib_rs(&lib_rs);
    let generated_tokens = generated_token_texts(&generated_file);

    assert!(
        generated_file_has_function(&generated_file, "decode_prefix"),
        "decode_prefix function should be generated"
    );
    assert!(
        generated_file_has_function(&generated_file, "strict_prefix_roles"),
        "strict_prefix_roles function should be generated"
    );
    assert!(
        generated_file_has_struct(&generated_file, "PrefixDecodeResult"),
        "PrefixDecodeResult should be used"
    );
    assert!(
        contains_ordered_tokens(
            &generated_tokens,
            &["m", "add_function", "wrap_pyfunction", "decode_prefix"],
        ),
        "decode_prefix should be registered"
    );
    assert!(
        contains_ordered_tokens(
            &generated_tokens,
            &[
                "m",
                "add_function",
                "wrap_pyfunction",
                "strict_prefix_roles"
            ],
        ),
        "strict_prefix_roles should be registered"
    );

    eprintln!("✅ decode_prefix function found in generated code");
    eprintln!();
    eprintln!("📦 To install and test:");
    eprintln!("   cd {output_dir}/tct-tsconfig-streaming && maturin build --release");
    eprintln!("   pip install target/wheels/*.whl");
    eprintln!();
}
