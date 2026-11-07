//! Code generation error types
//!
//! This module contains error types specifically related to code generation
//! phases and their associated context.

use std::fmt;

/// Code generation phase where error occurred
#[derive(Debug, Clone, Copy)]
pub enum GenerationPhase {
    /// Error during schema analysis phase
    SchemaAnalysis,
    /// Error during type generation phase
    TypeGeneration,
    /// Error during optimization application phase
    OptimizationApplication,
    /// Error during code formatting phase
    CodeFormatting,
}

impl fmt::Display for GenerationPhase {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            GenerationPhase::SchemaAnalysis => write!(f, "schema analysis"),
            GenerationPhase::TypeGeneration => write!(f, "type generation"),
            GenerationPhase::OptimizationApplication => write!(f, "optimization application"),
            GenerationPhase::CodeFormatting => write!(f, "code formatting"),
        }
    }
}
