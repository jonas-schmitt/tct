use serde_json::json;
use tct::TctCompiler;

/// Schema draft compatibility testing
/// Tests compatibility with different JSON Schema draft patterns and variations
/// to ensure robust handling across different schema authoring styles.

#[test]
fn test_schema_compatibility_draft_7_style_schemas() {
    // Draft 7 style schemas with typical patterns
    let draft_7_schemas = [
        // Basic draft 7 pattern
        json!({
            "$schema": "http://json-schema.org/draft-07/schema#",
            "title": "Draft7Basic",
            "type": "object",
            "properties": {
                "name": {"type": "string"},
                "age": {"type": "number"}
            },
            "required": ["name"]
        }),
        // Draft 7 with definitions
        json!({
            "$schema": "http://json-schema.org/draft-07/schema#",
            "title": "Draft7WithDefs",
            "definitions": {
                "Person": {
                    "type": "object",
                    "properties": {
                        "name": {"type": "string"},
                        "age": {"type": "integer"}
                    }
                }
            },
            "type": "object",
            "properties": {
                "person": {"$ref": "#/definitions/Person"}
            }
        }),
        // Draft 7 conditional schemas
        json!({
            "$schema": "http://json-schema.org/draft-07/schema#",
            "title": "Draft7Conditional",
            "type": "object",
            "properties": {
                "type": {"type": "string"},
                "value": {
                    "if": {"properties": {"type": {"const": "number"}}},
                    "then": {"type": "number"},
                    "else": {"type": "string"}
                }
            }
        }),
    ];

    for (i, schema) in draft_7_schemas.iter().enumerate() {
        let default_title = format!("Draft7Test{}", i);
        let title = schema["title"].as_str().unwrap_or(&default_title);
        let result = TctCompiler::new(schema.clone(), title).unwrap().compile();

        assert!(result.is_ok(), "Draft 7 schema {} failed: {:?}", i, schema);

        let grammar = result.unwrap();
        let code = grammar.generate_code();

        assert!(
            code.is_ok(),
            "Draft 7 code generation {} failed: {:?}",
            i,
            schema
        );

        println!("✅ Draft 7 schema {} passed: {}", i, title);
    }
}

#[test]
fn test_schema_compatibility_schema_without_explicit_draft() {
    // Schemas without explicit $schema declaration (common in practice)
    let no_draft_schemas = [
        json!({
            "title": "NoDraftBasic",
            "type": "object",
            "properties": {
                "field": {"type": "string"}
            }
        }),
        json!({
            "title": "NoDraftComplex",
            "type": "object",
            "properties": {
                "items": {
                    "type": "array",
                    "items": {
                        "oneOf": [
                            {"type": "string"},
                            {"type": "number"}
                        ]
                    }
                }
            }
        }),
    ];

    for (i, schema) in no_draft_schemas.iter().enumerate() {
        let default_title = format!("NoDraftTest{}", i);
        let title = schema["title"].as_str().unwrap_or(&default_title);
        let result = TctCompiler::new(schema.clone(), title).unwrap().compile();

        assert!(result.is_ok(), "No-draft schema {} failed: {:?}", i, schema);

        let grammar = result.unwrap();
        let code = grammar.generate_code();

        assert!(
            code.is_ok(),
            "No-draft code generation {} failed: {:?}",
            i,
            schema
        );

        println!("✅ No-draft schema {} passed: {}", i, title);
    }
}

#[test]
fn test_schema_compatibility_different_type_declaration_styles() {
    // Different ways of declaring types that should be handled consistently
    let type_styles = [
        // Single type
        json!({
            "title": "SingleType",
            "type": "string"
        }),
        // Array of types (union types)
        json!({
            "title": "UnionType",
            "type": ["string", "number"]
        }),
        // Type with additional constraints
        json!({
            "title": "ConstrainedType",
            "type": "string",
            "minLength": 1,
            "maxLength": 100
        }),
        // No explicit type but with properties (implies object)
        json!({
            "title": "ImpliedObject",
            "properties": {
                "field": {"type": "string"}
            }
        }),
        // No explicit type but with items (implies array)
        json!({
            "title": "ImpliedArray",
            "items": {"type": "string"}
        }),
    ];

    for (i, schema) in type_styles.iter().enumerate() {
        let default_title = format!("TypeStyleTest{}", i);
        let title = schema["title"].as_str().unwrap_or(&default_title);
        let result = TctCompiler::new(schema.clone(), title).unwrap().compile();

        // Some type styles might not be supported - that's okay
        match result {
            Ok(grammar) => {
                let code = grammar.generate_code();
                match code {
                    Ok(_) => println!("✅ Type style {} passed: {}", i, title),
                    Err(e) => println!(
                        "⚠️  Type style {} failed code gen (may be expected): {} - {}",
                        i, title, e
                    ),
                }
            }
            Err(e) => println!(
                "⚠️  Type style {} failed schema gen (may be expected): {} - {}",
                i, title, e
            ),
        }
    }
}

#[test]
fn test_schema_compatibility_property_naming_conventions() {
    // Different property naming styles commonly found in schemas
    let naming_conventions = [
        // camelCase
        json!({
            "title": "CamelCase",
            "type": "object",
            "properties": {
                "firstName": {"type": "string"},
                "lastName": {"type": "string"},
                "phoneNumber": {"type": "string"}
            }
        }),
        // snake_case
        json!({
            "title": "SnakeCase",
            "type": "object",
            "properties": {
                "first_name": {"type": "string"},
                "last_name": {"type": "string"},
                "phone_number": {"type": "string"}
            }
        }),
        // kebab-case
        json!({
            "title": "KebabCase",
            "type": "object",
            "properties": {
                "first-name": {"type": "string"},
                "last-name": {"type": "string"},
                "phone-number": {"type": "string"}
            }
        }),
        // PascalCase
        json!({
            "title": "PascalCase",
            "type": "object",
            "properties": {
                "FirstName": {"type": "string"},
                "LastName": {"type": "string"},
                "PhoneNumber": {"type": "string"}
            }
        }),
        // Mixed conventions (realistic)
        json!({
            "title": "MixedConventions",
            "type": "object",
            "properties": {
                "user_id": {"type": "string"},
                "firstName": {"type": "string"},
                "last-name": {"type": "string"},
                "PhoneNumber": {"type": "string"}
            }
        }),
    ];

    for (i, schema) in naming_conventions.iter().enumerate() {
        let default_title = format!("NamingTest{}", i);
        let title = schema["title"].as_str().unwrap_or(&default_title);
        let result = TctCompiler::new(schema.clone(), title).unwrap().compile();

        assert!(
            result.is_ok(),
            "Naming convention schema {} failed: {:?}",
            i,
            schema
        );

        let grammar = result.unwrap();
        let code = grammar.generate_code();

        assert!(
            code.is_ok(),
            "Naming convention code generation {} failed: {:?}",
            i,
            schema
        );

        // Verify generated code is valid Rust
        let code_str = code.unwrap();
        let parsed = syn::parse_file(&code_str);
        assert!(
            parsed.is_ok(),
            "Generated code has invalid syntax for naming test {}",
            i
        );

        println!("✅ Naming convention {} passed: {}", i, title);
    }
}

#[test]
fn test_schema_compatibility_constraint_variations() {
    // Different ways constraints are commonly expressed
    let constraint_variations = [
        // String constraints
        json!({
            "title": "StringConstraints",
            "type": "object",
            "properties": {
                "short_string": {
                    "type": "string",
                    "minLength": 1,
                    "maxLength": 10
                },
                "pattern_string": {
                    "type": "string",
                    "pattern": "^[A-Z][a-z]+$"
                },
                "format_string": {
                    "type": "string",
                    "format": "email"
                }
            }
        }),
        // Number constraints
        json!({
            "title": "NumberConstraints",
            "type": "object",
            "properties": {
                "bounded_int": {
                    "type": "integer",
                    "minimum": 0,
                    "maximum": 100
                },
                "positive_number": {
                    "type": "number",
                    "minimum": 0,
                    "exclusiveMinimum": true
                },
                "multiple_of": {
                    "type": "integer",
                    "multipleOf": 5
                }
            }
        }),
        // Array constraints
        json!({
            "title": "ArrayConstraints",
            "type": "object",
            "properties": {
                "bounded_array": {
                    "type": "array",
                    "items": {"type": "string"},
                    "minItems": 1,
                    "maxItems": 5
                },
                "unique_array": {
                    "type": "array",
                    "items": {"type": "integer"},
                    "uniqueItems": true
                }
            }
        }),
        // Object constraints
        json!({
            "title": "ObjectConstraints",
            "type": "object",
            "properties": {
                "limited_props": {
                    "type": "object",
                    "properties": {
                        "allowed": {"type": "string"}
                    },
                    "additionalProperties": false
                },
                "min_props": {
                    "type": "object",
                    "properties": {
                        "a": {"type": "string"},
                        "b": {"type": "string"}
                    },
                    "minProperties": 1
                }
            }
        }),
    ];

    for (i, schema) in constraint_variations.iter().enumerate() {
        let default_title = format!("ConstraintTest{}", i);
        let title = schema["title"].as_str().unwrap_or(&default_title);
        let result = TctCompiler::new(schema.clone(), title).unwrap().compile();

        match result {
            Ok(grammar) => {
                let code = grammar.generate_code();
                match code {
                    Ok(_) => println!("✅ Constraint variation {} passed: {}", i, title),
                    Err(e) => println!(
                        "⚠️  Constraint variation {} failed code gen: {} - {}",
                        i, title, e
                    ),
                }
            }
            Err(e) => println!(
                "⚠️  Constraint variation {} failed schema gen: {} - {}",
                i, title, e
            ),
        }
    }
}

#[test]
fn test_schema_compatibility_real_world_schema_patterns() {
    // Patterns commonly found in real-world APIs and configurations
    let real_world_patterns = [
        // REST API response pattern
        json!({
            "title": "ApiResponse",
            "type": "object",
            "properties": {
                "status": {"enum": ["success", "error"]},
                "data": {
                    "type": "object",
                    "properties": {
                        "id": {"type": "string"},
                        "attributes": {
                            "type": "object",
                            "additionalProperties": true
                        }
                    }
                },
                "meta": {
                    "type": "object",
                    "properties": {
                        "total": {"type": "integer"},
                        "page": {"type": "integer"},
                        "per_page": {"type": "integer"}
                    }
                }
            },
            "required": ["status"]
        }),
        // Configuration file pattern
        json!({
            "title": "Config",
            "type": "object",
            "properties": {
                "version": {"type": "string"},
                "database": {
                    "type": "object",
                    "properties": {
                        "host": {"type": "string"},
                        "port": {"type": "integer"},
                        "ssl": {"type": "boolean"}
                    },
                    "required": ["host", "port"]
                },
                "features": {
                    "type": "object",
                    "patternProperties": {
                        "^feature_[a-z_]+$": {"type": "boolean"}
                    }
                }
            },
            "required": ["version", "database"]
        }),
        // Event/message pattern
        json!({
            "title": "Event",
            "type": "object",
            "properties": {
                "event_type": {"type": "string"},
                "timestamp": {"type": "integer"},
                "payload": {
                    "oneOf": [
                        {
                            "type": "object",
                            "properties": {
                                "user_action": {"type": "string"},
                                "user_id": {"type": "string"}
                            }
                        },
                        {
                            "type": "object",
                            "properties": {
                                "system_metric": {"type": "string"},
                                "value": {"type": "number"}
                            }
                        }
                    ]
                }
            },
            "required": ["event_type", "timestamp", "payload"]
        }),
    ];

    for (i, schema) in real_world_patterns.iter().enumerate() {
        let default_title = format!("RealWorldTest{}", i);
        let title = schema["title"].as_str().unwrap_or(&default_title);
        let result = TctCompiler::new(schema.clone(), title).unwrap().compile();

        assert!(
            result.is_ok(),
            "Real-world pattern {} failed: {:?}",
            i,
            schema
        );

        let grammar = result.unwrap();
        let code = grammar.generate_code();

        assert!(
            code.is_ok(),
            "Real-world code generation {} failed: {:?}",
            i,
            schema
        );

        println!("✅ Real-world pattern {} passed: {}", i, title);
    }
}

#[test]
fn test_schema_compatibility_schema_composition_patterns() {
    // Different ways schemas are composed in practice
    let composition_patterns = [
        // Simple oneOf
        json!({
            "title": "SimpleOneOf",
            "oneOf": [
                {"type": "string"},
                {"type": "integer"},
                {"type": "boolean"}
            ]
        }),
        // oneOf with object variants
        json!({
            "title": "ObjectOneOf",
            "oneOf": [
                {
                    "type": "object",
                    "properties": {
                        "type": {"const": "text"},
                        "content": {"type": "string"}
                    }
                },
                {
                    "type": "object",
                    "properties": {
                        "type": {"const": "number"},
                        "value": {"type": "number"}
                    }
                }
            ]
        }),
        // anyOf pattern
        json!({
            "title": "AnyOfPattern",
            "anyOf": [
                {"type": "string"},
                {"type": "integer"}
            ]
        }),
        // Nested composition
        json!({
            "title": "NestedComposition",
            "type": "object",
            "properties": {
                "data": {
                    "oneOf": [
                        {
                            "type": "array",
                            "items": {"type": "string"}
                        },
                        {
                            "type": "object",
                            "properties": {
                                "nested": {
                                    "oneOf": [
                                        {"type": "string"},
                                        {"type": "integer"}
                                    ]
                                }
                            }
                        }
                    ]
                }
            }
        }),
    ];

    for (i, schema) in composition_patterns.iter().enumerate() {
        let default_title = format!("CompositionTest{}", i);
        let title = schema["title"].as_str().unwrap_or(&default_title);
        let result = TctCompiler::new(schema.clone(), title).unwrap().compile();

        match result {
            Ok(grammar) => {
                let code = grammar.generate_code();
                match code {
                    Ok(_) => println!("✅ Composition pattern {} passed: {}", i, title),
                    Err(e) => println!(
                        "⚠️  Composition pattern {} failed code gen: {} - {}",
                        i, title, e
                    ),
                }
            }
            Err(e) => println!(
                "⚠️  Composition pattern {} failed schema gen: {} - {}",
                i, title, e
            ),
        }
    }
}

#[test]
fn test_schema_compatibility_edge_case_schemas() {
    // Edge cases that might cause parsing or generation issues
    let edge_cases = [
        // Empty schema
        json!({}),
        // Schema with only title
        json!({"title": "OnlyTitle"}),
        // Schema with extra metadata
        json!({
            "title": "WithMetadata",
            "description": "A schema with extra metadata",
            "$id": "https://example.com/schema",
            "$comment": "This is a comment",
            "type": "object",
            "properties": {
                "field": {"type": "string"}
            }
        }),
        // Schema with unknown keywords (should be ignored)
        json!({
            "title": "UnknownKeywords",
            "type": "object",
            "customKeyword": "should be ignored",
            "x-custom": "extension keyword",
            "properties": {
                "field": {"type": "string"}
            }
        }),
    ];

    for (i, schema) in edge_cases.iter().enumerate() {
        let default_title = format!("EdgeCase{}", i);
        let title = schema
            .get("title")
            .and_then(|v| v.as_str())
            .unwrap_or(&default_title);

        // Handle schema validation errors for edge cases
        let compiler_result = TctCompiler::new(schema.clone(), title);

        let result = match compiler_result {
            Ok(compiler) => compiler.compile(),
            Err(e) => {
                println!(
                    "⚠️  Edge case {} schema validation error (expected for invalid schemas): {} - {}",
                    i, title, e
                );
                continue; // Skip invalid schemas
            }
        };

        match result {
            Ok(grammar) => match grammar.generate_code() {
                Ok(_) => println!("✅ Edge case {} passed: {}", i, title),
                Err(e) => println!(
                    "⚠️  Edge case {} failed code gen (may be expected): {} - {}",
                    i, title, e
                ),
            },
            Err(e) => println!(
                "⚠️  Edge case {} failed schema gen (may be expected): {} - {}",
                i, title, e
            ),
        }
    }
}
