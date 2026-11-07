//! Pattern analysis error types
//!
//! This module contains error types and context structures specifically
//! for regex pattern analysis and optimization errors.

use std::fmt;

/// Context for pattern analysis errors
#[derive(Debug, Clone)]
pub struct PatternContext {
    /// The type of pattern that caused the error
    pub pattern_type: PatternType,
    /// Optional hint about pattern complexity
    pub complexity_hint: Option<String>,
    /// List of supported pattern features
    pub supported_features: Vec<String>,
}

/// Types of regex patterns that can cause analysis errors
#[derive(Debug, Clone)]
pub enum PatternType {
    /// Simple alternation pattern like (a|b|c)
    SimpleAlternation,
    /// Character class pattern like [a-z]
    CharacterClass,
    /// Quantifier pattern like a{1,3}
    Quantifier,
    /// Complex regex that couldn't be classified
    ComplexRegex,
    /// Pattern type could not be determined
    Unknown,
}

impl fmt::Display for PatternType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PatternType::SimpleAlternation => write!(f, "simple alternation (a|b|c)"),
            PatternType::CharacterClass => write!(f, "character class ([a-z])"),
            PatternType::Quantifier => write!(f, "quantifier (*, +, ?, {{}})"),
            PatternType::ComplexRegex => write!(f, "complex regex"),
            PatternType::Unknown => write!(f, "unknown pattern type"),
        }
    }
}
