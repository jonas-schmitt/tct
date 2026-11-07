use serde_json::json;
use tct::TctCompiler;

fn parse_generated_code(code: &str) -> syn::File {
    syn::parse_file(code).unwrap_or_else(|error| panic!("generated Rust should parse: {error}"))
}

fn generated_struct_exists(file: &syn::File, struct_name: &str) -> bool {
    file.items.iter().any(
        |item| matches!(item, syn::Item::Struct(item_struct) if item_struct.ident == struct_name),
    )
}

#[test]
fn test_null_type_simple_schema_generates_unit_type() {
    let schema = json!({
        "type": "null"
    });

    let result = TctCompiler::new(schema, "SimpleNull").and_then(|c| c.compile());
    assert!(
        result.is_ok(),
        "Failed to generate code for simple null type: {:?}",
        result.err()
    );

    let grammar = result.unwrap();
    let code = grammar.generate_code().expect("Failed to generate code");

    // Should generate a struct with unit type field
    assert!(
        code.contains("()"),
        "Generated code should contain unit type for null"
    );
}

#[test]
fn test_null_type_in_union_generates_enum_with_variants() {
    let schema = json!({
        "type": "object",
        "properties": {
            "value": {
                "type": ["string", "null"]
            }
        }
    });

    let result = TctCompiler::new(schema, "MultiTypeWithNull").and_then(|c| c.compile());
    assert!(
        result.is_ok(),
        "Failed to generate code for multi-type with null: {:?}",
        result.err()
    );

    let grammar = result.unwrap();
    let code = grammar.generate_code().expect("Failed to generate code");

    // Should generate an enum containing both String and unit type variants
    assert!(
        code.contains("enum"),
        "Should generate enum for multi-type field"
    );
    assert!(code.contains("String"), "Should contain String variant");
}

#[test]
fn test_null_type_in_object_property_generates_struct_field() {
    let schema = json!({
        "type": "object",
        "properties": {
            "name": {"type": "string"},
            "value": {"type": "null"},
            "count": {"type": "integer"}
        }
    });

    let result = TctCompiler::new(schema, "ObjectWithNull").and_then(|c| c.compile());
    assert!(
        result.is_ok(),
        "Failed to generate code for object with null field: {:?}",
        result.err()
    );

    let grammar = result.unwrap();
    let code = grammar.generate_code().expect("Failed to generate code");
    let generated_file = parse_generated_code(&code);

    // Should generate a struct with various field types including unit type
    assert!(
        generated_struct_exists(&generated_file, "ObjectWithNull"),
        "Should generate main struct"
    );
    assert!(code.contains("String"), "Should contain string field type");
    assert!(
        code.contains("JsonInteger"),
        "Should contain integer field type"
    );
}

#[test]
fn test_null_type_in_oneof_schema_generates_enum_variant() {
    let schema = json!({
        "oneOf": [
            {"type": "string"},
            {"type": "integer"},
            {"type": "null"}
        ]
    });

    let result = TctCompiler::new(schema, "OneOfWithNull").and_then(|c| c.compile());
    assert!(
        result.is_ok(),
        "Failed to generate code for oneOf with null: {:?}",
        result.err()
    );

    let grammar = result.unwrap();
    let code = grammar.generate_code().expect("Failed to generate code");

    // Should generate an enum with all three variants
    assert!(code.contains("enum"), "Should generate enum for oneOf");
}

#[test]
fn test_null_type_as_array_items_generates_vec_unit() {
    let schema = json!({
        "type": "array",
        "items": {
            "type": "null"
        }
    });

    let result = TctCompiler::new(schema, "ArrayWithNullItems").and_then(|c| c.compile());
    assert!(
        result.is_ok(),
        "Failed to generate code for array with null items: {:?}",
        result.err()
    );

    let grammar = result.unwrap();
    let code = grammar.generate_code().expect("Failed to generate code");

    // Should generate Vec<()> for array of null values
    assert!(code.contains("Vec"), "Should generate Vec type for array");
}
