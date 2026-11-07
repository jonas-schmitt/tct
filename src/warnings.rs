//! Schema quality warnings for optimization guidance
//!
//! This module provides warnings about schema patterns that may result in
//! suboptimal code generation, helping developers improve their schemas
//! for better performance and type safety.

use std::fmt;

/// Types of schema quality warnings
#[derive(Debug, Clone, PartialEq)]
pub enum WarningType {
    /// Unconstrained additionalProperties that could benefit from explicit constraints
    UnconstrainedAdditionalProperties,
    /// Generic serde_json::Value usage where more specific types are possible
    GenericValueUsage,
    /// Large enum that might benefit from vocabulary optimization
    LargeEnumVocabulary,
    /// Pattern properties that could be optimized
    SuboptimalPatternProperties,
    /// Missing optimization opportunities
    MissedOptimization,
    /// Pattern that is too complex for enumeration optimization
    ComplexPatternOptimization,
    /// Pattern with unbounded repetition cannot be enumerated
    UnboundedPatternOptimization,
    /// Invalid regex pattern that cannot be analyzed
    InvalidPatternSyntax,
    /// Vocabulary constraint forcing multi-token pattern encoding
    VocabularyConstraintImpact,
}

/// Severity levels for warnings
#[derive(Debug, Clone, PartialEq, PartialOrd)]
pub enum WarningSeverity {
    /// Informational - minor optimization opportunity
    Info,
    /// Warning - significant optimization opportunity
    Warning,
    /// Important - major performance impact possible
    Important,
}

/// Schema quality warning with actionable suggestions
#[derive(Debug, Clone)]
pub struct SchemaWarning {
    /// Type of warning
    pub warning_type: WarningType,
    /// Severity level
    pub severity: WarningSeverity,
    /// Path in the schema where the issue occurs
    pub schema_path: String,
    /// Human-readable warning message
    pub message: String,
    /// Specific suggestion for improvement
    pub suggestion: String,
    /// Example of improved schema (optional)
    pub example: Option<String>,
    /// Estimated performance impact
    pub impact: OptimizationImpact,
}

/// Estimated impact of implementing the suggested optimization
#[derive(Debug, Clone)]
pub struct OptimizationImpact {
    /// Estimated token reduction (if applicable)
    pub token_reduction: Option<f64>,
    /// Estimated compilation time impact
    pub compilation_impact: CompilationImpact,
    /// Type safety improvement
    pub type_safety: TypeSafetyImpact,
}

/// Impact on compilation time
#[derive(Debug, Clone, PartialEq)]
pub enum CompilationImpact {
    /// No significant impact
    Neutral,
    /// May increase compilation time slightly
    SlightIncrease,
    /// May reduce compilation time
    Improvement,
}

/// Impact on type safety
#[derive(Debug, Clone, PartialEq)]
pub enum TypeSafetyImpact {
    /// No change in type safety
    None,
    /// Improved type safety
    Improved,
    /// Significantly improved type safety
    SignificantlyImproved,
}

/// Collection of schema warnings
#[derive(Debug, Clone)]
pub struct SchemaWarnings {
    /// List of warnings
    pub warnings: Vec<SchemaWarning>,
}

impl SchemaWarnings {
    /// Create new empty warnings collection
    pub fn new() -> Self {
        Self {
            warnings: Vec::new(),
        }
    }

    /// Add a warning to the collection
    pub fn add(&mut self, warning: SchemaWarning) {
        self.warnings.push(warning);
    }

    /// Check if there are any warnings
    pub fn is_empty(&self) -> bool {
        self.warnings.is_empty()
    }

    /// Get number of warnings
    pub fn len(&self) -> usize {
        self.warnings.len()
    }

    /// Get warnings by severity
    pub fn by_severity(&self, severity: WarningSeverity) -> Vec<&SchemaWarning> {
        self.warnings
            .iter()
            .filter(|w| w.severity == severity)
            .collect()
    }

    /// Get warnings by type
    pub fn by_type(&self, warning_type: WarningType) -> Vec<&SchemaWarning> {
        self.warnings
            .iter()
            .filter(|w| w.warning_type == warning_type)
            .collect()
    }

    /// Generate a formatted summary of all warnings
    pub fn summary(&self) -> String {
        if self.warnings.is_empty() {
            return "✅ No schema quality issues found".to_string();
        }

        let mut summary = String::new();
        summary.push_str(&format!(
            "⚠️  Found {} schema quality issue(s):\n\n",
            self.warnings.len()
        ));

        // Group by severity
        for severity in [
            WarningSeverity::Important,
            WarningSeverity::Warning,
            WarningSeverity::Info,
        ] {
            let warnings = self.by_severity(severity.clone());
            if !warnings.is_empty() {
                let icon = match severity {
                    WarningSeverity::Important => "🔴",
                    WarningSeverity::Warning => "🟡",
                    WarningSeverity::Info => "🔵",
                };
                summary.push_str(&format!(
                    "{} {} ({} issue(s)):\n",
                    icon,
                    severity,
                    warnings.len()
                ));

                for warning in warnings {
                    summary.push_str(&format!(
                        "  • {}: {}\n",
                        warning.schema_path, warning.message
                    ));
                    summary.push_str(&format!("    💡 {}\n", warning.suggestion));
                    if let Some(ref example) = warning.example {
                        summary.push_str(&format!("    📄 Example: {}\n", example));
                    }
                    if let Some(reduction) = warning.impact.token_reduction {
                        summary.push_str(&format!(
                            "    📈 Estimated token reduction: {:.1}%\n",
                            reduction * 100.0
                        ));
                    }
                    summary.push('\n');
                }
            }
        }

        summary
    }
}

impl Default for SchemaWarnings {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for WarningSeverity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            WarningSeverity::Info => write!(f, "Info"),
            WarningSeverity::Warning => write!(f, "Warning"),
            WarningSeverity::Important => write!(f, "Important"),
        }
    }
}

impl fmt::Display for WarningType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            WarningType::UnconstrainedAdditionalProperties => {
                write!(f, "Unconstrained additionalProperties")
            }
            WarningType::GenericValueUsage => write!(f, "Generic Value usage"),
            WarningType::LargeEnumVocabulary => write!(f, "Large enum vocabulary"),
            WarningType::SuboptimalPatternProperties => write!(f, "Suboptimal pattern properties"),
            WarningType::MissedOptimization => write!(f, "Missed optimization"),
            WarningType::ComplexPatternOptimization => write!(f, "Complex pattern optimization"),
            WarningType::UnboundedPatternOptimization => {
                write!(f, "Unbounded pattern optimization")
            }
            WarningType::InvalidPatternSyntax => write!(f, "Invalid pattern syntax"),
            WarningType::VocabularyConstraintImpact => write!(f, "Vocabulary constraint impact"),
        }
    }
}

/// Helper functions for creating common warnings
impl SchemaWarning {
    /// Create warning for unconstrained additionalProperties
    pub fn unconstrained_additional_properties(
        schema_path: String,
        optimization_level: crate::optimization_pass::OptimizationLevel,
    ) -> Self {
        use crate::optimization_pass::OptimizationLevel;

        let (message, suggestion, impact) = match optimization_level {
            OptimizationLevel::O0 => (
                "Using additionalProperties: true will encode all values as strings, losing type information".to_string(),
                "Consider adding explicit type constraints like {\"type\": \"string\"} or using enum constraints for known values".to_string(),
                OptimizationImpact {
                    token_reduction: Some(0.3), // 30% potential improvement
                    compilation_impact: CompilationImpact::Neutral,
                    type_safety: TypeSafetyImpact::SignificantlyImproved,
                }
            ),
            OptimizationLevel::O2 => (
                "Using additionalProperties: true with generic type preservation - consider explicit constraints for better optimization".to_string(),
                "Add specific type constraints like {\"type\": \"string\"} or enum values like {\"enum\": [\"value1\", \"value2\"]} for optimal encoding".to_string(),
                OptimizationImpact {
                    token_reduction: Some(0.2), // 20% potential improvement
                    compilation_impact: CompilationImpact::Neutral,
                    type_safety: TypeSafetyImpact::Improved,
                }
            ),
            OptimizationLevel::O3 | OptimizationLevel::Os | OptimizationLevel::OVocab => (
                "Contextual optimization in use - explicit constraints could provide more predictable optimization".to_string(),
                "For maximum performance, replace additionalProperties: true with specific constraints based on your data".to_string(),
                OptimizationImpact {
                    token_reduction: Some(0.15), // 15% potential improvement
                    compilation_impact: CompilationImpact::Improvement,
                    type_safety: TypeSafetyImpact::Improved,
                }
            ),
        };

        Self {
            warning_type: WarningType::UnconstrainedAdditionalProperties,
            severity: WarningSeverity::Warning,
            schema_path,
            message,
            suggestion,
            example: Some(
                r#"
// Instead of:
"additionalProperties": true

// Use specific constraints:
"additionalProperties": {"type": "string"}
// or
"additionalProperties": {"enum": ["low", "medium", "high"]}
// or
"additionalProperties": {
  "type": "object",
  "properties": {
    "enabled": {"type": "boolean"},
    "value": {"type": "number"}
  }
}
"#
                .to_string(),
            ),
            impact,
        }
    }

    /// Create warning for generic Value usage
    pub fn generic_value_usage(schema_path: String, reason: String) -> Self {
        Self {
            warning_type: WarningType::GenericValueUsage,
            severity: WarningSeverity::Info,
            schema_path,
            message: format!("Using generic serde_json::Value encoding: {}", reason),
            suggestion:
                "Consider adding more specific type constraints to enable optimized encoding"
                    .to_string(),
            example: Some(
                r#"
// Instead of generic Value, use:
{"type": "string"}           // for string values
{"type": "number"}           // for numeric values
{"enum": ["a", "b", "c"]}   // for finite sets
"#
                .to_string(),
            ),
            impact: OptimizationImpact {
                token_reduction: Some(0.25),
                compilation_impact: CompilationImpact::Neutral,
                type_safety: TypeSafetyImpact::Improved,
            },
        }
    }

    /// Create warning for large enum vocabulary
    pub fn large_enum_vocabulary(schema_path: String, enum_size: usize) -> Self {
        Self {
            warning_type: WarningType::LargeEnumVocabulary,
            severity: if enum_size > 100 {
                WarningSeverity::Important
            } else {
                WarningSeverity::Warning
            },
            schema_path,
            message: format!(
                "Large enum with {} values may benefit from vocabulary optimization",
                enum_size
            ),
            suggestion: format!(
                "Consider using OptimizationLevel::OVocab for enums with {} values",
                enum_size
            ),
            example: Some(
                r#"
TctCompiler::new(schema, "MyType")
    .with_optimization_level(OptimizationLevel::OVocab)
    .compile()
"#
                .to_string(),
            ),
            impact: OptimizationImpact {
                token_reduction: Some(0.4),
                compilation_impact: CompilationImpact::SlightIncrease,
                type_safety: TypeSafetyImpact::None,
            },
        }
    }

    /// Create warning for complex pattern that cannot be optimized
    pub fn complex_pattern_optimization(schema_path: String, pattern: String) -> Self {
        Self {
            warning_type: WarningType::ComplexPatternOptimization,
            severity: WarningSeverity::Info,
            schema_path,
            message: format!("Pattern '{}' is too complex for enumeration optimization", pattern),
            suggestion: "Consider simplifying the pattern or using explicit enum values if the set is finite".to_string(),
            example: Some(r#"
// Instead of complex pattern:
"pattern": "^[a-zA-Z0-9]{8,16}$"

// Use explicit constraints if possible:
"enum": ["value1", "value2", "value3"]
// or use simpler patterns:
"pattern": "^(small|medium|large)$"
"#.to_string()),
            impact: OptimizationImpact {
                token_reduction: Some(0.0), // No current optimization, but potential if simplified
                compilation_impact: CompilationImpact::Neutral,
                type_safety: TypeSafetyImpact::None,
            },
        }
    }

    /// Create warning for unbounded pattern that cannot be enumerated
    pub fn unbounded_pattern_optimization(schema_path: String, pattern: String) -> Self {
        Self {
            warning_type: WarningType::UnboundedPatternOptimization,
            severity: WarningSeverity::Info,
            schema_path,
            message: format!(
                "Pattern '{}' contains unbounded repetition and cannot be enumerated",
                pattern
            ),
            suggestion: "Replace with explicit enum values if the valid set is known and finite"
                .to_string(),
            example: Some(
                r#"
// Instead of unbounded pattern:
"pattern": "^test.*$"

// Use explicit values if known:
"enum": ["test1", "test2", "test_config"]
// or add specific constraints:
"pattern": "^test(1|2|3)$"
"#
                .to_string(),
            ),
            impact: OptimizationImpact {
                token_reduction: Some(0.0), // No current optimization possible
                compilation_impact: CompilationImpact::Neutral,
                type_safety: TypeSafetyImpact::None,
            },
        }
    }

    /// Create warning for invalid regex pattern syntax
    pub fn invalid_pattern_syntax(schema_path: String, pattern: String, error: String) -> Self {
        Self {
            warning_type: WarningType::InvalidPatternSyntax,
            severity: WarningSeverity::Warning,
            schema_path,
            message: format!("Invalid regex pattern '{}': {}", pattern, error),
            suggestion: "Fix the regex syntax or use simpler string constraints".to_string(),
            example: Some(
                r#"
// Fix common regex syntax errors:
"pattern": "^(value1|value2)$"  // ✅ Proper closing parenthesis
"pattern": "^[a-z]+$"           // ✅ Proper character class
"pattern": "^\\d{3}$"           // ✅ Escaped backslashes in JSON
"#
                .to_string(),
            ),
            impact: OptimizationImpact {
                token_reduction: Some(0.0), // Cannot optimize invalid patterns
                compilation_impact: CompilationImpact::Neutral,
                type_safety: TypeSafetyImpact::None,
            },
        }
    }

    /// Create warning for vocabulary constraint impact on pattern encoding
    pub fn vocabulary_constraint_impact(
        schema_path: String,
        pattern_values: usize,
        max_vocabulary: usize,
    ) -> Self {
        Self {
            warning_type: WarningType::VocabularyConstraintImpact,
            severity: WarningSeverity::Info,
            schema_path,
            message: format!(
                "Pattern optimization requires {} tokens but vocabulary is limited to {}. Using multi-token encoding.",
                pattern_values, max_vocabulary
            ),
            suggestion: format!(
                "Consider increasing vocabulary size to {} for optimal pattern encoding",
                pattern_values
            ),
            example: Some(
                r#"
TctCompiler::new(schema, "MyType")
    .with_max_vocabulary(50)  // Increase from default
    .with_optimization_level(OptimizationLevel::O2)
    .compile()
"#
                .to_string(),
            ),
            impact: OptimizationImpact {
                token_reduction: Some(0.1), // Reduced benefit due to multi-token encoding
                compilation_impact: CompilationImpact::Neutral,
                type_safety: TypeSafetyImpact::None,
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::optimization_pass::OptimizationLevel;

    #[test]
    fn test_warnings_collection() {
        let mut warnings = SchemaWarnings::new();
        assert!(warnings.is_empty());

        let warning = SchemaWarning::unconstrained_additional_properties(
            "/properties/config".to_string(),
            OptimizationLevel::O0,
        );
        warnings.add(warning);

        assert!(!warnings.is_empty());
        assert_eq!(warnings.len(), 1);
    }

    #[test]
    fn test_warning_severity_filtering() {
        let mut warnings = SchemaWarnings::new();

        warnings.add(SchemaWarning::unconstrained_additional_properties(
            "/properties/config".to_string(),
            OptimizationLevel::O0,
        ));

        warnings.add(SchemaWarning::large_enum_vocabulary(
            "/properties/status".to_string(),
            150,
        ));

        let warning_level = warnings.by_severity(WarningSeverity::Warning);
        let important_level = warnings.by_severity(WarningSeverity::Important);

        assert_eq!(warning_level.len(), 1);
        assert_eq!(important_level.len(), 1);
    }

    #[test]
    fn test_warnings_summary() {
        let mut warnings = SchemaWarnings::new();

        warnings.add(SchemaWarning::unconstrained_additional_properties(
            "/properties/config".to_string(),
            OptimizationLevel::O2,
        ));

        let summary = warnings.summary();
        assert!(summary.contains("schema quality issue"));
        assert!(summary.contains("additionalProperties"));
        assert!(summary.contains("💡"));
    }

    #[test]
    fn test_empty_warnings_summary() {
        let warnings = SchemaWarnings::new();
        let summary = warnings.summary();
        assert!(summary.contains("No schema quality issues found"));
    }
}
