//! Tests for JavaScript to Rust regex pattern conversion
//!
//! These tests verify that the regex conversion function correctly handles
//! JavaScript/ECMAScript patterns that are incompatible with Rust's regex crate.

use tct::pattern_analyzer::{PatternAnalysis, PatternAnalyzer};

#[test]
fn test_non_greedy_quantifier_conversion() {
    let analyzer = PatternAnalyzer::new();

    // Test basic non-greedy conversions
    assert_eq!(
        analyzer
            .convert_js_regex_to_rust("^\\${{\\s*if(.*?)\\s*}}$")
            .unwrap(),
        "^\\$\\{\\{\\s*if(.*)\\s*\\}\\}$"
    );

    assert_eq!(analyzer.convert_js_regex_to_rust("a.*?b").unwrap(), "a.*b");

    assert_eq!(analyzer.convert_js_regex_to_rust("a.+?b").unwrap(), "a.+b");

    assert_eq!(analyzer.convert_js_regex_to_rust("a.??b").unwrap(), "a.?b");
}

#[test]
fn test_literal_brace_escaping() {
    let analyzer = PatternAnalyzer::new();

    // Test literal double braces
    assert_eq!(
        analyzer.convert_js_regex_to_rust("{{text}}").unwrap(),
        "\\{\\{text\\}\\}"
    );

    // Test already escaped braces are preserved
    assert_eq!(
        analyzer.convert_js_regex_to_rust("\\{text\\}").unwrap(),
        "\\{text\\}"
    );

    // Test mixed escaped and unescaped braces
    assert_eq!(
        analyzer
            .convert_js_regex_to_rust("\\{{{text}}}\\}")
            .unwrap(),
        "\\{\\{\\{text\\}\\}}\\}"
    );
}

#[test]
fn test_quantifier_range_conversion() {
    let analyzer = PatternAnalyzer::new();

    // Test lazy quantifier ranges
    assert_eq!(
        analyzer.convert_js_regex_to_rust("a{2,4}?b").unwrap(),
        "a{2,4}b"
    );

    assert_eq!(
        analyzer
            .convert_js_regex_to_rust("a{1,3}?b{2,5}?c")
            .unwrap(),
        "a{1,3}b{2,5}c"
    );
}

#[test]
fn test_complex_pattern_conversion() {
    let analyzer = PatternAnalyzer::new();

    // Test the actual problematic pattern from architect.yml
    let original = "^\\${{\\s*if(.*?)\\s*}}$";
    let expected = "^\\$\\{\\{\\s*if(.*)\\s*\\}\\}$";
    assert_eq!(
        analyzer.convert_js_regex_to_rust(original).unwrap(),
        expected
    );

    // Verify the converted pattern is valid Rust regex
    assert!(regex::Regex::new(expected).is_ok());
}

#[test]
fn test_invalid_patterns_remain_invalid() {
    let analyzer = PatternAnalyzer::new();

    // Test that truly invalid patterns still fail
    assert!(analyzer.convert_js_regex_to_rust("[").is_err());
    assert!(analyzer.convert_js_regex_to_rust("(").is_err());
    assert!(analyzer.convert_js_regex_to_rust("*").is_err());
}

#[test]
fn test_pattern_detection_with_conversion() {
    let analyzer = PatternAnalyzer::new();

    // Test that finite enumeration detection works with converted patterns
    let pattern = "^(red|green|blue)$";
    let result = analyzer.detect_finite_enumeration(pattern);
    assert!(result.is_ok());

    let enumeration = result.unwrap();
    assert_eq!(enumeration.len(), 3);
    assert!(enumeration.contains(&"red".to_string()));
    assert!(enumeration.contains(&"green".to_string()));
    assert!(enumeration.contains(&"blue".to_string()));
}
