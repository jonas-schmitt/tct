//! Error suggestion utilities
//!
//! This module provides utility functions and constants for generating
//! helpful error suggestions and recovery advice.

/// Utility struct for common error suggestions
pub struct ErrorSuggestions;

impl ErrorSuggestions {
    /// Default schema validation suggestion
    pub const SCHEMA_VALIDATION: &'static str = "Check your JSON Schema syntax. Ensure all properties follow the JSON Schema specification.";

    /// Default pattern analysis suggestion
    pub const PATTERN_ANALYSIS: &'static str = "Simplify your regex pattern or use a supported pattern format. Consider using enums for finite sets.";

    /// Default code generation suggestion
    pub const CODE_GENERATION: &'static str =
        "Check your schema structure for unsupported constructs or overly complex nesting.";

    /// Default optimization failure suggestion
    pub const OPTIMIZATION_FAILURE: &'static str =
        "Try using a lower optimization level (-O0 or -O2) or simplify your schema structure.";

    /// Default build integration suggestion
    pub const BUILD_INTEGRATION: &'static str =
        "Check file permissions and build configuration. Ensure output directory is writable.";

    /// Generate context-aware suggestion for schema validation errors
    pub fn for_schema_validation(schema_type: &str) -> String {
        match schema_type {
            "object" => {
                "Ensure 'properties' is an object and all property schemas are valid".to_string()
            }
            "array" => "Check that 'items' contains a valid schema definition".to_string(),
            "string" => {
                "Verify string constraints like 'minLength', 'maxLength', or 'pattern'".to_string()
            }
            "number" | "integer" => {
                "Check numeric constraints like 'minimum', 'maximum', or 'multipleOf'".to_string()
            }
            _ => Self::SCHEMA_VALIDATION.to_string(),
        }
    }
}
