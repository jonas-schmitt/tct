//! Additional Properties Constraint Analysis
//!
//! This module analyzes additionalProperties schemas to determine the most efficient
//! type representation, avoiding generic serde_json::Value when more specific types
//! can be inferred from schema context and constraints.

use serde_json::{Map, Value};
use std::collections::HashSet;

/// Simple enum for type-preserving additional properties (O2 optimization level)
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(untagged)]
pub enum AdditionalValue {
    /// String value
    String(String),
    /// Numeric value (f64)
    Number(f64),
    /// Boolean value
    Bool(bool),
    /// Null value
    Null,
}

/// Result of analyzing additionalProperties for optimal type generation
#[derive(Debug, Clone, PartialEq)]
pub enum AdditionalPropertiesType {
    /// No additional properties allowed
    None,
    /// String-only additional properties from explicit constraint (no warning)
    StringOnlyConstrained,
    /// String-only additional properties from unconstrained analysis (warning)
    StringOnlyUnconstrained,
    /// Type-preserving additional properties (O2: balanced)
    TaggedUnion,
    /// Specific typed additional properties with schema constraint
    Typed(TypedConstraint),
    /// Number-only additional properties from explicit constraint
    NumberOnlyConstrained,
    /// Integer-only additional properties from explicit constraint
    IntegerOnlyConstrained,
    /// Boolean-only additional properties from explicit constraint
    BooleanOnlyConstrained,
    /// Number-only additional properties from unconstrained analysis
    NumberOnlyUnconstrained,
    /// Boolean-only additional properties from unconstrained analysis
    BooleanOnlyUnconstrained,
    /// Contextual optimization based on heuristics (O3: aggressive)
    Contextual(Box<AdditionalPropertiesType>),
    /// Array of specific type
    ArrayOf(Box<AdditionalPropertiesType>),
    /// Mixed types but constrained set
    Union(Vec<AdditionalPropertiesType>),
    /// Truly arbitrary values (fallback to serde_json::Value)
    Arbitrary,
}

/// Constraint information for typed additional properties
#[derive(Debug, Clone, PartialEq)]
pub struct TypedConstraint {
    /// The specific schema constraint
    pub schema: Map<String, Value>,
    /// Inferred Rust type representation
    pub rust_type: String,
    /// Confidence level in this inference (0.0 to 1.0)
    pub confidence: f64,
}

/// Context information for analyzing additional properties
#[derive(Debug, Clone)]
pub struct AnalysisContext {
    /// Schema title or description that might hint at type
    pub schema_title: Option<String>,
    /// Schema description
    pub schema_description: Option<String>,
    /// Existing properties in the same object
    pub existing_properties: Vec<PropertyInfo>,
    /// Examples provided in the schema
    pub examples: Vec<Value>,
    /// Parent schema context
    pub parent_context: Option<String>,
}

/// Information about existing properties that might influence additional property types
#[derive(Debug, Clone)]
pub struct PropertyInfo {
    /// Property name
    pub name: String,
    /// JSON type of the property (e.g., "string", "number", "object")
    pub json_type: String,
    /// Whether the property has a pattern constraint
    pub has_pattern: bool,
    /// Whether the property is required
    pub is_required: bool,
}

/// Analyzer for determining optimal additional properties representation
pub struct AdditionalPropertiesAnalyzer;

impl AdditionalPropertiesAnalyzer {
    /// Analyze additionalProperties with optimization level strategy
    pub fn analyze(
        additional_props: &Value,
        context: &AnalysisContext,
        optimization_level: crate::optimization_pass::OptimizationLevel,
    ) -> AdditionalPropertiesType {
        match additional_props {
            Value::Bool(false) => AdditionalPropertiesType::None,
            Value::Bool(true) => {
                Self::analyze_unconstrained_with_optimization(context, optimization_level)
            }
            Value::Object(schema) => Self::analyze_constrained(schema, context),
            _ => AdditionalPropertiesType::Arbitrary, // Invalid schema
        }
    }

    /// Analyze unconstrained additionalProperties based on optimization level
    fn analyze_unconstrained_with_optimization(
        context: &AnalysisContext,
        optimization_level: crate::optimization_pass::OptimizationLevel,
    ) -> AdditionalPropertiesType {
        use crate::optimization_pass::OptimizationLevel;

        match optimization_level {
            OptimizationLevel::O0 => {
                // O0: Safe but lossy - always use strings (unconstrained, needs warning)
                AdditionalPropertiesType::StringOnlyUnconstrained
            }
            OptimizationLevel::O2 => {
                // O2: Balanced - use tagged union for type preservation (unconstrained, needs warning)
                AdditionalPropertiesType::TaggedUnion
            }
            OptimizationLevel::O3 => {
                // O3: Aggressive - use contextual analysis (existing smart logic)
                let contextual_result = Self::analyze_unconstrained(context);
                AdditionalPropertiesType::Contextual(Box::new(contextual_result))
            }
            OptimizationLevel::Os => {
                // Os: Size optimization - use most compact representation (same as O3)
                let contextual_result = Self::analyze_unconstrained(context);
                AdditionalPropertiesType::Contextual(Box::new(contextual_result))
            }
            OptimizationLevel::OVocab => {
                // OVocab: Vocabulary optimization - prefer types that compress well
                AdditionalPropertiesType::TaggedUnion
            }
        }
    }

    /// Analyze additionalProperties: true case with context hints
    fn analyze_unconstrained(context: &AnalysisContext) -> AdditionalPropertiesType {
        // Check for schema hints first
        if let Some(hint) = Self::detect_schema_hints(context) {
            return hint;
        }

        // Analyze existing properties for patterns
        if let Some(inferred) = Self::infer_from_existing_properties(context) {
            return inferred;
        }

        // Analyze examples if available
        if let Some(inferred) = Self::infer_from_examples(context) {
            return inferred;
        }

        // Check parent context for common patterns
        if let Some(inferred) = Self::infer_from_parent_context(context) {
            return inferred;
        }

        // Default to arbitrary when no constraints can be inferred
        AdditionalPropertiesType::Arbitrary
    }

    /// Analyze additionalProperties with explicit schema constraint
    fn analyze_constrained(
        schema: &Map<String, Value>,
        _context: &AnalysisContext,
    ) -> AdditionalPropertiesType {
        // Direct type constraint
        if let Some(type_val) = schema.get("type") {
            return Self::map_type_constraint(type_val, schema);
        }

        // Enum constraint (finite set of values)
        if let Some(enum_vals) = schema.get("enum") {
            return Self::analyze_enum_constraint(enum_vals);
        }

        // Pattern constraint for strings
        if let Some(pattern) = schema.get("pattern")
            && let Some(pattern_str) = pattern.as_str()
        {
            return Self::analyze_pattern_constraint(pattern_str, schema);
        }

        // allOf/anyOf/oneOf constraints
        if schema.contains_key("allOf")
            || schema.contains_key("anyOf")
            || schema.contains_key("oneOf")
        {
            return Self::analyze_composition_constraint(schema);
        }

        // Fallback to typed constraint with the full schema
        AdditionalPropertiesType::Typed(TypedConstraint {
            schema: schema.clone(),
            rust_type: "Value".to_string(), // Will be resolved during code generation
            confidence: 0.5,
        })
    }

    /// Detect hints from schema metadata
    fn detect_schema_hints(context: &AnalysisContext) -> Option<AdditionalPropertiesType> {
        // Check title for common patterns
        if let Some(title) = &context.schema_title {
            let title_lower = title.to_lowercase();

            if title_lower.contains("config") || title_lower.contains("setting") {
                return Some(AdditionalPropertiesType::StringOnlyUnconstrained);
            }

            if title_lower.contains("metric") || title_lower.contains("counter") {
                return Some(AdditionalPropertiesType::NumberOnlyUnconstrained);
            }

            if title_lower.contains("flag") || title_lower.contains("option") {
                return Some(AdditionalPropertiesType::BooleanOnlyUnconstrained);
            }
        }

        // Check description for type hints
        if let Some(desc) = &context.schema_description {
            let desc_lower = desc.to_lowercase();

            if desc_lower.contains("string values") || desc_lower.contains("text properties") {
                return Some(AdditionalPropertiesType::StringOnlyUnconstrained);
            }

            if desc_lower.contains("numeric") || desc_lower.contains("number") {
                return Some(AdditionalPropertiesType::NumberOnlyUnconstrained);
            }
        }

        None
    }

    /// Infer type from existing properties in the same object
    fn infer_from_existing_properties(
        context: &AnalysisContext,
    ) -> Option<AdditionalPropertiesType> {
        if context.existing_properties.is_empty() {
            return None;
        }

        let mut types = HashSet::new();
        for prop in &context.existing_properties {
            types.insert(prop.json_type.clone());
        }

        // If all existing properties are the same type, additional properties likely are too
        if types.len() == 1 {
            let uniform_type = types
                .iter()
                .next()
                .expect("types.len() == 1 guarantees one element exists");
            match uniform_type.as_str() {
                "string" => return Some(AdditionalPropertiesType::StringOnlyUnconstrained),
                "number" | "integer" => {
                    return Some(AdditionalPropertiesType::NumberOnlyUnconstrained);
                }
                "boolean" => return Some(AdditionalPropertiesType::BooleanOnlyUnconstrained),
                _ => {}
            }
        }

        // If most properties are strings (common in config files)
        let string_count = context
            .existing_properties
            .iter()
            .filter(|p| p.json_type == "string")
            .count();

        if string_count >= context.existing_properties.len() * 2 / 3 {
            return Some(AdditionalPropertiesType::StringOnlyUnconstrained);
        }

        None
    }

    /// Infer type from schema examples
    fn infer_from_examples(context: &AnalysisContext) -> Option<AdditionalPropertiesType> {
        if context.examples.is_empty() {
            return None;
        }

        let mut types = HashSet::new();
        for example in &context.examples {
            if let Value::Object(obj) = example {
                // Look for additional properties in examples
                for (_, value) in obj.iter() {
                    match value {
                        Value::String(_) => {
                            types.insert("string");
                        }
                        Value::Number(_) => {
                            types.insert("number");
                        }
                        Value::Bool(_) => {
                            types.insert("boolean");
                        }
                        Value::Array(_) => {
                            types.insert("array");
                        }
                        Value::Object(_) => {
                            types.insert("object");
                        }
                        Value::Null => {
                            types.insert("null");
                        }
                    }
                }
            }
        }

        // If examples show consistent typing
        if types.len() == 1 {
            match *types
                .iter()
                .next()
                .expect("types.len() == 1 guarantees one element exists")
            {
                "string" => return Some(AdditionalPropertiesType::StringOnlyUnconstrained),
                "number" => return Some(AdditionalPropertiesType::NumberOnlyUnconstrained),
                "boolean" => return Some(AdditionalPropertiesType::BooleanOnlyUnconstrained),
                _ => {}
            }
        }

        None
    }

    /// Infer type from parent schema context
    fn infer_from_parent_context(context: &AnalysisContext) -> Option<AdditionalPropertiesType> {
        if let Some(parent) = &context.parent_context {
            let parent_lower = parent.to_lowercase();

            // Common configuration file patterns
            if parent_lower.contains("package.json") || parent_lower.contains("dependencies") {
                return Some(AdditionalPropertiesType::StringOnlyUnconstrained);
            }

            if parent_lower.contains("scripts") {
                return Some(AdditionalPropertiesType::StringOnlyUnconstrained);
            }

            if parent_lower.contains("env") || parent_lower.contains("environment") {
                return Some(AdditionalPropertiesType::StringOnlyUnconstrained);
            }
        }

        None
    }

    /// Map explicit type constraint to AdditionalPropertiesType
    fn map_type_constraint(
        type_val: &Value,
        schema: &Map<String, Value>,
    ) -> AdditionalPropertiesType {
        match type_val {
            Value::String(type_str) => match type_str.as_str() {
                "string" => AdditionalPropertiesType::StringOnlyConstrained,
                "number" => AdditionalPropertiesType::NumberOnlyConstrained,
                "integer" => AdditionalPropertiesType::IntegerOnlyConstrained,
                "boolean" => AdditionalPropertiesType::BooleanOnlyConstrained,
                "array" => {
                    // Check for items constraint
                    if let Some(Value::Object(items_schema)) = schema.get("items") {
                        let item_type = Self::analyze_constrained(
                            items_schema,
                            &AnalysisContext {
                                schema_title: None,
                                schema_description: None,
                                existing_properties: vec![],
                                examples: vec![],
                                parent_context: None,
                            },
                        );
                        return AdditionalPropertiesType::ArrayOf(Box::new(item_type));
                    }
                    AdditionalPropertiesType::Arbitrary // Unknown array type
                }
                "object" => AdditionalPropertiesType::Typed(TypedConstraint {
                    schema: schema.clone(),
                    rust_type: "Object".to_string(),
                    confidence: 0.9,
                }),
                _ => AdditionalPropertiesType::Arbitrary,
            },
            Value::Array(types) => {
                // Union type - analyze each type
                let mut union_types = Vec::new();
                for type_val in types {
                    let constraint_type = Self::map_type_constraint(type_val, schema);
                    union_types.push(constraint_type);
                }
                AdditionalPropertiesType::Union(union_types)
            }
            _ => AdditionalPropertiesType::Arbitrary,
        }
    }

    /// Analyze enum constraint for finite value sets
    fn analyze_enum_constraint(enum_vals: &Value) -> AdditionalPropertiesType {
        if let Value::Array(values) = enum_vals {
            if values.is_empty() {
                return AdditionalPropertiesType::None;
            }

            // Check if all enum values are the same type
            let mut types = HashSet::new();
            for val in values {
                match val {
                    Value::String(_) => {
                        types.insert("string");
                    }
                    Value::Number(_) => {
                        types.insert("number");
                    }
                    Value::Bool(_) => {
                        types.insert("boolean");
                    }
                    _ => {
                        types.insert("mixed");
                    }
                }
            }

            // For enum constraints, always use Typed to preserve the specific enum values
            // This ensures that enums like ["low", "medium", "high"] generate proper enum types
            // instead of being generalized to StringOnly
            let schema = serde_json::Map::from_iter([("enum".to_string(), enum_vals.clone())]);
            return AdditionalPropertiesType::Typed(TypedConstraint {
                schema,
                rust_type: "EnumValue".to_string(), // Will be compiled to proper enum
                confidence: 0.95,
            });
        }

        AdditionalPropertiesType::Arbitrary
    }

    /// Analyze pattern constraint for string optimization
    fn analyze_pattern_constraint(
        _pattern: &str,
        schema: &Map<String, Value>,
    ) -> AdditionalPropertiesType {
        // For now, all pattern-constrained strings are treated as strings
        // Future enhancement: detect specific patterns (UUID, email, etc.)
        AdditionalPropertiesType::Typed(TypedConstraint {
            schema: schema.clone(),
            rust_type: "String".to_string(),
            confidence: 0.8,
        })
    }

    /// Analyze composition constraints (allOf/anyOf/oneOf)
    fn analyze_composition_constraint(schema: &Map<String, Value>) -> AdditionalPropertiesType {
        // For composition constraints, we need to analyze the composed schemas
        // This is complex and may require recursive analysis
        AdditionalPropertiesType::Typed(TypedConstraint {
            schema: schema.clone(),
            rust_type: "Value".to_string(), // Conservative fallback
            confidence: 0.3,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_analyze_additional_properties_false() {
        let context = AnalysisContext {
            schema_title: None,
            schema_description: None,
            existing_properties: vec![],
            examples: vec![],
            parent_context: None,
        };

        let result = AdditionalPropertiesAnalyzer::analyze(
            &json!(false),
            &context,
            crate::optimization_pass::OptimizationLevel::O2,
        );
        assert_eq!(result, AdditionalPropertiesType::None);
    }

    #[test]
    fn test_analyze_string_constraint() {
        let context = AnalysisContext {
            schema_title: None,
            schema_description: None,
            existing_properties: vec![],
            examples: vec![],
            parent_context: None,
        };

        let schema = json!({"type": "string"});
        let result = AdditionalPropertiesAnalyzer::analyze(
            &schema,
            &context,
            crate::optimization_pass::OptimizationLevel::O2,
        );
        assert_eq!(result, AdditionalPropertiesType::StringOnlyConstrained);
    }

    #[test]
    fn test_analyze_config_context() {
        let context = AnalysisContext {
            schema_title: Some("Application Config".to_string()),
            schema_description: None,
            existing_properties: vec![],
            examples: vec![],
            parent_context: None,
        };

        let result = AdditionalPropertiesAnalyzer::analyze(
            &json!(true),
            &context,
            crate::optimization_pass::OptimizationLevel::O3,
        );
        assert_eq!(
            result,
            AdditionalPropertiesType::Contextual(Box::new(
                AdditionalPropertiesType::StringOnlyUnconstrained
            ))
        );
    }

    #[test]
    fn test_analyze_uniform_existing_properties() {
        let context = AnalysisContext {
            schema_title: None,
            schema_description: None,
            existing_properties: vec![
                PropertyInfo {
                    name: "prop1".to_string(),
                    json_type: "string".to_string(),
                    has_pattern: false,
                    is_required: true,
                },
                PropertyInfo {
                    name: "prop2".to_string(),
                    json_type: "string".to_string(),
                    has_pattern: false,
                    is_required: false,
                },
            ],
            examples: vec![],
            parent_context: None,
        };

        let result = AdditionalPropertiesAnalyzer::analyze(
            &json!(true),
            &context,
            crate::optimization_pass::OptimizationLevel::O3,
        );
        assert_eq!(
            result,
            AdditionalPropertiesType::Contextual(Box::new(
                AdditionalPropertiesType::StringOnlyUnconstrained
            ))
        );
    }

    #[test]
    fn test_analyze_package_json_context() {
        let context = AnalysisContext {
            schema_title: None,
            schema_description: None,
            existing_properties: vec![],
            examples: vec![],
            parent_context: Some("package.json dependencies".to_string()),
        };

        let result = AdditionalPropertiesAnalyzer::analyze(
            &json!(true),
            &context,
            crate::optimization_pass::OptimizationLevel::O3,
        );
        assert_eq!(
            result,
            AdditionalPropertiesType::Contextual(Box::new(
                AdditionalPropertiesType::StringOnlyUnconstrained
            ))
        );
    }

    // === Phase 4.6: Additional Properties Analyzer Coverage Tests ===
    // Batch 1: Optimization Levels

    #[test]
    fn test_optimization_level_o0() {
        // O0: Safe but lossy - always uses strings
        let context = AnalysisContext {
            schema_title: None,
            schema_description: None,
            existing_properties: vec![],
            examples: vec![],
            parent_context: None,
        };

        let result = AdditionalPropertiesAnalyzer::analyze(
            &json!(true),
            &context,
            crate::optimization_pass::OptimizationLevel::O0,
        );
        assert_eq!(result, AdditionalPropertiesType::StringOnlyUnconstrained);
    }

    #[test]
    fn test_optimization_level_o2() {
        // O2: Balanced - uses tagged union for type preservation
        let context = AnalysisContext {
            schema_title: None,
            schema_description: None,
            existing_properties: vec![],
            examples: vec![],
            parent_context: None,
        };

        let result = AdditionalPropertiesAnalyzer::analyze(
            &json!(true),
            &context,
            crate::optimization_pass::OptimizationLevel::O2,
        );
        assert_eq!(result, AdditionalPropertiesType::TaggedUnion);
    }

    #[test]
    fn test_optimization_level_os() {
        // Os: Size optimization - uses contextual analysis
        let context = AnalysisContext {
            schema_title: Some("Config Settings".to_string()),
            schema_description: None,
            existing_properties: vec![],
            examples: vec![],
            parent_context: None,
        };

        let result = AdditionalPropertiesAnalyzer::analyze(
            &json!(true),
            &context,
            crate::optimization_pass::OptimizationLevel::Os,
        );
        assert_eq!(
            result,
            AdditionalPropertiesType::Contextual(Box::new(
                AdditionalPropertiesType::StringOnlyUnconstrained
            ))
        );
    }

    #[test]
    fn test_optimization_level_ovocab() {
        // OVocab: Vocabulary optimization - prefers types that compress well
        let context = AnalysisContext {
            schema_title: None,
            schema_description: None,
            existing_properties: vec![],
            examples: vec![],
            parent_context: None,
        };

        let result = AdditionalPropertiesAnalyzer::analyze(
            &json!(true),
            &context,
            crate::optimization_pass::OptimizationLevel::OVocab,
        );
        assert_eq!(result, AdditionalPropertiesType::TaggedUnion);
    }

    // Batch 2: Type Constraints

    #[test]
    fn test_number_type_constraint() {
        let context = AnalysisContext {
            schema_title: None,
            schema_description: None,
            existing_properties: vec![],
            examples: vec![],
            parent_context: None,
        };

        let schema = json!({"type": "number"});
        let result = AdditionalPropertiesAnalyzer::analyze(
            &schema,
            &context,
            crate::optimization_pass::OptimizationLevel::O2,
        );
        assert_eq!(result, AdditionalPropertiesType::NumberOnlyConstrained);
    }

    #[test]
    fn test_integer_type_constraint() {
        let context = AnalysisContext {
            schema_title: None,
            schema_description: None,
            existing_properties: vec![],
            examples: vec![],
            parent_context: None,
        };

        let schema = json!({"type": "integer"});
        let result = AdditionalPropertiesAnalyzer::analyze(
            &schema,
            &context,
            crate::optimization_pass::OptimizationLevel::O2,
        );
        assert_eq!(result, AdditionalPropertiesType::IntegerOnlyConstrained);
    }

    #[test]
    fn test_boolean_type_constraint() {
        let context = AnalysisContext {
            schema_title: None,
            schema_description: None,
            existing_properties: vec![],
            examples: vec![],
            parent_context: None,
        };

        let schema = json!({"type": "boolean"});
        let result = AdditionalPropertiesAnalyzer::analyze(
            &schema,
            &context,
            crate::optimization_pass::OptimizationLevel::O2,
        );
        assert_eq!(result, AdditionalPropertiesType::BooleanOnlyConstrained);
    }

    #[test]
    fn test_object_type_constraint() {
        let context = AnalysisContext {
            schema_title: None,
            schema_description: None,
            existing_properties: vec![],
            examples: vec![],
            parent_context: None,
        };

        let schema = json!({"type": "object"});
        let result = AdditionalPropertiesAnalyzer::analyze(
            &schema,
            &context,
            crate::optimization_pass::OptimizationLevel::O2,
        );

        match result {
            AdditionalPropertiesType::Typed(constraint) => {
                assert_eq!(constraint.rust_type, "Object");
                assert_eq!(constraint.confidence, 0.9);
            }
            _ => panic!("Expected Typed with Object rust_type"),
        }
    }

    #[test]
    fn test_array_type_constraint_with_items() {
        let context = AnalysisContext {
            schema_title: None,
            schema_description: None,
            existing_properties: vec![],
            examples: vec![],
            parent_context: None,
        };

        let schema = json!({"type": "array", "items": {"type": "string"}});
        let result = AdditionalPropertiesAnalyzer::analyze(
            &schema,
            &context,
            crate::optimization_pass::OptimizationLevel::O2,
        );

        match result {
            AdditionalPropertiesType::ArrayOf(boxed_type) => {
                assert_eq!(*boxed_type, AdditionalPropertiesType::StringOnlyConstrained);
            }
            _ => panic!("Expected ArrayOf(StringOnlyConstrained)"),
        }
    }

    #[test]
    fn test_array_type_constraint_without_items() {
        let context = AnalysisContext {
            schema_title: None,
            schema_description: None,
            existing_properties: vec![],
            examples: vec![],
            parent_context: None,
        };

        let schema = json!({"type": "array"});
        let result = AdditionalPropertiesAnalyzer::analyze(
            &schema,
            &context,
            crate::optimization_pass::OptimizationLevel::O2,
        );
        assert_eq!(result, AdditionalPropertiesType::Arbitrary);
    }

    #[test]
    fn test_union_type_constraint() {
        let context = AnalysisContext {
            schema_title: None,
            schema_description: None,
            existing_properties: vec![],
            examples: vec![],
            parent_context: None,
        };

        let schema = json!({"type": ["string", "number"]});
        let result = AdditionalPropertiesAnalyzer::analyze(
            &schema,
            &context,
            crate::optimization_pass::OptimizationLevel::O2,
        );

        match result {
            AdditionalPropertiesType::Union(types) => {
                assert_eq!(types.len(), 2);
                assert_eq!(types[0], AdditionalPropertiesType::StringOnlyConstrained);
                assert_eq!(types[1], AdditionalPropertiesType::NumberOnlyConstrained);
            }
            _ => panic!("Expected Union type"),
        }
    }

    // Batch 3: Enum, Pattern, and Composition Constraints

    #[test]
    fn test_enum_constraint_string_values() {
        let context = AnalysisContext {
            schema_title: None,
            schema_description: None,
            existing_properties: vec![],
            examples: vec![],
            parent_context: None,
        };

        let schema = json!({"enum": ["low", "medium", "high"]});
        let result = AdditionalPropertiesAnalyzer::analyze(
            &schema,
            &context,
            crate::optimization_pass::OptimizationLevel::O2,
        );

        match result {
            AdditionalPropertiesType::Typed(constraint) => {
                assert_eq!(constraint.rust_type, "EnumValue");
                assert_eq!(constraint.confidence, 0.95);
            }
            _ => panic!("Expected Typed with EnumValue"),
        }
    }

    #[test]
    fn test_enum_constraint_number_values() {
        let context = AnalysisContext {
            schema_title: None,
            schema_description: None,
            existing_properties: vec![],
            examples: vec![],
            parent_context: None,
        };

        let schema = json!({"enum": [1, 2, 3, 5, 8]});
        let result = AdditionalPropertiesAnalyzer::analyze(
            &schema,
            &context,
            crate::optimization_pass::OptimizationLevel::O2,
        );

        match result {
            AdditionalPropertiesType::Typed(constraint) => {
                assert_eq!(constraint.rust_type, "EnumValue");
                assert_eq!(constraint.confidence, 0.95);
            }
            _ => panic!("Expected Typed with EnumValue"),
        }
    }

    #[test]
    fn test_enum_constraint_empty() {
        let context = AnalysisContext {
            schema_title: None,
            schema_description: None,
            existing_properties: vec![],
            examples: vec![],
            parent_context: None,
        };

        let schema = json!({"enum": []});
        let result = AdditionalPropertiesAnalyzer::analyze(
            &schema,
            &context,
            crate::optimization_pass::OptimizationLevel::O2,
        );
        assert_eq!(result, AdditionalPropertiesType::None);
    }

    #[test]
    fn test_pattern_constraint() {
        let context = AnalysisContext {
            schema_title: None,
            schema_description: None,
            existing_properties: vec![],
            examples: vec![],
            parent_context: None,
        };

        let schema = json!({"pattern": "^[a-z]+$"});
        let result = AdditionalPropertiesAnalyzer::analyze(
            &schema,
            &context,
            crate::optimization_pass::OptimizationLevel::O2,
        );

        match result {
            AdditionalPropertiesType::Typed(constraint) => {
                assert_eq!(constraint.rust_type, "String");
                assert_eq!(constraint.confidence, 0.8);
            }
            _ => panic!("Expected Typed with String rust_type"),
        }
    }

    #[test]
    fn test_composition_allof_constraint() {
        let context = AnalysisContext {
            schema_title: None,
            schema_description: None,
            existing_properties: vec![],
            examples: vec![],
            parent_context: None,
        };

        let schema = json!({"allOf": [{"type": "string"}, {"minLength": 5}]});
        let result = AdditionalPropertiesAnalyzer::analyze(
            &schema,
            &context,
            crate::optimization_pass::OptimizationLevel::O2,
        );

        match result {
            AdditionalPropertiesType::Typed(constraint) => {
                assert_eq!(constraint.rust_type, "Value");
                assert_eq!(constraint.confidence, 0.3);
            }
            _ => panic!("Expected Typed with Value rust_type"),
        }
    }

    #[test]
    fn test_composition_anyof_constraint() {
        let context = AnalysisContext {
            schema_title: None,
            schema_description: None,
            existing_properties: vec![],
            examples: vec![],
            parent_context: None,
        };

        let schema = json!({"anyOf": [{"type": "string"}, {"type": "number"}]});
        let result = AdditionalPropertiesAnalyzer::analyze(
            &schema,
            &context,
            crate::optimization_pass::OptimizationLevel::O2,
        );

        match result {
            AdditionalPropertiesType::Typed(constraint) => {
                assert_eq!(constraint.rust_type, "Value");
                assert_eq!(constraint.confidence, 0.3);
            }
            _ => panic!("Expected Typed with Value rust_type"),
        }
    }

    #[test]
    fn test_composition_oneof_constraint() {
        let context = AnalysisContext {
            schema_title: None,
            schema_description: None,
            existing_properties: vec![],
            examples: vec![],
            parent_context: None,
        };

        let schema = json!({"oneOf": [{"type": "string"}, {"type": "integer"}]});
        let result = AdditionalPropertiesAnalyzer::analyze(
            &schema,
            &context,
            crate::optimization_pass::OptimizationLevel::O2,
        );

        match result {
            AdditionalPropertiesType::Typed(constraint) => {
                assert_eq!(constraint.rust_type, "Value");
                assert_eq!(constraint.confidence, 0.3);
            }
            _ => panic!("Expected Typed with Value rust_type"),
        }
    }

    // Batch 4: Examples-Based Inference

    #[test]
    fn test_infer_from_examples_uniform_strings() {
        let context = AnalysisContext {
            schema_title: None,
            schema_description: None,
            existing_properties: vec![],
            examples: vec![
                json!({"key1": "value1", "key2": "value2"}),
                json!({"key3": "value3", "key4": "value4"}),
            ],
            parent_context: None,
        };

        let result = AdditionalPropertiesAnalyzer::analyze(
            &json!(true),
            &context,
            crate::optimization_pass::OptimizationLevel::O3,
        );
        assert_eq!(
            result,
            AdditionalPropertiesType::Contextual(Box::new(
                AdditionalPropertiesType::StringOnlyUnconstrained
            ))
        );
    }

    #[test]
    fn test_infer_from_examples_uniform_numbers() {
        let context = AnalysisContext {
            schema_title: None,
            schema_description: None,
            existing_properties: vec![],
            examples: vec![
                json!({"count": 42, "total": 100}),
                json!({"score": 95.5, "rank": 1}),
            ],
            parent_context: None,
        };

        let result = AdditionalPropertiesAnalyzer::analyze(
            &json!(true),
            &context,
            crate::optimization_pass::OptimizationLevel::O3,
        );
        assert_eq!(
            result,
            AdditionalPropertiesType::Contextual(Box::new(
                AdditionalPropertiesType::NumberOnlyUnconstrained
            ))
        );
    }

    #[test]
    fn test_infer_from_examples_uniform_booleans() {
        let context = AnalysisContext {
            schema_title: None,
            schema_description: None,
            existing_properties: vec![],
            examples: vec![
                json!({"enabled": true, "active": false}),
                json!({"visible": true, "readonly": false}),
            ],
            parent_context: None,
        };

        let result = AdditionalPropertiesAnalyzer::analyze(
            &json!(true),
            &context,
            crate::optimization_pass::OptimizationLevel::O3,
        );
        assert_eq!(
            result,
            AdditionalPropertiesType::Contextual(Box::new(
                AdditionalPropertiesType::BooleanOnlyUnconstrained
            ))
        );
    }

    #[test]
    fn test_infer_from_examples_mixed_types() {
        let context = AnalysisContext {
            schema_title: None,
            schema_description: None,
            existing_properties: vec![],
            examples: vec![json!({"key1": "value", "key2": 42, "key3": true})],
            parent_context: None,
        };

        let result = AdditionalPropertiesAnalyzer::analyze(
            &json!(true),
            &context,
            crate::optimization_pass::OptimizationLevel::O3,
        );
        // Mixed types, no single inference - should fall back to Arbitrary
        assert_eq!(
            result,
            AdditionalPropertiesType::Contextual(Box::new(AdditionalPropertiesType::Arbitrary))
        );
    }

    // Batch 5: Schema Hints and Edge Cases

    #[test]
    fn test_schema_hint_metrics() {
        let context = AnalysisContext {
            schema_title: Some("Performance Metrics".to_string()),
            schema_description: None,
            existing_properties: vec![],
            examples: vec![],
            parent_context: None,
        };

        let result = AdditionalPropertiesAnalyzer::analyze(
            &json!(true),
            &context,
            crate::optimization_pass::OptimizationLevel::O3,
        );
        assert_eq!(
            result,
            AdditionalPropertiesType::Contextual(Box::new(
                AdditionalPropertiesType::NumberOnlyUnconstrained
            ))
        );
    }

    #[test]
    fn test_schema_hint_flags() {
        let context = AnalysisContext {
            schema_title: Some("Feature Flags".to_string()),
            schema_description: None,
            existing_properties: vec![],
            examples: vec![],
            parent_context: None,
        };

        let result = AdditionalPropertiesAnalyzer::analyze(
            &json!(true),
            &context,
            crate::optimization_pass::OptimizationLevel::O3,
        );
        assert_eq!(
            result,
            AdditionalPropertiesType::Contextual(Box::new(
                AdditionalPropertiesType::BooleanOnlyUnconstrained
            ))
        );
    }

    #[test]
    fn test_schema_hint_description() {
        let context = AnalysisContext {
            schema_title: None,
            schema_description: Some("Object with string values for all properties".to_string()),
            existing_properties: vec![],
            examples: vec![],
            parent_context: None,
        };

        let result = AdditionalPropertiesAnalyzer::analyze(
            &json!(true),
            &context,
            crate::optimization_pass::OptimizationLevel::O3,
        );
        assert_eq!(
            result,
            AdditionalPropertiesType::Contextual(Box::new(
                AdditionalPropertiesType::StringOnlyUnconstrained
            ))
        );
    }

    #[test]
    fn test_mixed_existing_properties() {
        let context = AnalysisContext {
            schema_title: None,
            schema_description: None,
            existing_properties: vec![
                PropertyInfo {
                    name: "name".to_string(),
                    json_type: "string".to_string(),
                    has_pattern: false,
                    is_required: true,
                },
                PropertyInfo {
                    name: "count".to_string(),
                    json_type: "number".to_string(),
                    has_pattern: false,
                    is_required: false,
                },
            ],
            examples: vec![],
            parent_context: None,
        };

        let result = AdditionalPropertiesAnalyzer::analyze(
            &json!(true),
            &context,
            crate::optimization_pass::OptimizationLevel::O3,
        );
        // 1 string out of 2 total: 1 >= (2 * 2/3) = 1 >= 1, so infers StringOnly
        assert_eq!(
            result,
            AdditionalPropertiesType::Contextual(Box::new(
                AdditionalPropertiesType::StringOnlyUnconstrained
            ))
        );
    }

    #[test]
    fn test_majority_string_properties() {
        let context = AnalysisContext {
            schema_title: None,
            schema_description: None,
            existing_properties: vec![
                PropertyInfo {
                    name: "name".to_string(),
                    json_type: "string".to_string(),
                    has_pattern: false,
                    is_required: true,
                },
                PropertyInfo {
                    name: "description".to_string(),
                    json_type: "string".to_string(),
                    has_pattern: false,
                    is_required: false,
                },
                PropertyInfo {
                    name: "version".to_string(),
                    json_type: "string".to_string(),
                    has_pattern: false,
                    is_required: false,
                },
                PropertyInfo {
                    name: "count".to_string(),
                    json_type: "number".to_string(),
                    has_pattern: false,
                    is_required: false,
                },
            ],
            examples: vec![],
            parent_context: None,
        };

        let result = AdditionalPropertiesAnalyzer::analyze(
            &json!(true),
            &context,
            crate::optimization_pass::OptimizationLevel::O3,
        );
        // 3/4 are strings (>= 2/3), should infer string-only
        assert_eq!(
            result,
            AdditionalPropertiesType::Contextual(Box::new(
                AdditionalPropertiesType::StringOnlyUnconstrained
            ))
        );
    }

    #[test]
    fn test_parent_context_environment() {
        let context = AnalysisContext {
            schema_title: None,
            schema_description: None,
            existing_properties: vec![],
            examples: vec![],
            parent_context: Some("environment variables".to_string()),
        };

        let result = AdditionalPropertiesAnalyzer::analyze(
            &json!(true),
            &context,
            crate::optimization_pass::OptimizationLevel::O3,
        );
        assert_eq!(
            result,
            AdditionalPropertiesType::Contextual(Box::new(
                AdditionalPropertiesType::StringOnlyUnconstrained
            ))
        );
    }

    #[test]
    fn test_parent_context_scripts() {
        let context = AnalysisContext {
            schema_title: None,
            schema_description: None,
            existing_properties: vec![],
            examples: vec![],
            parent_context: Some("scripts section".to_string()),
        };

        let result = AdditionalPropertiesAnalyzer::analyze(
            &json!(true),
            &context,
            crate::optimization_pass::OptimizationLevel::O3,
        );
        assert_eq!(
            result,
            AdditionalPropertiesType::Contextual(Box::new(
                AdditionalPropertiesType::StringOnlyUnconstrained
            ))
        );
    }

    #[test]
    fn test_invalid_schema_value() {
        let context = AnalysisContext {
            schema_title: None,
            schema_description: None,
            existing_properties: vec![],
            examples: vec![],
            parent_context: None,
        };

        // Invalid schema (number instead of object/boolean)
        let result = AdditionalPropertiesAnalyzer::analyze(
            &json!(42),
            &context,
            crate::optimization_pass::OptimizationLevel::O2,
        );
        assert_eq!(result, AdditionalPropertiesType::Arbitrary);
    }

    #[test]
    fn test_no_constraint_schema_object() {
        let context = AnalysisContext {
            schema_title: None,
            schema_description: None,
            existing_properties: vec![],
            examples: vec![],
            parent_context: None,
        };

        // Empty schema object - no type, no enum, no pattern
        let schema = json!({});
        let result = AdditionalPropertiesAnalyzer::analyze(
            &schema,
            &context,
            crate::optimization_pass::OptimizationLevel::O2,
        );

        match result {
            AdditionalPropertiesType::Typed(constraint) => {
                assert_eq!(constraint.rust_type, "Value");
                assert_eq!(constraint.confidence, 0.5);
            }
            _ => panic!("Expected Typed with Value rust_type"),
        }
    }
}
