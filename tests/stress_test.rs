use serde_json::json;
use tct::TctCompiler;

fn parse_generated_code(code: &str) -> syn::File {
    syn::parse_file(code).unwrap_or_else(|error| panic!("generated Rust should parse: {error}"))
}

fn assert_generated_struct_with_encoder(code: &str, struct_name: &str) {
    let file = parse_generated_code(code);
    assert!(
        file.items.iter().any(
            |item| matches!(item, syn::Item::Struct(structure) if structure.ident == struct_name)
        ),
        "generated struct `{struct_name}` should exist"
    );
    assert!(
        file.items.iter().any(|item| {
            let syn::Item::Impl(item_impl) = item else {
                return false;
            };
            item_impl.trait_.as_ref().is_some_and(|(_, path, _)| {
                path.segments
                    .last()
                    .is_some_and(|segment| segment.ident == "JsonTokenEncoder")
            })
        }),
        "generated code should implement JsonTokenEncoder"
    );
}

#[test]
fn test_stress_large_schema_performance() {
    // Create a schema with many pattern properties to test performance
    let mut pattern_properties = serde_json::Map::new();

    // Add 50 different pattern properties
    for i in 0..50 {
        let pattern = format!("^pattern_{}_[a-z]+$", i);
        pattern_properties.insert(pattern, json!({"type": "string"}));
    }

    let schema = json!({
        "title": "StressTest",
        "type": "object",
        "properties": {
            "id": {"type": "string"}
        },
        "patternProperties": pattern_properties,
        "required": ["id"]
    });

    let start = std::time::Instant::now();
    let grammar = TctCompiler::new(schema.clone(), "StressTest")
        .unwrap()
        .compile()
        .expect("Failed to generate grammar");
    let code = grammar.generate_code().expect("Failed to generate code");
    let duration = start.elapsed();

    assert_generated_struct_with_encoder(&code, "StressTest");

    println!("Generated code for 50 pattern properties in {:?}", duration);
}

#[test]
fn test_stress_deeply_nested_schema_performance() {
    // Create a deeply nested schema
    let mut schema = json!({
        "title": "DeepNest",
        "type": "object",
        "properties": {
            "level_0": {
                "type": "object",
                "properties": {
                    "value": {"type": "string"}
                }
            }
        }
    });

    // Add 20 levels of nesting
    let mut current = &mut schema["properties"]["level_0"];
    for i in 1..20 {
        let next_level = json!({
            "type": "object",
            "properties": {
                "value": {"type": "string"},
                format!("level_{}", i): {
                    "type": "object",
                    "properties": {
                        "value": {"type": "string"}
                    }
                }
            }
        });

        current["properties"][format!("level_{}", i)] = next_level;
        current = &mut current["properties"][format!("level_{}", i)];
    }

    let start = std::time::Instant::now();
    let grammar = TctCompiler::new(schema.clone(), "DeepNest")
        .unwrap()
        .compile()
        .expect("Failed to generate grammar");
    let code = grammar.generate_code().expect("Failed to generate code");
    let duration = start.elapsed();

    assert_generated_struct_with_encoder(&code, "DeepNest");

    println!(
        "Generated code for 20-level nested schema in {:?}",
        duration
    );
}

#[test]
fn test_stress_complex_pattern_regex_compilation() {
    // Test complex regex patterns that might be slow to compile
    let complex_patterns = [
        r"^[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\.[a-zA-Z]{2,}$", // Email
        r"^(?:(?:25[0-5]|2[0-4][0-9]|[01]?[0-9][0-9]?)\.){3}(?:25[0-5]|2[0-4][0-9]|[01]?[0-9][0-9]?)$", // IPv4
        r"^(?:[0-9a-fA-F]{8}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{12})$", // UUID
        r"^https?://(?:[-\w.])+(?:\:[0-9]+)?(?:/(?:[\w/_.])*(?:\?(?:[\w&=%.])*)?(?:\#(?:[\w.])*)?)?$", // URL
    ];

    let mut pattern_properties = serde_json::Map::new();
    for (i, pattern) in complex_patterns.iter().enumerate() {
        pattern_properties.insert(format!("pattern_{}", i), json!({"type": "string"}));
        pattern_properties.insert(pattern.to_string(), json!({"type": "string"}));
    }

    let schema = json!({
        "title": "ComplexPatternTest",
        "type": "object",
        "properties": {
            "id": {"type": "string"}
        },
        "patternProperties": pattern_properties,
        "required": ["id"]
    });

    let start = std::time::Instant::now();
    let grammar = TctCompiler::new(schema.clone(), "ComplexPatternTest")
        .unwrap()
        .compile()
        .expect("Failed to generate grammar");
    let code = grammar.generate_code().expect("Failed to generate code");
    let duration = start.elapsed();

    assert_generated_struct_with_encoder(&code, "ComplexPatternTest");

    println!(
        "Generated code for complex regex patterns in {:?}",
        duration
    );
}

#[test]
fn test_stress_memory_usage_large_enum() {
    // Create a large enum to test memory usage
    let mut enum_values = Vec::new();
    for i in 0..1000 {
        enum_values.push(format!("value_{}", i));
    }

    let schema = json!({
        "title": "LargeEnum",
        "type": "object",
        "properties": {
            "large_field": {
                "enum": enum_values
            }
        },
        "required": ["large_field"]
    });

    let start = std::time::Instant::now();
    let grammar = TctCompiler::new(schema.clone(), "LargeEnum")
        .unwrap()
        .compile()
        .expect("Failed to generate grammar");
    let code = grammar.generate_code().expect("Failed to generate code");
    let duration = start.elapsed();

    assert_generated_struct_with_encoder(&code, "LargeEnum");

    println!("Generated code for 1000-value enum in {:?}", duration);
}
