//! Tests for BASE_VOCAB_SIZE and VOCAB_SIZE constants generation
//!
//! Verifies that:
//! - BASE_VOCAB_SIZE is always generated (actual structural vocabulary)
//! - VOCAB_SIZE correctly includes base + BPE + PAD
//! - Constants are accessible in generated code

use serde_json::json;
use std::fs;
use tct::bpe::{BpeMerge, BpeMergeTable, BpeMetadata};
use tct::{OptimizationLevel, TctCompiler};

fn parse_generated_code(code: &str) -> syn::File {
    syn::parse_file(code).unwrap_or_else(|error| panic!("generated Rust should parse: {error}"))
}

fn type_ends_with_ident(ty: &syn::Type, name: &str) -> bool {
    matches!(
        ty,
        syn::Type::Path(type_path)
            if type_path
                .path
                .segments
                .last()
                .is_some_and(|segment| segment.ident == name)
    )
}

fn expr_u32_literal_value(expr: &syn::Expr) -> Option<u32> {
    match expr {
        syn::Expr::Lit(expr_lit) => match &expr_lit.lit {
            syn::Lit::Int(lit_int) => lit_int.base10_parse::<u32>().ok(),
            _ => None,
        },
        _ => None,
    }
}

fn generated_file_u32_const_value(
    file: &syn::File,
    constant_name: &str,
    require_public: bool,
) -> Option<u32> {
    generated_items_u32_const_value(&file.items, constant_name, require_public)
}

fn generated_items_u32_const_value(
    items: &[syn::Item],
    constant_name: &str,
    require_public: bool,
) -> Option<u32> {
    items.iter().find_map(|item| match item {
        syn::Item::Const(item_const)
            if item_const.ident == constant_name
                && (!require_public || matches!(item_const.vis, syn::Visibility::Public(_)))
                && type_ends_with_ident(&item_const.ty, "u32") =>
        {
            expr_u32_literal_value(&item_const.expr)
        }
        syn::Item::Mod(item_mod) => item_mod.content.as_ref().and_then(|(_, items)| {
            generated_items_u32_const_value(items, constant_name, require_public)
        }),
        _ => None,
    })
}

fn generated_code_has_public_u32_const(code: &str, constant_name: &str) -> bool {
    let file = parse_generated_code(code);
    generated_file_u32_const_value(&file, constant_name, true).is_some()
}

fn write_bpe_merge_file(base_vocab_size: u32, merges: Vec<BpeMerge>) -> tempfile::NamedTempFile {
    let file = tempfile::NamedTempFile::new().expect("failed to create temp BPE file");
    let max_merges = merges.len();
    let table = BpeMergeTable {
        schema: "test".to_string(),
        base_vocab_size,
        merges,
        metadata: BpeMetadata {
            trained_on_instances: 1,
            training_date: "2026-01-01".to_string(),
            max_merges,
            dict_size: None,
        },
    };
    let json = serde_json::to_string_pretty(&table).expect("failed to serialize BPE table");
    fs::write(file.path(), json).expect("failed to write temp BPE file");
    file
}

#[test]
fn test_base_vocab_size_always_generated() {
    // Simple schema without BPE
    let schema = json!({
        "type": "object",
        "properties": {
            "status": {
                "enum": ["active", "inactive", "pending"]
            }
        }
    });

    let grammar = TctCompiler::new(schema, "Status")
        .unwrap()
        .with_optimization_level(OptimizationLevel::O2)
        .compile()
        .unwrap();

    let code = grammar.generate_code().unwrap();

    // Verify BASE_VOCAB_SIZE constant exists
    assert!(
        generated_code_has_public_u32_const(&code, "BASE_VOCAB_SIZE"),
        "BASE_VOCAB_SIZE constant must always be generated"
    );
}

#[test]
fn test_vocab_size_without_compression() {
    // Schema without BPE
    let schema = json!({
        "type": "object",
        "properties": {
            "status": {
                "enum": ["active", "inactive"]
            }
        }
    });

    let grammar = TctCompiler::new(schema, "Status")
        .unwrap()
        .with_optimization_level(OptimizationLevel::O2)
        .compile()
        .unwrap();

    let code = grammar.generate_code().unwrap();

    // Verify VOCAB_SIZE constant exists
    assert!(
        generated_code_has_public_u32_const(&code, "VOCAB_SIZE"),
        "VOCAB_SIZE constant must be generated"
    );

    // Extract BASE_VOCAB_SIZE value
    let base_vocab_size = extract_constant_value(&code, "BASE_VOCAB_SIZE");

    // Extract VOCAB_SIZE value
    let vocab_size = extract_constant_value(&code, "VOCAB_SIZE");

    // With UTF-8 encoding: VOCAB_SIZE includes 257 string bytes (0-255 + END_STRING_MARKER).
    assert!(
        vocab_size >= 257,
        "VOCAB_SIZE should include UTF-8 string bytes (257 tokens minimum)"
    );

    // VOCAB_SIZE should be at least the generated base vocabulary.
    assert!(
        vocab_size >= base_vocab_size,
        "VOCAB_SIZE ({}) should be >= BASE_VOCAB_SIZE ({})",
        vocab_size,
        base_vocab_size
    );
}

#[test]
fn test_vocab_size_with_bpe() {
    let schema = json!({"type": "string"});
    let bpe_file = write_bpe_merge_file(
        257,
        vec![
            BpeMerge {
                pair: [97, 98],
                new_token: 257,
                frequency: 10,
            },
            BpeMerge {
                pair: [257, 99],
                new_token: 258,
                frequency: 5,
            },
        ],
    );

    let grammar = TctCompiler::new(schema, "StringBpe")
        .unwrap()
        .with_bpe_merges(bpe_file.path())
        .expect("failed to load BPE")
        .compile()
        .expect("BPE compilation should succeed");

    let code = grammar.generate_code().unwrap();

    assert_eq!(extract_constant_value(&code, "BASE_VOCAB_SIZE"), 257);
    assert_eq!(
        extract_constant_value(&code, "BPE_START"),
        257,
        "BPE should start after base_vocab"
    );
    assert_eq!(
        extract_constant_value(&code, "VOCAB_SIZE"),
        260,
        "VOCAB_SIZE should be base_vocab + BPE merges + PAD"
    );
}

#[test]
fn test_base_vocab_size_with_constrained_enum() {
    // Test that BASE_VOCAB_SIZE reflects optimization
    let schema = json!({
        "type": "object",
        "properties": {
            "status": {
                "enum": ["a", "b", "c", "d", "e", "f", "g", "h", "i", "j"]
            }
        }
    });

    let grammar = TctCompiler::new(schema, "Status")
        .unwrap()
        .with_optimization_level(OptimizationLevel::O2)
        .compile()
        .unwrap();

    let code = grammar.generate_code().unwrap();

    // Extract BASE_VOCAB_SIZE
    let base_vocab_size = extract_constant_value(&code, "BASE_VOCAB_SIZE");

    // Should optimize enum with standard encoding (10 tokens for 10 variants)
    assert!(
        base_vocab_size >= 10,
        "BASE_VOCAB_SIZE should accommodate all 10 enum variants"
    );

    // Should also have VOCAB_SIZE
    let vocab_size = extract_constant_value(&code, "VOCAB_SIZE");

    // With UTF-8 encoding: VOCAB_SIZE includes 257 string bytes minimum
    assert!(
        vocab_size >= 257,
        "VOCAB_SIZE should include UTF-8 string bytes (257 tokens minimum)"
    );

    // VOCAB_SIZE should be max of string bytes and structural tokens
    assert!(
        vocab_size >= base_vocab_size,
        "VOCAB_SIZE ({}) should be >= BASE_VOCAB_SIZE ({})",
        vocab_size,
        base_vocab_size
    );
}

#[test]
fn test_vocab_size_accessible_in_code() {
    // Verify constants can be used in const expressions
    let schema = json!({
        "type": "object",
        "properties": {
            "value": {"type": "integer"}
        }
    });

    let grammar = TctCompiler::new(schema, "Test").unwrap().compile().unwrap();

    let code = grammar.generate_code().unwrap();

    // Both constants should be public
    assert!(
        generated_code_has_public_u32_const(&code, "BASE_VOCAB_SIZE"),
        "BASE_VOCAB_SIZE must be public"
    );
    assert!(
        generated_code_has_public_u32_const(&code, "VOCAB_SIZE"),
        "VOCAB_SIZE must be public"
    );
}

// Helper function to extract constant value from generated code
fn extract_constant_value(code: &str, constant_name: &str) -> u32 {
    let file = parse_generated_code(code);
    generated_file_u32_const_value(&file, constant_name, false)
        .unwrap_or_else(|| panic!("{constant_name} u32 constant not found or invalid"))
}
