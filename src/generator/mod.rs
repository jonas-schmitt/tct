//! Code generation engine for JSON Schema to Rust conversion
//!
//! This module contains the core code generation logic that transforms JSON schemas
//! into optimized Rust types with tokenization support. It handles enum generation,
//! struct generation, optimization passes, and vocabulary management.

// Note: type_mapper module available for future refactoring

// --- Sub-modules for organized code structure ---
pub mod additional_properties_analyzer;
pub mod bounded_integer_generator;
pub mod code_generator;
pub mod encoding_decision;
pub mod enum_generator;
pub(crate) mod external_schemas;
pub mod generation_output;
pub(crate) mod json_schema_validator;
pub mod optimization_phase;
pub mod passes;
pub mod pattern_validator_generator;
pub mod profiling_generator;
pub mod root_generator;
pub mod schema_discovery;
pub mod schema_walker;
pub mod struct_generator;
pub mod structural_codec;
pub mod type_alias_generator;
pub mod type_analysis;
pub mod type_analyzer;

#[doc(hidden)]
pub use generation_output as code_generation;

// --- Unit Tests ---
#[cfg(test)]
mod tests;

#[cfg(test)]
mod pattern_validation_tests;

// Legacy Generator implementation removed - using three-pass architecture
// See PassOrchestrator for the current compilation system

// --- Helper Functions ---
