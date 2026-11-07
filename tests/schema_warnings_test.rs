//! Tests for schema quality warnings

use serde_json::json;
use tct::{OptimizationLevel, TctCompiler};

#[test]
fn test_unconstrained_additional_properties_warning_o0() {
    let schema = json!({
        "type": "object",
        "properties": {
            "name": {"type": "string"}
        },
        "additionalProperties": true
    });

    let result = TctCompiler::new(schema, "TestSchema")
        .unwrap()
        .with_optimization_level(OptimizationLevel::O0)
        .compile();

    assert!(result.is_ok());
    let grammar = result.unwrap();

    // Should have warnings for unconstrained additionalProperties
    assert!(grammar.has_warnings());
    assert_eq!(grammar.warning_count(), 1);

    let warnings_summary = grammar.warnings_summary();
    assert!(warnings_summary.contains("additionalProperties"));
    assert!(warnings_summary.contains("losing type information"));
    assert!(warnings_summary.contains("explicit type constraints"));
}

#[test]
fn test_unconstrained_additional_properties_warning_o2() {
    let schema = json!({
        "type": "object",
        "properties": {
            "config": {"type": "string"}
        },
        "additionalProperties": true
    });

    let result = TctCompiler::new(schema, "ConfigSchema")
        .unwrap()
        .with_optimization_level(OptimizationLevel::O2)
        .compile();

    assert!(result.is_ok());
    let grammar = result.unwrap();

    // Should have warnings for unconstrained additionalProperties
    assert!(grammar.has_warnings());
    let warnings_summary = grammar.warnings_summary();
    assert!(warnings_summary.contains("explicit constraints"));
    assert!(warnings_summary.contains("better optimization"));
}

#[test]
fn test_no_warnings_for_constrained_additional_properties() {
    let schema = json!({
        "type": "object",
        "properties": {
            "name": {"type": "string"}
        },
        "additionalProperties": {
            "type": "string"
        }
    });

    let result = TctCompiler::new(schema, "ConstrainedSchema")
        .unwrap()
        .with_optimization_level(OptimizationLevel::O2)
        .compile();

    assert!(result.is_ok());
    let grammar = result.unwrap();

    // Should not have warnings for properly constrained additionalProperties
    assert!(!grammar.has_warnings());
}

#[test]
fn test_no_warnings_for_additional_properties_false() {
    let schema = json!({
        "type": "object",
        "properties": {
            "name": {"type": "string"}
        },
        "additionalProperties": false
    });

    let result = TctCompiler::new(schema, "StrictSchema")
        .unwrap()
        .with_optimization_level(OptimizationLevel::O2)
        .compile();

    assert!(result.is_ok());
    let grammar = result.unwrap();

    // Should not have warnings when additionalProperties is false
    assert!(!grammar.has_warnings());
}

#[test]
fn test_enum_constraint_no_unconstrained_warning() {
    let schema = json!({
        "type": "object",
        "properties": {
            "status": {"type": "string"}
        },
        "additionalProperties": {
            "enum": ["low", "medium", "high"]
        }
    });

    let result = TctCompiler::new(schema, "EnumSchema")
        .unwrap()
        .with_optimization_level(OptimizationLevel::O2)
        .compile();

    assert!(result.is_ok());
    let grammar = result.unwrap();

    // Should not have unconstrained additionalProperties warnings for enum constraints
    // The enum constraint provides specific value constraints
    assert!(!grammar.has_warnings());
}

#[test]
fn test_complex_object_constraint_no_unconstrained_warning() {
    let schema = json!({
        "type": "object",
        "properties": {
            "config": {"type": "string"}
        },
        "additionalProperties": {
            "type": "object",
            "properties": {
                "enabled": {"type": "boolean"},
                "value": {"type": "integer"}
            }
        }
    });

    let result = TctCompiler::new(schema, "ComplexSchema")
        .unwrap()
        .with_optimization_level(OptimizationLevel::O2)
        .compile();

    assert!(result.is_ok());
    let grammar = result.unwrap();

    // Should not have unconstrained warnings for explicit object constraints
    assert!(!grammar.has_warnings());
}

#[test]
fn test_warnings_summary_format() {
    let schema = json!({
        "type": "object",
        "properties": {
            "id": {"type": "string"}
        },
        "additionalProperties": true
    });

    let result = TctCompiler::new(schema, "SummaryTest")
        .unwrap()
        .with_optimization_level(OptimizationLevel::O0)
        .compile();

    assert!(result.is_ok());
    let grammar = result.unwrap();

    let summary = grammar.warnings_summary();

    // Check summary format contains expected elements
    assert!(summary.contains("schema quality issue"));
    assert!(summary.contains("🟡")); // Warning icon
    assert!(summary.contains("💡")); // Suggestion icon
    assert!(summary.contains("📈")); // Impact icon
    assert!(summary.contains("token reduction"));
}

#[test]
fn test_contextual_optimization_still_warns() {
    let schema = json!({
        "title": "User Config",
        "type": "object",
        "properties": {
            "username": {"type": "string"}
        },
        "additionalProperties": true
    });

    let result = TctCompiler::new(schema, "ContextualTest")
        .unwrap()
        .with_optimization_level(OptimizationLevel::O3)
        .compile();

    assert!(result.is_ok());
    let grammar = result.unwrap();

    // Even with contextual optimization (O3), should still warn about unconstrained properties
    assert!(grammar.has_warnings());
    let summary = grammar.warnings_summary();
    assert!(summary.contains("additionalProperties"));
    // The O3 message may differ from O0/O2, but should still warn
    assert!(summary.contains("constraint") || summary.contains("optimization"));
}
