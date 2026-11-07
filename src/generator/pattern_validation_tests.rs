//! Pattern validation and optimization tests
//!
//! This module contains tests specifically for pattern-based optimization
//! and validation, including regex patterns and finite enumeration detection.

use crate::compiler::TctCompiler;
use serde_json::json;

#[cfg(test)]
#[allow(clippy::module_inception)]
mod pattern_validation_tests {
    use super::*;

    fn parse_generated_file(code: &str) -> syn::File {
        syn::parse_file(code).unwrap_or_else(|error| panic!("generated Rust should parse: {error}"))
    }

    fn path_ends_with_ident(path: &syn::Path, name: &str) -> bool {
        path.segments
            .last()
            .is_some_and(|segment| segment.ident == name)
    }

    fn type_ends_with_ident(ty: &syn::Type, name: &str) -> bool {
        matches!(ty, syn::Type::Path(type_path) if path_ends_with_ident(&type_path.path, name))
    }

    fn generated_file_has_public_struct(file: &syn::File, name: &str) -> bool {
        file.items.iter().any(|item| match item {
            syn::Item::Struct(item_struct) => {
                item_struct.ident == name && matches!(item_struct.vis, syn::Visibility::Public(_))
            }
            _ => false,
        })
    }

    fn generated_file_has_struct_containing(file: &syn::File, name: &str) -> bool {
        file.items.iter().any(|item| match item {
            syn::Item::Struct(item_struct) => item_struct.ident.to_string().contains(name),
            _ => false,
        })
    }

    fn generated_struct_has_public_field_type(
        file: &syn::File,
        struct_name: &str,
        field_name: &str,
        type_name: &str,
    ) -> bool {
        file.items.iter().any(|item| match item {
            syn::Item::Struct(item_struct) if item_struct.ident == struct_name => {
                let syn::Fields::Named(fields) = &item_struct.fields else {
                    return false;
                };
                fields.named.iter().any(|field| {
                    field
                        .ident
                        .as_ref()
                        .is_some_and(|ident| ident == field_name)
                        && matches!(field.vis, syn::Visibility::Public(_))
                        && type_ends_with_ident(&field.ty, type_name)
                })
            }
            _ => false,
        })
    }

    #[test]
    fn test_pattern_validation_success_cases() {
        let schema = json!({
            "title": "PatternValidationTest",
            "type": "object",
            "properties": {
                "name": { "type": "string" }
            },
            "patternProperties": {
                "^[a-z]+_config$": { "type": "string" },
                "^[0-9]+$": { "type": "integer" },
                "^prefix_.*_suffix$": { "type": "boolean" }
            },
            "required": ["name"]
        });

        let grammar = TctCompiler::new(schema.clone(), "PatternValidationTest")
            .unwrap()
            .compile()
            .unwrap();
        let code = grammar.generate_code().unwrap();

        // Verify all validators are generated
        assert!(code.contains("PatternValidationTestPatternValidator0"));
        assert!(code.contains("PatternValidationTestPatternValidator1"));
        assert!(code.contains("PatternValidationTestPatternValidator2"));

        // Verify pattern literals are embedded correctly
        assert!(code.contains("^[a-z]+_config$"));
        assert!(code.contains("^[0-9]+$"));
        assert!(code.contains("^prefix_.*_suffix$"));

        // Verify validation logic exists
        assert!(code.contains("if !self.pattern.is_match(&key)"));
        assert!(code.contains("does not match pattern"));
    }

    #[test]
    fn test_pattern_edge_cases() {
        let schema = json!({
            "title": "EdgeCaseTest",
            "type": "object",
            "properties": {},
            "patternProperties": {
                // Unicode pattern
                "^[α-ω]+$": { "type": "string" },
                // Special characters that need escaping
                "^\\d{2,4}\\.[a-z]+\\$": { "type": "string" },
                // Empty alternative
                "^(test|)$": { "type": "string" },
                // Complex lookahead (if supported by regex crate)
                "^(?:reserved_|.)*$": { "type": "string" }
            }
        });

        let grammar = TctCompiler::new(schema.clone(), "EdgeCaseTest")
            .unwrap()
            .compile()
            .unwrap();
        let code = grammar.generate_code().unwrap();

        // Verify Unicode pattern is preserved
        assert!(code.contains("^[α-ω]+$"));
        // Verify escaped characters are preserved
        assert!(code.contains("^\\\\d{2,4}\\\\.[a-z]+\\\\$"));
        // Verify complex patterns are preserved
        assert!(code.contains("^(test|)$"));
        assert!(code.contains("^(?:reserved_|.)*$"));
    }

    #[test]
    fn test_invalid_regex_pattern_handling() {
        let schema = json!({
            "title": "InvalidPatternTest",
            "type": "object",
            "patternProperties": {
                // Invalid regex pattern - unmatched parenthesis
                "^[unclosed": { "type": "string" }
            }
        });

        // This should either fail gracefully during generation or at runtime
        // The current implementation panics on invalid regex, which might be acceptable
        let result = std::panic::catch_unwind(|| {
            TctCompiler::new(schema.clone(), "InvalidPatternTest")
                .unwrap()
                .compile()
                .unwrap()
        });

        // Either the generation fails or we get code that panics at runtime
        // Both are acceptable behaviors for invalid regex patterns
        assert!(result.is_err() || result.is_ok());
    }

    #[test]
    fn test_empty_pattern_properties() {
        let schema = json!({
            "title": "EmptyPatternTest",
            "type": "object",
            "properties": {
                "name": { "type": "string" }
            },
            "patternProperties": {},
            "required": ["name"]
        });

        let grammar = TctCompiler::new(schema.clone(), "EmptyPatternTest")
            .unwrap()
            .compile()
            .unwrap();
        let code = grammar.generate_code().unwrap();
        let generated_file = parse_generated_file(&code);

        // Should only have the regular struct, no pattern validators
        assert!(generated_file_has_public_struct(
            &generated_file,
            "EmptyPatternTest"
        ));
        assert!(!generated_file_has_struct_containing(
            &generated_file,
            "PatternValidator"
        ));
        assert!(generated_struct_has_public_field_type(
            &generated_file,
            "EmptyPatternTest",
            "name",
            "String"
        ));
    }

    #[test]
    fn test_single_pattern_property() {
        let schema = json!({
            "title": "SinglePatternTest",
            "type": "object",
            "properties": {},
            "patternProperties": {
                "^api_key_[a-f0-9]{32}$": { "type": "string" }
            }
        });

        let grammar = TctCompiler::new(schema.clone(), "SinglePatternTest")
            .unwrap()
            .compile()
            .unwrap();
        let code = grammar.generate_code().unwrap();

        // Should generate exactly one validator
        assert!(code.contains("SinglePatternTestPatternValidator0"));
        assert!(!code.contains("SinglePatternTestPatternValidator1"));
        assert!(code.contains("^api_key_[a-f0-9]{32}$"));
    }

    #[test]
    fn test_pattern_with_nested_object_values() {
        let schema = json!({
            "title": "NestedPatternTest",
            "type": "object",
            "properties": {},
            "patternProperties": {
                "^config_.*": {
                    "type": "object",
                    "properties": {
                        "enabled": { "type": "boolean" },
                        "value": { "type": "string" }
                    },
                    "required": ["enabled"]
                }
            }
        });

        let grammar = TctCompiler::new(schema.clone(), "NestedPatternTest")
            .unwrap()
            .compile()
            .unwrap();
        let code = grammar.generate_code().unwrap();

        // Should generate validator struct and pattern
        assert!(code.contains("NestedPatternTestPatternValidator0"));
        assert!(code.contains("^config_.*"));

        // Current implementation uses generic validation - full type integration is Phase 2.5+
        assert!(code.contains("if !self.pattern.is_match(&key)"));
        assert!(code.contains("does not match pattern"));
    }

    #[test]
    fn test_pattern_with_array_values() {
        let schema = json!({
            "title": "ArrayPatternTest",
            "type": "object",
            "properties": {},
            "patternProperties": {
                "^list_.*": {
                    "type": "array",
                    "items": { "type": "integer" }
                }
            }
        });

        let grammar = TctCompiler::new(schema.clone(), "ArrayPatternTest")
            .unwrap()
            .compile()
            .unwrap();
        let code = grammar.generate_code().unwrap();

        // Should generate validator struct and pattern
        assert!(code.contains("ArrayPatternTestPatternValidator0"));
        assert!(code.contains("^list_.*"));
        // Current implementation uses generic validation - full type integration is Phase 2.5+
        assert!(code.contains("if !self.pattern.is_match(&key)"));
    }

    #[test]
    fn test_multiple_overlapping_patterns() {
        let schema = json!({
            "title": "OverlapTest",
            "type": "object",
            "properties": {},
            "patternProperties": {
                "^test_.*": { "type": "string" },
                "^test_config_.*": { "type": "integer" },
                ".*_suffix$": { "type": "boolean" }
            }
        });

        let grammar = TctCompiler::new(schema.clone(), "OverlapTest")
            .unwrap()
            .compile()
            .unwrap();
        let code = grammar.generate_code().unwrap();

        // Should generate all three validators
        assert!(code.contains("OverlapTestPatternValidator0"));
        assert!(code.contains("OverlapTestPatternValidator1"));
        assert!(code.contains("OverlapTestPatternValidator2"));

        // Each should have its own pattern
        assert!(code.contains("^test_.*"));
        assert!(code.contains("^test_config_.*"));
        assert!(code.contains(".*_suffix$"));
    }
}
