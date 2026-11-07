//! Comprehensive error handling for TCT
//!
//! This module provides structured, actionable error types with detailed context
//! and recovery suggestions for better developer experience.

use std::{error::Error, fmt};

// Sub-modules for organized error handling
pub mod generation_errors;
pub mod pattern_errors;
pub mod schema_errors;
pub mod suggestions;

// Re-export commonly used types
pub use generation_errors::GenerationPhase;
pub use pattern_errors::{PatternContext, PatternType};
pub use schema_errors::{ProcessingPhase, SchemaContext};
pub use suggestions::ErrorSuggestions;

/// Main error type for TCT operations
#[derive(Debug)]
pub enum TctError {
    /// Schema validation failed
    SchemaValidation {
        /// Error description
        message: String,
        /// Path within the schema where the error occurred
        path: String,
        /// Suggestion for fixing the error
        suggestion: String,
        /// Additional context about the schema
        context: Option<Box<SchemaContext>>,
    },
    /// Code generation failed
    CodeGeneration {
        /// Error description
        message: String,
        /// Path within the schema where the error occurred
        schema_path: String,
        /// Suggestion for fixing the error
        suggestion: String,
        /// The phase of generation where the error occurred
        phase: GenerationPhase,
    },
    /// Optimization pass failed
    OptimizationFailure {
        /// Name of the optimization pass that failed
        pass_name: String,
        /// Error description
        message: String,
        /// Suggestion for fixing the error
        suggestion: String,
        /// Path within the schema where the error occurred
        schema_path: String,
    },
    /// Pattern analysis error
    PatternAnalysis {
        /// The regex pattern that caused the error
        pattern: String,
        /// Error description
        message: String,
        /// Suggestion for fixing the error
        suggestion: String,
        /// Additional context about the pattern
        context: Box<PatternContext>,
    },
    /// Build system integration error
    Build {
        /// Error description
        message: String,
        /// Path to the file that caused the error
        file_path: String,
        /// Suggestion for fixing the error
        suggestion: String,
        /// Underlying error that caused this error
        source: Option<Box<dyn Error>>,
    },
    /// I/O related errors
    Io {
        /// The I/O operation that failed
        operation: String,
        /// Path to the file that caused the error
        path: String,
        /// Underlying I/O error
        source: std::io::Error,
        /// Suggestion for fixing the error
        suggestion: String,
    },
    /// JSON parsing errors
    JsonParse {
        /// Path to the JSON file that failed to parse
        path: String,
        /// Line number where the error occurred (if available)
        line: Option<usize>,
        /// Column number where the error occurred (if available)
        column: Option<usize>,
        /// Underlying JSON parsing error
        source: serde_json::Error,
        /// Suggestion for fixing the error
        suggestion: String,
    },
    /// Invalid configuration for TctCompiler
    InvalidConfiguration {
        /// The configuration field that is invalid
        field: String,
        /// Reason why the configuration is invalid
        reason: String,
        /// Suggestion for fixing the configuration
        suggestion: String,
    },
    /// Schema processing error during compilation
    SchemaProcessing {
        /// Error description
        message: String,
        /// Path within the schema where the error occurred
        schema_path: String,
        /// Suggestion for fixing the error
        suggestion: String,
        /// The processing phase where the error occurred
        phase: ProcessingPhase,
    },
}

/// Type alias for Results with TctError
pub type TctResult<T> = Result<T, TctError>;

impl fmt::Display for TctError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TctError::SchemaValidation {
                message,
                path,
                suggestion,
                context,
            } => {
                writeln!(f, "Schema validation error at '{}':", path)?;
                writeln!(f, "  {}", message)?;

                if let Some(ctx) = context {
                    if let Some(prop) = &ctx.property_name {
                        writeln!(f, "  Property: {}", prop)?;
                    }
                    if let Some(expected) = &ctx.expected_format {
                        writeln!(f, "  Expected: {}", expected)?;
                    }
                    if let Some(received) = &ctx.received_value {
                        writeln!(f, "  Received: {}", received)?;
                    }
                }

                writeln!(f, "  💡 Suggestion: {}", suggestion)
            }
            TctError::CodeGeneration {
                message,
                schema_path,
                suggestion,
                phase,
            } => {
                writeln!(f, "Code generation error during {} phase:", phase)?;
                writeln!(f, "  Schema path: {}", schema_path)?;
                writeln!(f, "  {}", message)?;
                writeln!(f, "  💡 Suggestion: {}", suggestion)
            }
            TctError::OptimizationFailure {
                pass_name,
                message,
                suggestion,
                schema_path,
            } => {
                writeln!(f, "Optimization pass '{}' failed:", pass_name)?;
                writeln!(f, "  Schema path: {}", schema_path)?;
                writeln!(f, "  {}", message)?;
                writeln!(f, "  💡 Suggestion: {}", suggestion)
            }
            TctError::PatternAnalysis {
                pattern,
                message,
                suggestion,
                context,
            } => {
                writeln!(f, "Pattern analysis error for '{}':", pattern)?;
                writeln!(f, "  Type: {}", context.pattern_type)?;
                writeln!(f, "  {}", message)?;

                if let Some(hint) = &context.complexity_hint {
                    writeln!(f, "  Complexity: {}", hint)?;
                }

                if !context.supported_features.is_empty() {
                    writeln!(
                        f,
                        "  Supported features: {}",
                        context.supported_features.join(", ")
                    )?;
                }

                writeln!(f, "  💡 Suggestion: {}", suggestion)
            }
            TctError::Build {
                message,
                file_path,
                suggestion,
                source,
            } => {
                writeln!(f, "Build integration error:")?;
                writeln!(f, "  File: {}", file_path)?;
                writeln!(f, "  {}", message)?;

                if let Some(source_err) = source {
                    writeln!(f, "  Caused by: {}", source_err)?;
                }

                writeln!(f, "  💡 Suggestion: {}", suggestion)
            }
            TctError::Io {
                operation,
                path,
                source,
                suggestion,
            } => {
                writeln!(f, "I/O error during {} operation:", operation)?;
                writeln!(f, "  File: {}", path)?;
                writeln!(f, "  Error: {}", source)?;
                writeln!(f, "  💡 Suggestion: {}", suggestion)
            }
            TctError::JsonParse {
                path,
                line,
                column,
                source,
                suggestion,
            } => {
                write!(f, "JSON parsing error in '{}'", path)?;

                if let (Some(line), Some(col)) = (line, column) {
                    write!(f, " at line {}, column {}", line, col)?;
                }

                writeln!(f, ":")?;
                writeln!(f, "  {}", source)?;
                writeln!(f, "  💡 Suggestion: {}", suggestion)
            }
            TctError::InvalidConfiguration {
                field,
                reason,
                suggestion,
            } => {
                writeln!(f, "Invalid configuration for field '{}':", field)?;
                writeln!(f, "  {}", reason)?;
                writeln!(f, "  💡 Suggestion: {}", suggestion)
            }
            TctError::SchemaProcessing {
                message,
                schema_path,
                suggestion,
                phase,
            } => {
                writeln!(f, "Schema processing error during {} phase:", phase)?;
                writeln!(f, "  Path: {}", schema_path)?;
                writeln!(f, "  {}", message)?;
                writeln!(f, "  💡 Suggestion: {}", suggestion)
            }
        }
    }
}

impl Error for TctError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            TctError::Build {
                source: Some(err), ..
            } => Some(err.as_ref()),
            TctError::Io { source, .. } => Some(source),
            TctError::JsonParse { source, .. } => Some(source),
            _ => None,
        }
    }
}

// Conversion implementations for common error types
impl From<Box<dyn Error>> for TctError {
    fn from(err: Box<dyn Error>) -> Self {
        TctError::CodeGeneration {
            message: err.to_string(),
            schema_path: "unknown".to_string(),
            suggestion: ErrorSuggestions::SCHEMA_VALIDATION.to_string(),
            phase: GenerationPhase::TypeGeneration,
        }
    }
}

impl From<&str> for TctError {
    fn from(msg: &str) -> Self {
        TctError::CodeGeneration {
            message: msg.to_string(),
            schema_path: "unknown".to_string(),
            suggestion: "Check the schema structure and ensure all required properties are present"
                .to_string(),
            phase: GenerationPhase::TypeGeneration,
        }
    }
}

impl From<String> for TctError {
    fn from(msg: String) -> Self {
        TctError::CodeGeneration {
            message: msg,
            schema_path: "unknown".to_string(),
            suggestion: "Check the schema structure and ensure all required properties are present"
                .to_string(),
            phase: GenerationPhase::TypeGeneration,
        }
    }
}

impl From<std::io::Error> for TctError {
    fn from(err: std::io::Error) -> Self {
        TctError::Io {
            operation: "file operation".to_string(),
            path: "unknown".to_string(),
            source: err,
            suggestion: ErrorSuggestions::BUILD_INTEGRATION.to_string(),
        }
    }
}

impl From<serde_json::Error> for TctError {
    fn from(err: serde_json::Error) -> Self {
        TctError::json_parse(
            "unknown".to_string(),
            err,
            "Check JSON syntax and ensure valid JSON format".to_string(),
        )
    }
}

impl TctError {
    /// Create a schema validation error with rich context
    pub fn schema_validation(
        message: impl Into<String>,
        path: impl Into<String>,
        suggestion: impl Into<String>,
    ) -> Self {
        TctError::SchemaValidation {
            message: message.into(),
            path: path.into(),
            suggestion: suggestion.into(),
            context: None,
        }
    }

    /// Create a schema validation error with detailed context
    pub fn schema_validation_with_context(
        message: impl Into<String>,
        path: impl Into<String>,
        suggestion: impl Into<String>,
        context: SchemaContext,
    ) -> Self {
        TctError::SchemaValidation {
            message: message.into(),
            path: path.into(),
            suggestion: suggestion.into(),
            context: Some(Box::new(context)),
        }
    }

    /// Create a code generation error
    pub fn code_generation(
        message: impl Into<String>,
        schema_path: impl Into<String>,
        suggestion: impl Into<String>,
        phase: GenerationPhase,
    ) -> Self {
        TctError::CodeGeneration {
            message: message.into(),
            schema_path: schema_path.into(),
            suggestion: suggestion.into(),
            phase,
        }
    }

    /// Create an optimization failure error
    pub fn optimization_failure(
        pass_name: impl Into<String>,
        message: impl Into<String>,
        suggestion: impl Into<String>,
        schema_path: impl Into<String>,
    ) -> Self {
        TctError::OptimizationFailure {
            pass_name: pass_name.into(),
            message: message.into(),
            suggestion: suggestion.into(),
            schema_path: schema_path.into(),
        }
    }

    /// Create a pattern analysis error
    pub fn pattern_analysis(
        pattern: impl Into<String>,
        message: impl Into<String>,
        suggestion: impl Into<String>,
        context: PatternContext,
    ) -> Self {
        TctError::PatternAnalysis {
            pattern: pattern.into(),
            message: message.into(),
            suggestion: suggestion.into(),
            context: Box::new(context),
        }
    }

    /// Create a build integration error
    pub fn build(
        message: impl Into<String>,
        file_path: impl Into<String>,
        suggestion: impl Into<String>,
    ) -> Self {
        TctError::Build {
            message: message.into(),
            file_path: file_path.into(),
            suggestion: suggestion.into(),
            source: None,
        }
    }

    /// Create a build integration error with source cause
    pub fn build_with_source(
        message: impl Into<String>,
        file_path: impl Into<String>,
        suggestion: impl Into<String>,
        source: Box<dyn Error>,
    ) -> Self {
        TctError::Build {
            message: message.into(),
            file_path: file_path.into(),
            suggestion: suggestion.into(),
            source: Some(source),
        }
    }

    /// Create an I/O error
    pub fn io(
        operation: impl Into<String>,
        path: impl Into<String>,
        source: std::io::Error,
        suggestion: impl Into<String>,
    ) -> Self {
        TctError::Io {
            operation: operation.into(),
            path: path.into(),
            source,
            suggestion: suggestion.into(),
        }
    }

    /// Create a JSON parsing error
    pub fn json_parse(
        path: impl Into<String>,
        source: serde_json::Error,
        suggestion: impl Into<String>,
    ) -> Self {
        let line = source.line();
        let column = source.column();

        TctError::JsonParse {
            path: path.into(),
            line: Some(line),
            column: Some(column),
            source,
            suggestion: suggestion.into(),
        }
    }

    /// Create an invalid configuration error
    pub fn invalid_configuration(
        field: impl Into<String>,
        reason: impl Into<String>,
        suggestion: impl Into<String>,
    ) -> Self {
        TctError::InvalidConfiguration {
            field: field.into(),
            reason: reason.into(),
            suggestion: suggestion.into(),
        }
    }

    /// Create a schema processing error
    pub fn schema_processing(
        message: impl Into<String>,
        schema_path: impl Into<String>,
        suggestion: impl Into<String>,
        phase: ProcessingPhase,
    ) -> Self {
        TctError::SchemaProcessing {
            message: message.into(),
            schema_path: schema_path.into(),
            suggestion: suggestion.into(),
            phase,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::error::Error;

    #[test]
    fn error_suggestions_are_context_aware_with_default_fallback() {
        let cases = [
            (
                "object",
                "Ensure 'properties' is an object and all property schemas are valid",
            ),
            (
                "array",
                "Check that 'items' contains a valid schema definition",
            ),
            (
                "string",
                "Verify string constraints like 'minLength', 'maxLength', or 'pattern'",
            ),
            (
                "number",
                "Check numeric constraints like 'minimum', 'maximum', or 'multipleOf'",
            ),
            (
                "integer",
                "Check numeric constraints like 'minimum', 'maximum', or 'multipleOf'",
            ),
            ("boolean", ErrorSuggestions::SCHEMA_VALIDATION),
        ];

        for (schema_type, expected) in cases {
            assert_eq!(
                ErrorSuggestions::for_schema_validation(schema_type),
                expected
            );
        }

        assert!(ErrorSuggestions::PATTERN_ANALYSIS.contains("regex pattern"));
        assert!(ErrorSuggestions::CODE_GENERATION.contains("schema structure"));
        assert!(ErrorSuggestions::OPTIMIZATION_FAILURE.contains("optimization level"));
        assert!(ErrorSuggestions::BUILD_INTEGRATION.contains("permissions"));
    }

    #[test]
    fn phase_and_pattern_display_names_are_stable() {
        let generation = [
            (GenerationPhase::SchemaAnalysis, "schema analysis"),
            (GenerationPhase::TypeGeneration, "type generation"),
            (
                GenerationPhase::OptimizationApplication,
                "optimization application",
            ),
            (GenerationPhase::CodeFormatting, "code formatting"),
        ];
        for (phase, expected) in generation {
            assert_eq!(phase.to_string(), expected);
        }

        let processing = [
            (ProcessingPhase::Validation, "validation"),
            (ProcessingPhase::Analysis, "analysis"),
            (
                ProcessingPhase::CircularReferenceDetection,
                "circular reference detection",
            ),
            (ProcessingPhase::CodeGeneration, "code generation"),
            (ProcessingPhase::PostProcessing, "post-processing"),
        ];
        for (phase, expected) in processing {
            assert_eq!(phase.to_string(), expected);
        }

        let pattern_types = [
            (PatternType::SimpleAlternation, "simple alternation (a|b|c)"),
            (PatternType::CharacterClass, "character class ([a-z])"),
            (PatternType::Quantifier, "quantifier (*, +, ?, {})"),
            (PatternType::ComplexRegex, "complex regex"),
            (PatternType::Unknown, "unknown pattern type"),
        ];
        for (pattern_type, expected) in pattern_types {
            assert_eq!(pattern_type.to_string(), expected);
        }
    }

    #[test]
    fn rich_error_display_includes_context_and_suggestions() {
        let basic_schema_error =
            TctError::schema_validation("missing type", "$", "add a type").to_string();
        assert!(basic_schema_error.contains("Schema validation error at '$'"));
        assert!(!basic_schema_error.contains("Property:"));

        let schema_error = TctError::schema_validation_with_context(
            "invalid property schema",
            "$.properties.name",
            "make name a string schema",
            SchemaContext {
                schema_type: "object".to_string(),
                property_name: Some("name".to_string()),
                expected_format: Some("JSON Schema object".to_string()),
                received_value: Some("true".to_string()),
            },
        )
        .to_string();
        assert!(schema_error.contains("Schema validation error"));
        assert!(schema_error.contains("$.properties.name"));
        assert!(schema_error.contains("Property: name"));
        assert!(schema_error.contains("Expected: JSON Schema object"));
        assert!(schema_error.contains("Received: true"));
        assert!(schema_error.contains("make name a string schema"));

        let pattern_error = TctError::pattern_analysis(
            "^[a-z]+$",
            "unsupported anchors",
            "use a simpler pattern",
            PatternContext {
                pattern_type: PatternType::ComplexRegex,
                complexity_hint: Some("anchors plus character class".to_string()),
                supported_features: vec!["alternation".to_string(), "literals".to_string()],
            },
        )
        .to_string();
        assert!(pattern_error.contains("Pattern analysis error"));
        assert!(pattern_error.contains("Type: complex regex"));
        assert!(pattern_error.contains("Complexity: anchors plus character class"));
        assert!(pattern_error.contains("Supported features: alternation, literals"));
        assert!(pattern_error.contains("use a simpler pattern"));

        let simple_pattern_error = TctError::pattern_analysis(
            "abc",
            "not optimizable",
            "keep it as a runtime regex",
            PatternContext {
                pattern_type: PatternType::Unknown,
                complexity_hint: None,
                supported_features: Vec::new(),
            },
        )
        .to_string();
        assert!(simple_pattern_error.contains("Type: unknown pattern type"));
        assert!(!simple_pattern_error.contains("Complexity:"));
        assert!(!simple_pattern_error.contains("Supported features:"));

        let code_error = TctError::code_generation(
            "missing root type",
            "$",
            "check root schema",
            GenerationPhase::TypeGeneration,
        )
        .to_string();
        assert!(code_error.contains("type generation phase"));
        assert!(code_error.contains("Schema path: $"));

        let optimization_error = TctError::optimization_failure(
            "field-layout",
            "could not refine members",
            "disable the pass",
            "$.properties",
        )
        .to_string();
        assert!(optimization_error.contains("Optimization pass 'field-layout' failed"));
        assert!(optimization_error.contains("disable the pass"));

        let processing_error = TctError::schema_processing(
            "cycle detected",
            "$.definitions.node",
            "break the cycle",
            ProcessingPhase::CircularReferenceDetection,
        )
        .to_string();
        assert!(processing_error.contains("circular reference detection phase"));
        assert!(processing_error.contains("Path: $.definitions.node"));
    }

    #[test]
    fn io_json_build_and_configuration_errors_preserve_sources() {
        let build = TctError::build("write failed", "target/out.rs", "check output directory");
        assert!(build.source().is_none());
        assert!(build.to_string().contains("File: target/out.rs"));

        let build_with_source = TctError::build_with_source(
            "copy failed",
            "target/lib.rs",
            "retry the copy",
            Box::new(std::io::Error::other("disk full")),
        );
        assert_eq!(
            build_with_source
                .source()
                .expect("source should be preserved")
                .to_string(),
            "disk full"
        );
        assert!(
            build_with_source
                .to_string()
                .contains("Caused by: disk full")
        );

        let io_error = TctError::io(
            "read",
            "schema.json",
            std::io::Error::new(std::io::ErrorKind::NotFound, "missing"),
            "check the path",
        );
        assert!(io_error.source().is_some());
        assert!(
            io_error
                .to_string()
                .contains("I/O error during read operation")
        );

        let json_source = serde_json::from_str::<serde_json::Value>("{ invalid").unwrap_err();
        let json_error = TctError::json_parse("schema.json", json_source, "fix syntax");
        assert!(json_error.source().is_some());
        let rendered = json_error.to_string();
        assert!(rendered.contains("JSON parsing error in 'schema.json'"));
        assert!(rendered.contains("line 1, column 3"));

        let config_error =
            TctError::invalid_configuration("max_base_vocab", "too small", "use at least 257")
                .to_string();
        assert!(config_error.contains("Invalid configuration for field 'max_base_vocab'"));
        assert!(config_error.contains("use at least 257"));
    }

    #[test]
    fn conversions_create_actionable_tct_errors() {
        let boxed_error: Box<dyn Error> = Box::new(std::fmt::Error);
        let boxed: TctError = boxed_error.into();
        match boxed {
            TctError::CodeGeneration {
                message,
                phase: GenerationPhase::TypeGeneration,
                suggestion,
                ..
            } => {
                assert!(!message.is_empty());
                assert_eq!(suggestion, ErrorSuggestions::SCHEMA_VALIDATION);
            }
            other => panic!("unexpected boxed conversion: {other:?}"),
        }

        let borrowed: TctError = "plain failure".into();
        assert!(matches!(
            borrowed,
            TctError::CodeGeneration {
                phase: GenerationPhase::TypeGeneration,
                ..
            }
        ));

        let owned: TctError = String::from("owned failure").into();
        assert!(owned.to_string().contains("owned failure"));

        let io_error: TctError = std::io::Error::other("permission denied").into();
        assert!(io_error.source().is_some());
        assert!(io_error.to_string().contains("file operation"));

        let json_error: TctError = serde_json::from_str::<serde_json::Value>("{ missing")
            .unwrap_err()
            .into();
        assert!(json_error.source().is_some());
        assert!(
            json_error
                .to_string()
                .contains("Check JSON syntax and ensure valid JSON format")
        );
    }
}
