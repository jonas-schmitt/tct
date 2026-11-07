//! Schema-related error types
//!
//! This module contains error types and context structures specifically
//! for JSON Schema validation and processing errors.

use std::fmt;

/// Additional context about schema validation errors
#[derive(Debug, Clone)]
pub struct SchemaContext {
    /// The type of schema element that caused the error
    pub schema_type: String,
    /// Name of the property that caused the error (if applicable)
    pub property_name: Option<String>,
    /// Expected format or constraint
    pub expected_format: Option<String>,
    /// The actual value that was received
    pub received_value: Option<String>,
}

/// Processing phases for schema processing errors
#[derive(Debug, Clone)]
pub enum ProcessingPhase {
    /// Error during schema validation phase
    Validation,
    /// Error during schema analysis phase
    Analysis,
    /// Error during circular reference detection
    CircularReferenceDetection,
    /// Error during code generation phase
    CodeGeneration,
    /// Error during post-processing phase
    PostProcessing,
}

impl fmt::Display for ProcessingPhase {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ProcessingPhase::Validation => write!(f, "validation"),
            ProcessingPhase::Analysis => write!(f, "analysis"),
            ProcessingPhase::CircularReferenceDetection => {
                write!(f, "circular reference detection")
            }
            ProcessingPhase::CodeGeneration => write!(f, "code generation"),
            ProcessingPhase::PostProcessing => write!(f, "post-processing"),
        }
    }
}
