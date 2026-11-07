//! Pattern Analysis for Regex-Based Finite Set Detection
//!
//! This module analyzes regex patterns to identify finite sets and bounded
//! ranges that can be optimized with dense token encoding.
//!
//! Includes JavaScript/ECMAScript to Rust regex conversion for compatibility
//! with JSON Schema pattern constraints.

use std::error::Error;
use std::fmt;

/// Trait for pattern analysis strategies to reduce coupling
pub trait PatternAnalysis {
    /// Detect if a regex pattern represents a finite enumeration
    fn detect_finite_enumeration(&self, pattern: &str) -> Result<Vec<String>, PatternError>;

    /// Detect bounded patterns that generate a finite set of values
    fn detect_bounded_pattern(&self, pattern: &str) -> Result<BoundedRange, PatternError>;
}

/// Analyzes regex patterns to detect optimization opportunities
pub struct PatternAnalyzer {
    _private: (), // Prevent direct construction during stub phase
}

impl Default for PatternAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

impl PatternAnalyzer {
    /// Create a new PatternAnalyzer instance
    pub fn new() -> Self {
        PatternAnalyzer { _private: () }
    }

    /// Convert JavaScript/ECMAScript regex patterns to Rust-compatible patterns
    ///
    /// This function handles common incompatibilities between JavaScript regex
    /// and Rust's regex crate, particularly non-greedy quantifiers and literal braces.
    pub fn convert_js_regex_to_rust(&self, pattern: &str) -> Result<String, PatternError> {
        let mut converted = pattern.to_string();

        // Convert non-greedy quantifiers to greedy equivalents
        // JavaScript: (.*?) -> Rust: (.*)
        // This is a lossy conversion but maintains functional equivalence for most cases
        converted = converted.replace("*?", "*");
        converted = converted.replace("+?", "+");
        converted = converted.replace("??", "?");

        // Handle lazy quantifier ranges: {n,m}? -> {n,m}
        converted = regex::Regex::new(r"\{(\d+),(\d+)\}\?")
            .expect("hardcoded regex pattern should be valid")
            .replace_all(&converted, "{$1,$2}")
            .to_string();

        // Handle lookahead and lookbehind assertions (not supported in Rust regex)
        // These need to be converted to equivalent patterns where possible
        converted = self.convert_lookaround_patterns(&converted)?;

        // Fix literal braces that should be escaped
        // In JavaScript, {{ is often used for literal braces, but in Rust regex they need escaping
        // First, temporarily replace already escaped braces to avoid double-escaping
        converted = converted.replace("\\{", "<!ESCAPED_OPEN_BRACE!>");
        converted = converted.replace("\\}", "<!ESCAPED_CLOSE_BRACE!>");

        // Now escape unescaped braces that appear to be literal (like {{)
        converted = converted.replace("{{", "\\{\\{");
        converted = converted.replace("}}", "\\}\\}");

        // Restore the originally escaped braces
        converted = converted.replace("<!ESCAPED_OPEN_BRACE!>", "\\{");
        converted = converted.replace("<!ESCAPED_CLOSE_BRACE!>", "\\}");

        // Test that the converted pattern is valid in Rust
        regex::Regex::new(&converted).map_err(|e| {
            PatternError::InvalidRegex(format!(
                "Pattern '{}' cannot be converted to valid Rust regex: {}",
                pattern, e
            ))
        })?;

        Ok(converted)
    }

    /// Convert lookahead and lookbehind patterns to Rust-compatible equivalents
    ///
    /// JavaScript regex supports lookahead (?=...) and lookbehind (?<=...) assertions
    /// that are not supported in Rust's regex crate. This function attempts to convert
    /// common patterns to functionally equivalent forms.
    fn convert_lookaround_patterns(&self, pattern: &str) -> Result<String, PatternError> {
        let mut converted = pattern.to_string();

        // Handle negative lookahead patterns
        // JavaScript negative lookahead (?!x) means "not followed by x"

        // Pattern 1: ^@(?!@).+$ means "starts with @, not followed by another @, then any chars"
        // Convert to: ^@[^@].* (starts with @, followed by non-@, then any chars)
        if converted.contains("@(?!@)") {
            converted = converted.replace("@(?!@).+", "@[^@].*");
        }
        // Pattern 2: ^(?!@@).+$ means "starts with anything that's not @@, then any chars"
        // Convert to: ^[^@].*|^@[^@].* (either doesn't start with @ or starts with @ but not @@)
        else if converted.contains("^(?!@@)") {
            converted = converted.replace("^(?!@@).+", "^([^@].*|@[^@].*)");
        }
        // General case for simple character negative lookahead
        else if let Some(caps) = regex::Regex::new(r"\(\?\!([^)]+)\)")
            .expect("hardcoded regex pattern should be valid")
            .captures(&converted)
        {
            let negative_pattern = &caps[1];

            // For simple character negation like (?!x), convert to [^x]
            if negative_pattern.len() == 1 {
                let char_to_exclude = negative_pattern
                    .chars()
                    .next()
                    .expect("pattern length is 1, so first char exists");
                converted = converted.replace(
                    &format!("(?!{})", negative_pattern),
                    &format!("[^{}]", char_to_exclude),
                );
                // Adjust following .+ to .* since we consumed one character
                converted = converted.replace(
                    &format!("[^{}].+", char_to_exclude),
                    &format!("[^{}].*", char_to_exclude),
                );
            } else {
                // For complex patterns, provide specific guidance
                return Err(PatternError::UnsupportedPattern(format!(
                    "Complex negative lookahead '(?!{})' cannot be automatically converted. Try using character classes like [^{}] instead.",
                    negative_pattern,
                    if negative_pattern.len() <= 3 {
                        negative_pattern
                    } else {
                        "chars"
                    }
                )));
            }
        }

        // Handle positive lookahead: (?=x) - "followed by x"
        // This is more complex to convert, so we'll provide an error with guidance
        if converted.contains("(?=") {
            return Err(PatternError::UnsupportedPattern(
                "Positive lookahead (?=...) is not supported in Rust regex. Consider restructuring the pattern.".to_string()
            ));
        }

        // Handle lookbehind: (?<=x) and (?<!x) - "preceded by x" or "not preceded by x"
        if converted.contains("(?<=") || converted.contains("(?<!") {
            return Err(PatternError::UnsupportedPattern(
                "Lookbehind assertions (?<=...) and (?<!...) are not supported in Rust regex."
                    .to_string(),
            ));
        }

        Ok(converted)
    }
}

impl PatternAnalysis for PatternAnalyzer {
    /// Detect if a regex pattern represents a finite enumeration
    ///
    /// Returns Ok(Vec<String>) if the pattern can be converted to a finite
    /// set of possible values, Err otherwise.
    ///
    /// Example: "^(red|green|blue)$" -> Ok(["red", "green", "blue"])
    fn detect_finite_enumeration(&self, pattern: &str) -> Result<Vec<String>, PatternError> {
        // Convert JavaScript regex to Rust-compatible pattern first
        let converted_pattern = self.convert_js_regex_to_rust(pattern)?;

        // Validate and compile the converted regex
        let regex = regex::Regex::new(&converted_pattern).map_err(|e| {
            PatternError::InvalidRegex(format!(
                "Invalid regex '{}' (converted from '{}'): {}",
                converted_pattern, pattern, e
            ))
        })?;

        // Check if pattern matches the basic alternation structure: ^(option1|option2|...)$
        if let Some(enumeration) = self.parse_simple_alternation(pattern)? {
            // Verify that all extracted alternatives actually match the regex
            for value in &enumeration {
                if !regex.is_match(value) {
                    return Err(PatternError::TooComplex(
                        "Extracted alternatives don't match original pattern".to_string(),
                    ));
                }
            }
            return Ok(enumeration);
        }

        // Pattern doesn't represent a finite enumeration
        Err(PatternError::InfiniteSet(
            "Pattern does not represent a finite enumeration".to_string(),
        ))
    }

    /// Detect if a regex pattern represents a bounded finite range
    ///
    /// Returns Ok(BoundedRange) if the pattern represents a finite set
    /// with a calculable size (e.g., \d{4} = 10,000 possibilities).
    ///
    /// Example: "^\d{4}$" -> Ok(BoundedRange { min: 0, max: 9999, total: 10000 })
    fn detect_bounded_pattern(&self, pattern: &str) -> Result<BoundedRange, PatternError> {
        // Convert JavaScript regex to Rust-compatible pattern first
        let converted_pattern = self.convert_js_regex_to_rust(pattern)?;

        // Validate and compile the converted regex
        let _regex = regex::Regex::new(&converted_pattern).map_err(|e| {
            PatternError::InvalidRegex(format!(
                "Invalid regex '{}' (converted from '{}'): {}",
                converted_pattern, pattern, e
            ))
        })?;

        // Remove anchors if present
        let inner = pattern.strip_prefix('^').unwrap_or(pattern);
        let inner = inner.strip_suffix('$').unwrap_or(inner);

        // Try to parse as character class with repetition: \d{n} or [char-class]{n}
        if let Some((char_class, repetition)) = self.parse_character_class_repetition(inner)? {
            let charset_size = self.calculate_character_class_size(&char_class)?;

            let total_values = charset_size.pow(repetition as u32);
            let max_value = total_values.saturating_sub(1);

            return Ok(BoundedRange {
                min_value: 0,
                max_value: max_value as u64,
                total_values,
                character_class: char_class,
                repetition,
            });
        }

        // Pattern doesn't represent a bounded range
        Err(PatternError::InfiniteSet(
            "Pattern does not represent a bounded range".to_string(),
        ))
    }
}

impl PatternAnalyzer {
    /// Parse simple alternation patterns like ^(a|b|c)$
    fn parse_simple_alternation(&self, pattern: &str) -> Result<Option<Vec<String>>, PatternError> {
        // Remove anchors if present
        let inner = pattern.strip_prefix('^').unwrap_or(pattern);
        let inner = inner.strip_suffix('$').unwrap_or(inner);

        // Check if it's a parenthesized alternation: (a|b|c)
        if let Some(stripped) = inner.strip_prefix('(').and_then(|s| s.strip_suffix(')')) {
            return self.parse_alternation_group(stripped);
        }

        // Not a simple alternation pattern
        Ok(None)
    }

    /// Parse the content inside parentheses for alternation
    fn parse_alternation_group(
        &self,
        group_content: &str,
    ) -> Result<Option<Vec<String>>, PatternError> {
        // Smart split that respects escaped | characters
        let raw_alternatives = self.split_alternatives(group_content)?;

        let alternatives: Vec<String> = raw_alternatives
            .into_iter()
            .map(|alt| self.unescape_alternative(&alt))
            .collect::<Result<Vec<_>, _>>()?;

        // Ensure we have at least one alternative and no empty alternatives
        if alternatives.is_empty() || alternatives.iter().any(|alt| alt.is_empty()) {
            return Ok(None);
        }

        // Check that alternatives are simple strings (no complex regex features)
        for alternative in &alternatives {
            if self.contains_complex_regex_features(alternative) {
                return Ok(None);
            }
        }

        Ok(Some(alternatives))
    }

    /// Split alternation content while respecting escaped | characters
    fn split_alternatives(&self, content: &str) -> Result<Vec<String>, PatternError> {
        let mut alternatives = Vec::new();
        let mut current_alt = String::new();
        let mut chars = content.chars().peekable();

        while let Some(ch) = chars.next() {
            if ch == '\\' {
                // Escape sequence - add both characters to current alternative
                current_alt.push(ch);
                if let Some(next_ch) = chars.next() {
                    current_alt.push(next_ch);
                }
            } else if ch == '|' {
                // Unescaped | means end of current alternative
                alternatives.push(current_alt.trim().to_string());
                current_alt.clear();
            } else {
                current_alt.push(ch);
            }
        }

        // Don't forget the last alternative
        if !current_alt.is_empty() || !alternatives.is_empty() {
            alternatives.push(current_alt.trim().to_string());
        }

        Ok(alternatives)
    }

    /// Unescape common regex escape sequences in alternatives
    fn unescape_alternative(&self, alternative: &str) -> Result<String, PatternError> {
        let mut result = String::new();
        let mut chars = alternative.chars().peekable();

        while let Some(ch) = chars.next() {
            if ch == '\\' {
                if let Some(&next_ch) = chars.peek() {
                    match next_ch {
                        '|' => {
                            result.push('|');
                            chars.next(); // consume the escaped character
                        }
                        '-' => {
                            result.push('-');
                            chars.next();
                        }
                        '.' => {
                            result.push('.');
                            chars.next();
                        }
                        '\\' => {
                            result.push('\\');
                            chars.next();
                        }
                        '(' | ')' => {
                            result.push(next_ch);
                            chars.next();
                        }
                        // Add more escape sequences as needed
                        _ => {
                            // Unknown escape, keep both characters
                            result.push(ch);
                        }
                    }
                } else {
                    // Trailing backslash
                    result.push(ch);
                }
            } else {
                result.push(ch);
            }
        }

        Ok(result)
    }

    /// Check if alternative contains complex regex features that prevent finite enumeration
    fn contains_complex_regex_features(&self, alternative: &str) -> bool {
        // After unescaping, we should only reject patterns with actual regex metacharacters
        // that would make the pattern non-finite. Literal characters like '.' are fine after unescaping.
        let complex_chars = ['*', '+', '?', '{', '}', '[', ']', '^', '$'];

        alternative.chars().any(|ch| complex_chars.contains(&ch))
    }

    /// Parse character class with repetition like \d{4} or [A-Z]{2}
    fn parse_character_class_repetition(
        &self,
        pattern: &str,
    ) -> Result<Option<(String, usize)>, PatternError> {
        // Pattern: \d{n}
        if let Some(captures) = regex::Regex::new(r"^\\d\{(\d+)\}$")
            .expect("Hardcoded regex should always compile")
            .captures(pattern)
        {
            let repetition = captures[1]
                .parse::<usize>()
                .map_err(|_| PatternError::TooComplex("Invalid repetition count".to_string()))?;
            return Ok(Some((r"\d".to_string(), repetition)));
        }

        // Pattern: [char-class]{n}
        if let Some(captures) = regex::Regex::new(r"^(\[.*?\])\{(\d+)\}$")
            .expect("Hardcoded regex should always compile")
            .captures(pattern)
        {
            let char_class = captures[1].to_string();
            let repetition = captures[2]
                .parse::<usize>()
                .map_err(|_| PatternError::TooComplex("Invalid repetition count".to_string()))?;
            return Ok(Some((char_class, repetition)));
        }

        Ok(None)
    }

    /// Calculate the size of a character class
    fn calculate_character_class_size(&self, char_class: &str) -> Result<usize, PatternError> {
        match char_class {
            r"\d" => Ok(10),         // 0-9
            "[A-Z]" => Ok(26),       // A-Z
            "[a-z]" => Ok(26),       // a-z
            "[A-Za-z]" => Ok(52),    // A-Z + a-z
            "[0-9]" => Ok(10),       // 0-9 (same as \d)
            "[a-zA-Z0-9]" => Ok(62), // a-z + A-Z + 0-9
            _ => {
                // For more complex character classes, try to parse them
                self.parse_complex_character_class(char_class)
            }
        }
    }

    /// Parse complex character classes like [a-zA-Z0-9] manually
    fn parse_complex_character_class(&self, char_class: &str) -> Result<usize, PatternError> {
        if !char_class.starts_with('[') || !char_class.ends_with(']') {
            return Err(PatternError::TooComplex(format!(
                "Invalid character class format: {}",
                char_class
            )));
        }

        let inner = &char_class[1..char_class.len() - 1];
        let mut size = 0;

        // Simple parsing for common ranges
        let mut chars = inner.chars().peekable();
        while let Some(ch) = chars.next() {
            if chars.peek() == Some(&'-') && chars.nth(1).is_some() {
                // Range like a-z, A-Z, 0-9
                chars.next(); // consume the '-'
                if let Some(end_ch) = chars.next() {
                    match (ch, end_ch) {
                        ('a', 'z') => size += 26,
                        ('A', 'Z') => size += 26,
                        ('0', '9') => size += 10,
                        _ => {
                            return Err(PatternError::TooComplex(format!(
                                "Unsupported character range: {}-{}",
                                ch, end_ch
                            )));
                        }
                    }
                }
            } else {
                // Individual character
                size += 1;
            }
        }

        if size == 0 {
            return Err(PatternError::TooComplex(
                "Could not determine character class size".to_string(),
            ));
        }

        Ok(size)
    }
}

/// Represents a bounded range detected from a regex pattern
#[derive(Debug, Clone, PartialEq)]
pub struct BoundedRange {
    /// Minimum value in the range
    pub min_value: u64,
    /// Maximum value in the range  
    pub max_value: u64,
    /// Total number of possible values
    pub total_values: usize,
    /// The character class pattern (e.g., "\d", "[A-Z]")
    pub character_class: String,
    /// The repetition count (e.g., 4 for \d{4})
    pub repetition: usize,
}

/// Domain information for pattern-based optimization
#[derive(Debug, Clone)]
pub struct PatternDomain {
    /// Field path in the schema
    pub field_path: String,
    /// Type of pattern domain
    pub pattern_type: PatternDomainType,
}

/// Type of pattern domain detected
#[derive(Debug, Clone)]
pub enum PatternDomainType {
    /// Finite enumeration from alternation pattern
    Enumeration(Vec<String>),
    /// Bounded range from character class pattern
    Bounded(BoundedRange),
}

/// Errors that can occur during pattern analysis
#[derive(Debug, Clone)]
pub enum PatternError {
    /// Invalid regex syntax
    InvalidRegex(String),
    /// Pattern represents an infinite set
    InfiniteSet(String),
    /// Pattern is too complex to analyze
    TooComplex(String),
    /// Pattern uses unsupported features (e.g., lookahead/lookbehind)
    UnsupportedPattern(String),
    /// Feature not implemented for this pattern type
    NotImplemented(String),
}

impl fmt::Display for PatternError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PatternError::InvalidRegex(msg) => write!(f, "Invalid regex: {}", msg),
            PatternError::InfiniteSet(msg) => write!(f, "Infinite set: {}", msg),
            PatternError::TooComplex(msg) => write!(f, "Too complex: {}", msg),
            PatternError::UnsupportedPattern(msg) => write!(f, "Unsupported pattern: {}", msg),
            PatternError::NotImplemented(msg) => write!(f, "Not implemented: {}", msg),
        }
    }
}

impl Error for PatternError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_analyzer_creation() {
        let analyzer = PatternAnalyzer::new();
        // Should be able to create analyzer successfully
        // Verify that the analyzer is properly initialized
        assert!(analyzer.detect_finite_enumeration("^(a|b)$").is_ok());
    }

    #[test]
    fn test_simple_alternation_detection() {
        let analyzer = PatternAnalyzer::new();

        // Test simple pattern
        let result = analyzer.detect_finite_enumeration("^(red|green|blue)$");
        println!("Simple pattern result: {:?}", result);
        assert!(result.is_ok());
        let values = result.unwrap();
        assert_eq!(values.len(), 3);
        assert!(values.contains(&"red".to_string()));
        assert!(values.contains(&"green".to_string()));
        assert!(values.contains(&"blue".to_string()));
    }

    #[test]
    fn test_complex_alternation_with_escapes() {
        let analyzer = PatternAnalyzer::new();

        // Test pattern with escapes
        let result = analyzer.detect_finite_enumeration(r"^(hello\|world|foo\-bar|baz\.qux)$");
        println!("Complex pattern result: {:?}", result);
        assert!(result.is_ok());
        let values = result.unwrap();
        assert_eq!(values.len(), 3);
        assert!(values.contains(&"hello|world".to_string()));
        assert!(values.contains(&"foo-bar".to_string()));
        assert!(values.contains(&"baz.qux".to_string()));
    }

    #[test]
    fn test_bounded_pattern_detection() {
        let analyzer = PatternAnalyzer::new();

        // Test successful bounded pattern detection
        let bounded_result = analyzer.detect_bounded_pattern(r"^\d{4}$");
        assert!(bounded_result.is_ok());
        let bounded = bounded_result.unwrap();
        assert_eq!(bounded.total_values, 10000);
        assert_eq!(bounded.character_class, r"\d");
        assert_eq!(bounded.repetition, 4);

        // Test rejection of unbounded patterns
        let unbounded_result = analyzer.detect_bounded_pattern(r"^\d+$");
        assert!(unbounded_result.is_err());
    }

    // ========== PatternError Display Tests ==========

    #[test]
    fn test_pattern_error_invalid_regex_display() {
        let error = PatternError::InvalidRegex("unclosed bracket".to_string());
        assert_eq!(error.to_string(), "Invalid regex: unclosed bracket");
    }

    #[test]
    fn test_pattern_error_infinite_set_display() {
        let error = PatternError::InfiniteSet("unbounded quantifier".to_string());
        assert_eq!(error.to_string(), "Infinite set: unbounded quantifier");
    }

    #[test]
    fn test_pattern_error_too_complex_display() {
        let error = PatternError::TooComplex("nested quantifiers".to_string());
        assert_eq!(error.to_string(), "Too complex: nested quantifiers");
    }

    #[test]
    fn test_pattern_error_unsupported_pattern_display() {
        let error = PatternError::UnsupportedPattern("lookahead".to_string());
        assert_eq!(error.to_string(), "Unsupported pattern: lookahead");
    }

    #[test]
    fn test_pattern_error_not_implemented_display() {
        let error = PatternError::NotImplemented("backreferences".to_string());
        assert_eq!(error.to_string(), "Not implemented: backreferences");
    }

    // ========== Default Trait ==========

    #[test]
    fn test_pattern_analyzer_default() {
        let analyzer = PatternAnalyzer::default();
        let result = analyzer.detect_finite_enumeration("^(a|b)$");
        assert!(result.is_ok());
    }

    // ========== JS Regex Conversion Tests ==========

    #[test]
    fn test_convert_js_regex_non_greedy_quantifiers() {
        let analyzer = PatternAnalyzer::new();

        // Test *? conversion
        let result = analyzer.convert_js_regex_to_rust(".*?");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), ".*");

        // Test +? conversion
        let result = analyzer.convert_js_regex_to_rust(".+?");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), ".+");

        // Test ?? conversion
        let result = analyzer.convert_js_regex_to_rust(".??");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), ".?");
    }

    #[test]
    fn test_convert_js_regex_lazy_quantifier_ranges() {
        let analyzer = PatternAnalyzer::new();

        // Test {n,m}? conversion
        let result = analyzer.convert_js_regex_to_rust(r"a{2,5}?");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), r"a{2,5}");
    }

    #[test]
    fn test_convert_js_regex_literal_braces() {
        let analyzer = PatternAnalyzer::new();

        // Test {{ and }} conversion
        let result = analyzer.convert_js_regex_to_rust("{{literal}}");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), r"\{\{literal\}\}");
    }

    #[test]
    fn test_convert_js_regex_escaped_braces_preserved() {
        let analyzer = PatternAnalyzer::new();

        // Test that already escaped braces are preserved
        let result = analyzer.convert_js_regex_to_rust(r"\{escaped\}");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), r"\{escaped\}");
    }

    #[test]
    fn test_convert_js_regex_invalid_pattern_error() {
        let analyzer = PatternAnalyzer::new();

        // Test invalid regex after conversion
        let result = analyzer.convert_js_regex_to_rust("[unclosed");
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), PatternError::InvalidRegex(_)));
    }

    // ========== Lookaround Conversion Tests ==========

    #[test]
    fn test_convert_lookaround_negative_lookahead_at_pattern() {
        let analyzer = PatternAnalyzer::new();

        // Test @(?!@) conversion
        let result = analyzer.convert_js_regex_to_rust("@(?!@).+");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "@[^@].*");
    }

    #[test]
    fn test_convert_lookaround_negative_lookahead_double_at() {
        let analyzer = PatternAnalyzer::new();

        // Test ^(?!@@) conversion
        let result = analyzer.convert_js_regex_to_rust("^(?!@@).+");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "^([^@].*|@[^@].*)");
    }

    #[test]
    fn test_convert_lookaround_simple_character_negation() {
        let analyzer = PatternAnalyzer::new();

        // Test simple (?!x) conversion
        let result = analyzer.convert_js_regex_to_rust("(?!a).+");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "[^a].*");
    }

    #[test]
    fn test_convert_lookaround_complex_negative_lookahead_error() {
        let analyzer = PatternAnalyzer::new();

        // Test complex (?!pattern) that can't be converted
        let result = analyzer.convert_js_regex_to_rust("(?!abc).+");
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            PatternError::UnsupportedPattern(_)
        ));
    }

    #[test]
    fn test_convert_lookaround_positive_lookahead_error() {
        let analyzer = PatternAnalyzer::new();

        // Test (?=...) not supported
        let result = analyzer.convert_js_regex_to_rust("(?=test).+");
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            PatternError::UnsupportedPattern(_)
        ));
    }

    #[test]
    fn test_convert_lookaround_positive_lookbehind_error() {
        let analyzer = PatternAnalyzer::new();

        // Test (?<=...) not supported
        let result = analyzer.convert_js_regex_to_rust("(?<=test).+");
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            PatternError::UnsupportedPattern(_)
        ));
    }

    #[test]
    fn test_convert_lookaround_negative_lookbehind_error() {
        let analyzer = PatternAnalyzer::new();

        // Test (?<!...) not supported
        let result = analyzer.convert_js_regex_to_rust("(?<!test).+");
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            PatternError::UnsupportedPattern(_)
        ));
    }

    // ========== Finite Enumeration Tests ==========

    #[test]
    fn test_detect_finite_enumeration_without_anchors() {
        let analyzer = PatternAnalyzer::new();

        // Test pattern without ^ and $ anchors
        let result = analyzer.detect_finite_enumeration("(red|green|blue)");
        assert!(result.is_ok());
        let values = result.unwrap();
        assert_eq!(values.len(), 3);
    }

    #[test]
    fn test_detect_finite_enumeration_with_complex_regex_rejected() {
        let analyzer = PatternAnalyzer::new();

        // Test pattern with regex metacharacters (should be rejected)
        let result = analyzer.detect_finite_enumeration("^(a*|b+)$");
        assert!(result.is_err());
    }

    #[test]
    fn test_detect_finite_enumeration_invalid_pattern_error() {
        let analyzer = PatternAnalyzer::new();

        // Test invalid regex pattern
        let result = analyzer.detect_finite_enumeration("[unclosed");
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), PatternError::InvalidRegex(_)));
    }

    // ========== Bounded Pattern Tests ==========

    #[test]
    fn test_detect_bounded_pattern_character_class() {
        let analyzer = PatternAnalyzer::new();

        // Test [A-Z]{2}
        let result = analyzer.detect_bounded_pattern("^[A-Z]{2}$");
        assert!(result.is_ok());
        let bounded = result.unwrap();
        assert_eq!(bounded.total_values, 676); // 26^2
        assert_eq!(bounded.character_class, "[A-Z]");
        assert_eq!(bounded.repetition, 2);
    }

    #[test]
    fn test_detect_bounded_pattern_without_anchors() {
        let analyzer = PatternAnalyzer::new();

        // Test pattern without anchors
        let result = analyzer.detect_bounded_pattern(r"\d{3}");
        assert!(result.is_ok());
        let bounded = result.unwrap();
        assert_eq!(bounded.total_values, 1000); // 10^3
    }

    #[test]
    fn test_detect_bounded_pattern_invalid_pattern_error() {
        let analyzer = PatternAnalyzer::new();

        // Test invalid regex pattern
        let result = analyzer.detect_bounded_pattern("[unclosed{2}");
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), PatternError::InvalidRegex(_)));
    }

    // ========== parse_simple_alternation Tests ==========

    #[test]
    fn test_parse_simple_alternation_no_parentheses() {
        let analyzer = PatternAnalyzer::new();

        // Test pattern without parentheses
        let result = analyzer.parse_simple_alternation("a|b|c");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), None);
    }

    // ========== parse_alternation_group Tests ==========

    #[test]
    fn test_parse_alternation_group_empty_alternative() {
        let analyzer = PatternAnalyzer::new();

        // Test pattern with empty alternative (should be rejected)
        let result = analyzer.parse_alternation_group("a||b");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), None);
    }

    #[test]
    fn test_parse_alternation_group_with_complex_regex() {
        let analyzer = PatternAnalyzer::new();

        // Test pattern with regex metacharacters
        let result = analyzer.parse_alternation_group("a*|b+");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), None);
    }

    // ========== split_alternatives Tests ==========

    #[test]
    fn test_split_alternatives_escaped_pipe() {
        let analyzer = PatternAnalyzer::new();

        // Test pattern with escaped pipe
        let result = analyzer.split_alternatives(r"a\|b|c");
        assert!(result.is_ok());
        let alternatives = result.unwrap();
        assert_eq!(alternatives.len(), 2);
        assert_eq!(alternatives[0], r"a\|b");
        assert_eq!(alternatives[1], "c");
    }

    #[test]
    fn test_split_alternatives_trailing_pipe() {
        let analyzer = PatternAnalyzer::new();

        // Test pattern with trailing pipe
        let result = analyzer.split_alternatives("a|b|");
        assert!(result.is_ok());
        let alternatives = result.unwrap();
        assert_eq!(alternatives.len(), 3);
    }

    // ========== unescape_alternative Tests ==========

    #[test]
    fn test_unescape_alternative_escaped_backslash() {
        let analyzer = PatternAnalyzer::new();

        // Test escaped backslash
        let result = analyzer.unescape_alternative(r"\\");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), r"\");
    }

    #[test]
    fn test_unescape_alternative_escaped_parentheses() {
        let analyzer = PatternAnalyzer::new();

        // Test escaped parentheses
        let result = analyzer.unescape_alternative(r"\(\)");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "()");
    }

    #[test]
    fn test_unescape_alternative_unknown_escape() {
        let analyzer = PatternAnalyzer::new();

        // Test unknown escape sequence (keeps backslash)
        let result = analyzer.unescape_alternative(r"\x");
        assert!(result.is_ok());
        let unescaped = result.unwrap();
        assert!(unescaped.contains('\\'));
    }

    #[test]
    fn test_unescape_alternative_trailing_backslash() {
        let analyzer = PatternAnalyzer::new();

        // Test trailing backslash
        let result = analyzer.unescape_alternative(r"abc\");
        assert!(result.is_ok());
        let unescaped = result.unwrap();
        assert!(unescaped.ends_with('\\'));
    }

    // ========== contains_complex_regex_features Tests ==========

    #[test]
    fn test_contains_complex_regex_features_metacharacters() {
        let analyzer = PatternAnalyzer::new();

        let complex_patterns = vec!["a*", "b+", "c?", "d{2}", "[a-z]", "^start", "end$"];

        for pattern in complex_patterns {
            assert!(
                analyzer.contains_complex_regex_features(pattern),
                "Pattern '{}' should be complex",
                pattern
            );
        }
    }

    #[test]
    fn test_contains_complex_regex_features_simple_string() {
        let analyzer = PatternAnalyzer::new();

        let simple_patterns = vec!["hello", "world", "foo-bar", "baz_qux"];

        for pattern in simple_patterns {
            assert!(
                !analyzer.contains_complex_regex_features(pattern),
                "Pattern '{}' should be simple",
                pattern
            );
        }
    }

    // ========== parse_character_class_repetition Tests ==========

    #[test]
    fn test_parse_character_class_repetition_digit_class() {
        let analyzer = PatternAnalyzer::new();

        // Test \d{4}
        let result = analyzer.parse_character_class_repetition(r"\d{4}");
        assert!(result.is_ok());
        let parsed = result.unwrap();
        assert!(parsed.is_some());
        let (char_class, repetition) = parsed.unwrap();
        assert_eq!(char_class, r"\d");
        assert_eq!(repetition, 4);
    }

    #[test]
    fn test_parse_character_class_repetition_bracket_class() {
        let analyzer = PatternAnalyzer::new();

        // Test [A-Z]{3}
        let result = analyzer.parse_character_class_repetition("[A-Z]{3}");
        assert!(result.is_ok());
        let parsed = result.unwrap();
        assert!(parsed.is_some());
        let (char_class, repetition) = parsed.unwrap();
        assert_eq!(char_class, "[A-Z]");
        assert_eq!(repetition, 3);
    }

    #[test]
    fn test_parse_character_class_repetition_no_match() {
        let analyzer = PatternAnalyzer::new();

        // Test pattern that doesn't match
        let result = analyzer.parse_character_class_repetition(r"\d+");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), None);
    }

    #[test]
    fn test_parse_character_class_repetition_invalid_count() {
        let analyzer = PatternAnalyzer::new();

        // Test invalid repetition count (would overflow)
        let result = analyzer.parse_character_class_repetition(r"\d{999999999999999999999}");
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), PatternError::TooComplex(_)));
    }

    // ========== calculate_character_class_size Tests ==========

    #[test]
    fn test_calculate_character_class_size_predefined() {
        let analyzer = PatternAnalyzer::new();

        assert_eq!(analyzer.calculate_character_class_size(r"\d").unwrap(), 10);
        assert_eq!(
            analyzer.calculate_character_class_size("[A-Z]").unwrap(),
            26
        );
        assert_eq!(
            analyzer.calculate_character_class_size("[a-z]").unwrap(),
            26
        );
        assert_eq!(
            analyzer.calculate_character_class_size("[A-Za-z]").unwrap(),
            52
        );
        assert_eq!(
            analyzer.calculate_character_class_size("[0-9]").unwrap(),
            10
        );
        assert_eq!(
            analyzer
                .calculate_character_class_size("[a-zA-Z0-9]")
                .unwrap(),
            62
        );
    }

    // ========== parse_complex_character_class Tests ==========

    #[test]
    fn test_parse_complex_character_class_individual_chars() {
        let analyzer = PatternAnalyzer::new();

        // Test individual characters (no ranges)
        let result = analyzer.parse_complex_character_class("[XYZ]");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 3);
    }

    #[test]
    fn test_parse_complex_character_class_invalid_format() {
        let analyzer = PatternAnalyzer::new();

        // Missing brackets
        let result = analyzer.parse_complex_character_class("abc");
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), PatternError::TooComplex(_)));
    }

    #[test]
    fn test_parse_complex_character_class_unsupported_range() {
        let analyzer = PatternAnalyzer::new();

        // Unsupported range (Greek letters)
        let result = analyzer.parse_complex_character_class("[α-ω]");
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), PatternError::TooComplex(_)));
    }

    #[test]
    fn test_parse_complex_character_class_empty() {
        let analyzer = PatternAnalyzer::new();

        // Empty character class
        let result = analyzer.parse_complex_character_class("[]");
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), PatternError::TooComplex(_)));
    }

    #[test]
    fn test_calculate_character_class_size_calls_complex_parser() {
        let analyzer = PatternAnalyzer::new();

        // Test pattern that's not predefined, forcing complex parser
        let result = analyzer.calculate_character_class_size("[XYZ]");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 3);
    }
}
