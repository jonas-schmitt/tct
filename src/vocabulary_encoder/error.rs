//! Error handling for vocabulary encoding operations
//!
//! This module provides comprehensive error types and handling for all
//! vocabulary encoding and decoding operations.

use serde_json::Value;

/// Errors that can occur during vocabulary-aware encoding
#[derive(Debug, Clone, PartialEq)]
pub enum VocabularyEncodingError {
    /// Error during schema analysis phase
    SchemaAnalysis(crate::schema_analyzer::AnalysisError),
    /// Field not found in schema domains
    UnknownField(String),
    /// No token allocation found for field
    NoAllocation(String),
    /// Value not valid for the field's domain
    InvalidValue(String, Value),
}

impl std::fmt::Display for VocabularyEncodingError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            VocabularyEncodingError::SchemaAnalysis(err) => {
                write!(f, "Schema analysis error: {}", err)
            }
            VocabularyEncodingError::UnknownField(field) => {
                write!(
                    f,
                    "Unknown field '{}'. Available fields can be retrieved with get_optimized_fields()",
                    field
                )
            }
            VocabularyEncodingError::NoAllocation(field) => {
                write!(
                    f,
                    "No token allocation for field '{}'. Field may not be optimizable or allocation failed",
                    field
                )
            }
            VocabularyEncodingError::InvalidValue(field, value) => {
                write!(
                    f,
                    "Invalid value {:?} for field '{}'. Value must match the field's domain constraints",
                    value, field
                )
            }
        }
    }
}

impl std::error::Error for VocabularyEncodingError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            VocabularyEncodingError::SchemaAnalysis(err) => Some(err),
            _ => None,
        }
    }
}

impl VocabularyEncodingError {
    /// Get recovery suggestions for the error
    pub fn recovery_suggestion(&self) -> String {
        match self {
            VocabularyEncodingError::SchemaAnalysis(_) => {
                "Check that the JSON schema is valid and contains optimizable fields (enums, bounded integers, etc.)".to_string()
            }
            VocabularyEncodingError::UnknownField(field) => {
                format!(
                    "Verify that '{}' is defined in the schema and supports optimization. Use get_optimized_fields() to see available fields.",
                    field
                )
            }
            VocabularyEncodingError::NoAllocation(field) => {
                format!(
                    "Field '{}' may not be optimizable or vocabulary allocation failed. Check that the field has constraints (enum, min/max, etc.).",
                    field
                )
            }
            VocabularyEncodingError::InvalidValue(field, value) => {
                format!(
                    "For field '{}', value {:?} is not in the allowed domain. Check the field's constraints in the schema.",
                    field, value
                )
            }
        }
    }

    /// Check if the error is recoverable by user action
    pub fn is_recoverable(&self) -> bool {
        match self {
            VocabularyEncodingError::SchemaAnalysis(_) => true, // User can fix schema
            VocabularyEncodingError::UnknownField(_) => true,   // User can use correct field
            VocabularyEncodingError::NoAllocation(_) => false,  // System/schema issue
            VocabularyEncodingError::InvalidValue(_, _) => true, // User can provide valid value
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_display() {
        let error = VocabularyEncodingError::UnknownField("status".to_string());
        let display = format!("{}", error);
        assert!(display.contains("Unknown field 'status'"));
        assert!(display.contains("get_optimized_fields()"));
    }

    #[test]
    fn test_recovery_suggestion() {
        let error = VocabularyEncodingError::InvalidValue(
            "color".to_string(),
            Value::String("purple".to_string()),
        );
        let suggestion = error.recovery_suggestion();
        assert!(suggestion.contains("color"));
        assert!(suggestion.contains("purple"));
        assert!(suggestion.contains("domain"));
    }

    #[test]
    fn test_is_recoverable() {
        assert!(VocabularyEncodingError::UnknownField("test".to_string()).is_recoverable());
        assert!(!VocabularyEncodingError::NoAllocation("test".to_string()).is_recoverable());
    }
}
