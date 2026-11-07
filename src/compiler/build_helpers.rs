//! Build Helper Utilities
//!
//! This module provides internal utilities for validating configurations
//! and mapping errors during compilation.

use crate::error::{TctError, TctResult};

/// Validate compiler configuration for correctness
pub fn validate_configuration(max_vocabulary_size: Option<usize>) -> TctResult<()> {
    // Validate vocabulary size
    if let Some(size) = max_vocabulary_size
        && size == 0
    {
        return Err(TctError::InvalidConfiguration {
            field: "max_vocabulary_size".to_string(),
            reason: "Vocabulary size cannot be zero".to_string(),
            suggestion: "Use a positive integer for vocabulary size, or None for unlimited"
                .to_string(),
        });
    }

    Ok(())
}

/// Map compilation errors to appropriate TctError variants with context
pub fn map_compilation_error(error: TctError, type_name: &str) -> TctError {
    match error {
        // If it's already a well-formed TctError, pass it through
        e @ TctError::SchemaValidation { .. } => e,
        e @ TctError::SchemaProcessing { .. } => e,
        e @ TctError::CodeGeneration { .. } => e,
        e @ TctError::OptimizationFailure { .. } => e,

        // Map other errors to SchemaProcessing
        _ => TctError::SchemaProcessing {
            message: error.to_string(),
            schema_path: format!("schema.{}", type_name),
            suggestion: "Check the schema for valid JSON Schema syntax and supported features"
                .to_string(),
            phase: crate::error::ProcessingPhase::CodeGeneration,
        },
    }
}
