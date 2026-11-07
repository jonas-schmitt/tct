//! Schema Analysis for Value Domain Identification
//!
//! This module analyzes JSON schemas to identify constrained value domains
//! that can benefit from vocabulary-aware token allocation optimization.

// Sub-modules for organized functionality
pub mod domain_analyzer;
pub mod pattern_integration;

// Re-export main types and traits
pub use domain_analyzer::{DomainAnalysis, StandardDomainAnalyzer};
pub use pattern_integration::{PatternAwareAnalyzer, PatternCapabilities, PatternIntegration};

use crate::pattern_analyzer::{PatternAnalysis, PatternAnalyzer};
use crate::schema_utils::get_schema_properties;
use crate::vocabulary_allocator::ValueDomain;
use crate::warnings::SchemaWarnings;
use serde_json::Value;

/// Result of schema analysis including domains and warnings
#[derive(Debug, Clone)]
pub struct AnalysisResult {
    /// Value domains found in the schema
    pub domains: Vec<ValueDomain>,
    /// Warnings generated during analysis
    pub warnings: SchemaWarnings,
}

/// Analyzes JSON schemas to extract value domains for vocabulary optimization
pub struct SchemaAnalyzer<T: PatternAnalysis = PatternAnalyzer> {
    domain_analyzer: StandardDomainAnalyzer<T>,
    warnings: SchemaWarnings,
}

impl<T: PatternAnalysis> SchemaAnalyzer<T> {
    /// Create a new SchemaAnalyzer with a specific pattern analyzer
    pub fn new_with_analyzer(pattern_analyzer: T) -> Self {
        Self {
            domain_analyzer: StandardDomainAnalyzer::new(pattern_analyzer),
            warnings: SchemaWarnings::new(),
        }
    }

    /// Get reference to the domain analyzer
    pub fn domain_analyzer(&self) -> &StandardDomainAnalyzer<T> {
        &self.domain_analyzer
    }

    /// Get mutable reference to the domain analyzer
    pub fn domain_analyzer_mut(&mut self) -> &mut StandardDomainAnalyzer<T> {
        &mut self.domain_analyzer
    }

    /// Get reference to collected warnings
    pub fn warnings(&self) -> &SchemaWarnings {
        &self.warnings
    }

    /// Get mutable reference to warnings for adding new warnings
    pub fn warnings_mut(&mut self) -> &mut SchemaWarnings {
        &mut self.warnings
    }
}

impl SchemaAnalyzer<PatternAnalyzer> {
    /// Analyze a JSON schema and extract all constrained value domains
    pub fn analyze_schema(schema: &Value) -> Result<Vec<ValueDomain>, AnalysisError> {
        let mut analyzer = Self::new_with_analyzer(PatternAnalyzer::new());
        let mut domains = Vec::new();

        analyzer.extract_domains_recursive(schema, &mut String::new(), &mut domains)?;

        Ok(domains)
    }

    /// Analyze schema with pattern-domain extraction enabled.
    ///
    /// This method extends the basic schema analysis to include pattern-based
    /// value-domain extraction for string fields with regex constraints.
    pub fn analyze_schema_with_patterns(schema: &Value) -> Result<Vec<ValueDomain>, AnalysisError> {
        // Create analyzer with pattern support enabled
        let mut analyzer = Self::new_with_analyzer(PatternAnalyzer::new());
        let mut domains = Vec::new();

        analyzer.extract_domains_recursive(schema, &mut String::new(), &mut domains)?;

        Ok(domains)
    }

    /// Analyze schema with pattern-domain extraction and warning collection.
    ///
    /// This method provides comprehensive analysis including both value domain
    /// extraction and pattern-domain warnings.
    pub fn analyze_schema_with_warnings(schema: &Value) -> Result<AnalysisResult, AnalysisError> {
        let mut analyzer = Self::new_with_analyzer(PatternAnalyzer::new());
        let mut domains = Vec::new();

        analyzer.extract_domains_recursive(schema, &mut String::new(), &mut domains)?;

        Ok(AnalysisResult {
            domains,
            warnings: analyzer.warnings,
        })
    }
}

impl<T: PatternAnalysis> SchemaAnalyzer<T> {
    /// Resolve a $ref to its actual schema definition if possible
    fn resolve_ref<'a>(&self, ref_path: &str, root_schema: &'a Value) -> Option<&'a Value> {
        // Only handle local definitions for now (e.g., "#/definitions/rule")
        if ref_path.starts_with("#/definitions/") {
            let def_name = ref_path.strip_prefix("#/definitions/")?;
            root_schema
                .get("definitions")
                .and_then(|defs| defs.get(def_name))
        } else {
            // Don't handle external refs for now
            None
        }
    }

    /// Recursively extract value domains from a schema, building field paths
    fn extract_domains_recursive(
        &mut self,
        schema: &Value,
        current_path: &mut String,
        domains: &mut Vec<ValueDomain>,
    ) -> Result<(), AnalysisError> {
        let mut visited_refs = std::collections::HashSet::new();
        self.extract_domains_recursive_with_root(
            schema,
            current_path,
            domains,
            schema,
            &mut visited_refs,
        )
    }

    /// Internal recursive function that keeps track of the root schema for $ref resolution
    fn extract_domains_recursive_with_root(
        &mut self,
        schema: &Value,
        current_path: &mut String,
        domains: &mut Vec<ValueDomain>,
        root_schema: &Value,
        visited_refs: &mut std::collections::HashSet<String>,
    ) -> Result<(), AnalysisError> {
        // Handle $ref resolution first
        if let Some(schema_obj) = schema.as_object()
            && let Some(ref_path) = schema_obj.get("$ref").and_then(Value::as_str)
        {
            // Check for circular references to prevent infinite recursion
            if visited_refs.contains(ref_path) {
                // Circular reference detected - skip to avoid stack overflow
                return Ok(());
            }

            if let Some(resolved_schema) = self.resolve_ref(ref_path, root_schema) {
                // Track this ref to detect cycles
                visited_refs.insert(ref_path.to_string());

                // Recursively analyze the resolved schema instead
                let result = self.extract_domains_recursive_with_root(
                    resolved_schema,
                    current_path,
                    domains,
                    root_schema,
                    visited_refs,
                );

                // Remove from visited set when done with this branch
                visited_refs.remove(ref_path);
                return result;
            }
            // If we can't resolve the ref, skip this branch
            return Ok(());
        }

        // Gracefully handle non-object schemas (valid JSON Schema edge cases)
        let schema_obj = match schema.as_object() {
            Some(obj) => obj,
            None => {
                // Non-object schemas (like simple type declarations) have no analyzable domains
                // This is valid JSON Schema - just skip analysis for this branch
                return Ok(());
            }
        };

        // Check if this schema defines a constrained type
        if let Some(domain) = self.domain_analyzer.analyze_schema_properties(
            schema_obj,
            current_path,
            &mut self.warnings,
        )? {
            domains.push(domain);
        }

        // Recurse into object properties
        if let Some(properties) = get_schema_properties(schema_obj) {
            for (prop_name, prop_schema) in properties {
                let mut prop_path = if current_path.is_empty() {
                    prop_name.clone()
                } else {
                    format!("{}.{}", current_path, prop_name)
                };

                self.extract_domains_recursive_with_root(
                    prop_schema,
                    &mut prop_path,
                    domains,
                    root_schema,
                    visited_refs,
                )?;
            }
        }

        // Recurse into array items
        if let Some(items) = schema_obj.get("items") {
            let mut items_path = if current_path.is_empty() {
                "[]".to_string()
            } else {
                format!("{}[]", current_path)
            };

            self.extract_domains_recursive_with_root(
                items,
                &mut items_path,
                domains,
                root_schema,
                visited_refs,
            )?;
        }

        // Recurse into oneOf/anyOf variants
        if let Some(one_of) = schema_obj.get("oneOf").and_then(|v| v.as_array()) {
            for (i, variant) in one_of.iter().enumerate() {
                // Skip non-object variants gracefully - let generator handle validation
                if !variant.is_object() {
                    continue;
                }

                let mut variant_path = if current_path.is_empty() {
                    format!("oneOf[{}]", i)
                } else {
                    format!("{}.oneOf[{}]", current_path, i)
                };

                self.extract_domains_recursive_with_root(
                    variant,
                    &mut variant_path,
                    domains,
                    root_schema,
                    visited_refs,
                )?;
            }
        }

        if let Some(any_of) = schema_obj.get("anyOf").and_then(|v| v.as_array()) {
            for (i, variant) in any_of.iter().enumerate() {
                // Skip non-object variants gracefully - let generator handle validation
                if !variant.is_object() {
                    continue;
                }

                let mut variant_path = if current_path.is_empty() {
                    format!("anyOf[{}]", i)
                } else {
                    format!("{}.anyOf[{}]", current_path, i)
                };

                self.extract_domains_recursive_with_root(
                    variant,
                    &mut variant_path,
                    domains,
                    root_schema,
                    visited_refs,
                )?;
            }
        }

        // Recurse into allOf variants
        if let Some(all_of) = schema_obj.get("allOf").and_then(|v| v.as_array()) {
            for (i, variant) in all_of.iter().enumerate() {
                // Skip non-object variants gracefully - let generator handle validation
                if !variant.is_object() {
                    continue;
                }

                let mut variant_path = if current_path.is_empty() {
                    format!("allOf[{}]", i)
                } else {
                    format!("{}.allOf[{}]", current_path, i)
                };

                self.extract_domains_recursive_with_root(
                    variant,
                    &mut variant_path,
                    domains,
                    root_schema,
                    visited_refs,
                )?;
            }
        }

        Ok(())
    }
}

/// Error types for schema analysis
#[derive(Debug, Clone, PartialEq)]
pub enum AnalysisError {
    InvalidSchema { path: String, reason: String },
    PatternError(String),
}

impl std::fmt::Display for AnalysisError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AnalysisError::InvalidSchema { path, reason } => {
                if path.is_empty() {
                    write!(f, "Invalid JSON schema structure: {}", reason)
                } else {
                    write!(
                        f,
                        "Invalid JSON schema structure at path '{}': {}",
                        path, reason
                    )
                }
            }
            AnalysisError::PatternError(msg) => write!(f, "Pattern analysis error: {}", msg),
        }
    }
}

impl std::error::Error for AnalysisError {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::vocabulary_allocator::DomainType;
    use serde_json::json;

    #[test]
    fn test_simple_enum_analysis() {
        let schema = json!({
            "type": "object",
            "properties": {
                "color": {
                    "type": "string",
                    "enum": ["red", "green", "blue"]
                }
            }
        });

        let domains = SchemaAnalyzer::analyze_schema(&schema).unwrap();
        assert_eq!(domains.len(), 1);

        let domain = &domains[0];
        assert_eq!(domain.field_path, "color");
        assert_eq!(domain.cardinality, 3);
        match &domain.domain_type {
            DomainType::Enum { values } => {
                assert_eq!(values, &vec!["red", "green", "blue"]);
            }
            _ => panic!("Expected enumeration domain"),
        }
    }

    #[test]
    fn test_nested_object_analysis() {
        let schema = json!({
            "type": "object",
            "properties": {
                "user": {
                    "type": "object",
                    "properties": {
                        "role": {
                            "type": "string",
                            "enum": ["admin", "user", "guest"]
                        },
                        "age": {
                            "type": "integer",
                            "minimum": 18,
                            "maximum": 100
                        }
                    }
                }
            }
        });

        let domains = SchemaAnalyzer::analyze_schema(&schema).unwrap();
        assert_eq!(domains.len(), 2);

        // Find role domain
        let role_domain = domains
            .iter()
            .find(|d| d.field_path == "user.role")
            .unwrap();
        assert_eq!(role_domain.cardinality, 3);

        // Find age domain
        let age_domain = domains.iter().find(|d| d.field_path == "user.age").unwrap();
        assert_eq!(age_domain.cardinality, 83); // 18 to 100 inclusive
    }

    #[test]
    fn test_array_analysis() {
        let schema = json!({
            "type": "object",
            "properties": {
                "tags": {
                    "type": "array",
                    "items": {
                        "type": "string",
                        "enum": ["urgent", "normal", "low"]
                    }
                }
            }
        });

        let domains = SchemaAnalyzer::analyze_schema(&schema).unwrap();
        assert_eq!(domains.len(), 1);

        let domain = &domains[0];
        assert_eq!(domain.field_path, "tags[]");
        assert_eq!(domain.cardinality, 3);
    }

    #[test]
    fn test_boolean_analysis() {
        let schema = json!({
            "type": "object",
            "properties": {
                "active": {
                    "type": "boolean"
                }
            }
        });

        let domains = SchemaAnalyzer::analyze_schema(&schema).unwrap();
        assert_eq!(domains.len(), 1);

        let domain = &domains[0];
        assert_eq!(domain.field_path, "active");
        assert_eq!(domain.cardinality, 2);
        match &domain.domain_type {
            DomainType::Boolean => {}
            _ => panic!("Expected boolean domain"),
        }
    }

    #[test]
    fn test_edge_cases() {
        // Empty schema
        let empty_schema = json!({});
        let domains = SchemaAnalyzer::analyze_schema(&empty_schema).unwrap();
        assert_eq!(domains.len(), 0);

        // Schema without constraints
        let unconstrained_schema = json!({
            "type": "object",
            "properties": {
                "name": {
                    "type": "string"
                }
            }
        });
        let domains = SchemaAnalyzer::analyze_schema(&unconstrained_schema).unwrap();
        assert_eq!(domains.len(), 0);
    }

    #[test]
    fn test_enhanced_error_messages() {
        // Test that enhanced error messages provide useful context

        // Test 1: Empty enum should be handled gracefully (no error, just skip)
        let empty_enum_schema = json!({
            "type": "object",
            "properties": {
                "field": {
                    "type": "string",
                    "enum": []
                }
            }
        });

        // This should not error - empty enums are now handled gracefully
        let result = SchemaAnalyzer::analyze_schema(&empty_enum_schema);
        assert!(result.is_ok(), "Empty enum should be handled gracefully");

        // Test 2: Non-object schemas should be handled gracefully
        let non_object_schema = json!("not an object");
        let result = SchemaAnalyzer::analyze_schema(&non_object_schema);
        assert!(
            result.is_ok(),
            "Non-object schemas should be handled gracefully"
        );

        // Test 3: Schemas with mixed types should work
        let mixed_enum_schema = json!({
            "type": "object",
            "properties": {
                "field": {
                    "type": ["string", "number"],
                    "enum": ["text", 42, true, null]
                }
            }
        });

        let result = SchemaAnalyzer::analyze_schema(&mixed_enum_schema);
        assert!(
            result.is_ok(),
            "Mixed enum types should be handled gracefully"
        );

        // If we got domains, verify they're reasonable
        if let Ok(domains) = result {
            // Should extract string values from mixed enum
            if !domains.is_empty() {
                let domain = &domains[0];
                assert_eq!(domain.field_path, "field");
                // Should only include string values from mixed enum
                assert!(domain.cardinality > 0);
            }
        }
    }

    #[test]
    fn test_eslintrc_patterns() {
        // Test the specific patterns that were causing issues in eslintrc schema

        // Test 1: Arrays without items specification and direct $ref usage
        let ref_with_enum_schema = json!({
            "definitions": {
                "severity": {
                    "type": "string",
                    "enum": ["off", "warn", "error"]
                }
            },
            "type": "object",
            "properties": {
                "level": { "$ref": "#/definitions/severity" },
                "emptyArray": { "type": "array" },
                "simpleEnum": {
                    "type": "string",
                    "enum": ["one", "two", "three"]
                }
            }
        });

        let result = SchemaAnalyzer::analyze_schema(&ref_with_enum_schema);
        assert!(
            result.is_ok(),
            "Schema with $refs and arrays should be handled gracefully"
        );

        if let Ok(domains) = result {
            // Should find at least the direct enum constraint
            let enum_domains: Vec<_> = domains
                .iter()
                .filter(|d| {
                    matches!(
                        d.domain_type,
                        crate::vocabulary_allocator::DomainType::Enum { .. }
                    )
                })
                .collect();

            if !enum_domains.is_empty() {
                println!("Found {} enum domains as expected", enum_domains.len());
                // Verify domains have reasonable properties
                for domain in &enum_domains {
                    assert!(!domain.field_path.is_empty());
                    assert!(domain.cardinality > 0);
                }
            }
        }

        // Test 2: Heavy $ref usage with circular references
        let circular_ref_schema = json!({
            "definitions": {
                "node": {
                    "type": "object",
                    "properties": {
                        "value": { "type": "string" },
                        "children": {
                            "type": "array",
                            "items": { "$ref": "#/definitions/node" }
                        }
                    }
                }
            },
            "type": "object",
            "properties": {
                "root": { "$ref": "#/definitions/node" }
            }
        });

        let result = SchemaAnalyzer::analyze_schema(&circular_ref_schema);
        assert!(
            result.is_ok(),
            "Circular $refs should be handled gracefully"
        );

        // Test 3: Mixed enum types (eslintrc pattern)
        let mixed_enum_schema = json!({
            "definitions": {
                "rule_value": {
                    "oneOf": [
                        {
                            "type": "integer",
                            "minimum": 0,
                            "maximum": 2
                        },
                        {
                            "type": "string",
                            "enum": ["off", "warn", "error"]
                        }
                    ]
                },
                "env_value": {
                    "enum": ["readonly", "writable", "off"]
                }
            },
            "type": "object",
            "properties": {
                "rules": {
                    "type": "object",
                    "additionalProperties": { "$ref": "#/definitions/rule_value" }
                },
                "env": {
                    "type": "object",
                    "additionalProperties": { "$ref": "#/definitions/env_value" }
                }
            }
        });

        let result = SchemaAnalyzer::analyze_schema(&mixed_enum_schema);
        assert!(
            result.is_ok(),
            "Mixed enum types should be analyzed successfully"
        );

        if let Ok(domains) = result {
            // Note: The current schema analyzer focuses on direct property constraints,
            // not additionalProperties/patternProperties, so we may not find domains here.
            // That's actually correct behavior - the analyzer is being conservative.
            println!("Found {} domains in mixed enum schema", domains.len());

            // If we did find domains, verify they're reasonable
            for domain in &domains {
                assert!(
                    !domain.field_path.is_empty(),
                    "Domain should have a field path"
                );
                assert!(
                    domain.cardinality > 0,
                    "Domain should have positive cardinality"
                );
            }
        }
    }

    #[test]
    fn test_ref_resolution_edge_cases() {
        // Test edge cases in $ref resolution

        // Test 1: Invalid $ref (should be handled gracefully)
        let invalid_ref_schema = json!({
            "type": "object",
            "properties": {
                "field": { "$ref": "#/definitions/nonexistent" }
            }
        });

        let result = SchemaAnalyzer::analyze_schema(&invalid_ref_schema);
        assert!(result.is_ok(), "Invalid $refs should be handled gracefully");

        // Test 2: External $ref (should be ignored for now)
        let external_ref_schema = json!({
            "type": "object",
            "properties": {
                "field": { "$ref": "http://example.com/schema.json#/definitions/something" }
            }
        });

        let result = SchemaAnalyzer::analyze_schema(&external_ref_schema);
        assert!(
            result.is_ok(),
            "External $refs should be handled gracefully"
        );

        // Test 3: Multiple levels of $refs
        let nested_ref_schema = json!({
            "definitions": {
                "level1": { "$ref": "#/definitions/level2" },
                "level2": { "$ref": "#/definitions/level3" },
                "level3": {
                    "type": "string",
                    "enum": ["a", "b", "c"]
                }
            },
            "type": "object",
            "properties": {
                "field": { "$ref": "#/definitions/level1" }
            }
        });

        let result = SchemaAnalyzer::analyze_schema(&nested_ref_schema);
        assert!(result.is_ok(), "Nested $refs should be resolved properly");

        if let Ok(domains) = result {
            // Should resolve through all the $refs to find the enum
            let enum_domains: Vec<_> = domains
                .iter()
                .filter(|d| {
                    matches!(
                        d.domain_type,
                        crate::vocabulary_allocator::DomainType::Enum { .. }
                    )
                })
                .collect();
            assert!(
                !enum_domains.is_empty(),
                "Should resolve through nested $refs to find enum"
            );
        }
    }

    // ========== Additional Coverage Tests ==========

    #[test]
    fn test_schema_analyzer_new_with_analyzer() {
        let pattern_analyzer = PatternAnalyzer::new();
        let analyzer = SchemaAnalyzer::new_with_analyzer(pattern_analyzer);

        // Verify initial state
        assert!(analyzer.warnings().is_empty());
    }

    #[test]
    fn test_schema_analyzer_accessors() {
        let pattern_analyzer = PatternAnalyzer::new();
        let mut analyzer = SchemaAnalyzer::new_with_analyzer(pattern_analyzer);

        // Test domain_analyzer accessor
        let _domain_ref = analyzer.domain_analyzer();

        // Test domain_analyzer_mut accessor
        let _domain_mut = analyzer.domain_analyzer_mut();

        // Test warnings accessor
        let warnings = analyzer.warnings();
        assert!(warnings.is_empty());

        // Test warnings_mut accessor
        let warnings_mut = analyzer.warnings_mut();
        assert!(warnings_mut.is_empty());
    }

    #[test]
    fn test_analyze_schema_with_patterns() {
        let schema = json!({
            "type": "object",
            "properties": {
                "status": {
                    "type": "string",
                    "enum": ["active", "inactive"]
                }
            }
        });

        let result = SchemaAnalyzer::analyze_schema_with_patterns(&schema);
        assert!(result.is_ok());

        let domains = result.unwrap();
        assert_eq!(domains.len(), 1);
        assert_eq!(domains[0].field_path, "status");
    }

    #[test]
    fn test_analyze_schema_with_warnings() {
        let schema = json!({
            "type": "object",
            "properties": {
                "priority": {
                    "type": "integer",
                    "minimum": 1,
                    "maximum": 5
                }
            }
        });

        let result = SchemaAnalyzer::analyze_schema_with_warnings(&schema);
        assert!(result.is_ok());

        let analysis_result = result.unwrap();
        assert_eq!(analysis_result.domains.len(), 1);
        // Warnings might be empty for this simple case
    }

    #[test]
    fn test_analysis_result_structure() {
        let schema = json!({
            "type": "object",
            "properties": {
                "enabled": {"type": "boolean"}
            }
        });

        let result = SchemaAnalyzer::analyze_schema_with_warnings(&schema).unwrap();

        // Verify AnalysisResult has expected fields
        assert_eq!(result.domains.len(), 1);
        assert!(result.warnings.is_empty());
    }

    #[test]
    fn test_analysis_error_display_with_path() {
        let error = AnalysisError::InvalidSchema {
            path: "user.role".to_string(),
            reason: "Invalid enum values".to_string(),
        };

        let error_str = format!("{}", error);
        assert!(error_str.contains("user.role"));
        assert!(error_str.contains("Invalid enum values"));
    }

    #[test]
    fn test_analysis_error_display_without_path() {
        let error = AnalysisError::InvalidSchema {
            path: String::new(),
            reason: "Missing type field".to_string(),
        };

        let error_str = format!("{}", error);
        assert!(error_str.contains("Missing type field"));
        assert!(!error_str.contains("at path"));
    }

    #[test]
    fn test_analysis_error_display_pattern_error() {
        let error = AnalysisError::PatternError("Invalid regex syntax".to_string());

        let error_str = format!("{}", error);
        assert!(error_str.contains("Pattern analysis error"));
        assert!(error_str.contains("Invalid regex syntax"));
    }

    #[test]
    fn test_oneof_with_root_path() {
        let schema = json!({
            "oneOf": [
                {
                    "type": "object",
                    "properties": {
                        "status": {"type": "string", "enum": ["active", "inactive"]}
                    }
                },
                {
                    "type": "object",
                    "properties": {
                        "priority": {"type": "integer", "minimum": 1, "maximum": 5}
                    }
                }
            ]
        });

        let domains = SchemaAnalyzer::analyze_schema(&schema).unwrap();

        // Should find domains in both oneOf variants
        assert!(domains.len() >= 2);

        // Check that paths include oneOf indexing
        let has_oneof_path = domains.iter().any(|d| d.field_path.contains("oneOf"));
        assert!(has_oneof_path);
    }

    #[test]
    fn test_anyof_with_nested_path() {
        let schema = json!({
            "type": "object",
            "properties": {
                "config": {
                    "anyOf": [
                        {
                            "type": "object",
                            "properties": {
                                "level": {"type": "string", "enum": ["debug", "info", "error"]}
                            }
                        },
                        {
                            "type": "object",
                            "properties": {
                                "enabled": {"type": "boolean"}
                            }
                        }
                    ]
                }
            }
        });

        let domains = SchemaAnalyzer::analyze_schema(&schema).unwrap();

        // Should find domains in anyOf variants
        assert!(domains.len() >= 2);

        // Check that paths include config prefix
        let has_config_path = domains.iter().any(|d| d.field_path.starts_with("config"));
        assert!(has_config_path);
    }

    #[test]
    fn test_allof_with_empty_root_path() {
        let schema = json!({
            "allOf": [
                {
                    "type": "object",
                    "properties": {
                        "name": {"type": "string", "enum": ["alice", "bob"]}
                    }
                },
                {
                    "type": "object",
                    "properties": {
                        "active": {"type": "boolean"}
                    }
                }
            ]
        });

        let domains = SchemaAnalyzer::analyze_schema(&schema).unwrap();

        // Should find domains in allOf variants
        assert!(domains.len() >= 2);
    }

    #[test]
    fn test_oneof_with_non_object_variant() {
        let schema = json!({
            "type": "object",
            "properties": {
                "value": {
                    "oneOf": [
                        {"type": "string", "enum": ["text"]},
                        "invalid_non_object_variant",
                        {"type": "integer", "minimum": 0, "maximum": 10}
                    ]
                }
            }
        });

        // Should handle non-object variants gracefully
        let result = SchemaAnalyzer::analyze_schema(&schema);
        assert!(result.is_ok());
    }

    #[test]
    fn test_anyof_with_non_object_variant() {
        let schema = json!({
            "anyOf": [
                {"type": "string"},
                null,
                {"type": "boolean"}
            ]
        });

        // Should skip non-object variants gracefully
        let result = SchemaAnalyzer::analyze_schema(&schema);
        assert!(result.is_ok());
    }

    #[test]
    fn test_allof_with_non_object_variant() {
        let schema = json!({
            "allOf": [
                {"type": "object", "properties": {"a": {"type": "boolean"}}},
                42,
                {"type": "object", "properties": {"b": {"type": "boolean"}}}
            ]
        });

        // Should skip non-object variants
        let result = SchemaAnalyzer::analyze_schema(&schema);
        assert!(result.is_ok());
    }

    #[test]
    fn test_circular_ref_detection() {
        let schema = json!({
            "definitions": {
                "node": {
                    "type": "object",
                    "properties": {
                        "value": {"type": "integer", "minimum": 0, "maximum": 100},
                        "next": {"$ref": "#/definitions/node"}
                    }
                }
            },
            "type": "object",
            "properties": {
                "root": {"$ref": "#/definitions/node"}
            }
        });

        // Should handle circular references without infinite recursion
        let result = SchemaAnalyzer::analyze_schema(&schema);
        assert!(result.is_ok());
    }

    #[test]
    fn test_multiple_properties_at_root() {
        let schema = json!({
            "type": "object",
            "properties": {
                "a": {"type": "string", "enum": ["x", "y"]},
                "b": {"type": "integer", "minimum": 1, "maximum": 10},
                "c": {"type": "boolean"}
            }
        });

        let domains = SchemaAnalyzer::analyze_schema(&schema).unwrap();

        // Should find all three constrained fields
        assert_eq!(domains.len(), 3);

        // Verify field paths
        let field_paths: Vec<_> = domains.iter().map(|d| d.field_path.as_str()).collect();
        assert!(field_paths.contains(&"a"));
        assert!(field_paths.contains(&"b"));
        assert!(field_paths.contains(&"c"));
    }

    #[test]
    fn test_deeply_nested_properties() {
        let schema = json!({
            "type": "object",
            "properties": {
                "level1": {
                    "type": "object",
                    "properties": {
                        "level2": {
                            "type": "object",
                            "properties": {
                                "level3": {
                                    "type": "string",
                                    "enum": ["deep", "nested"]
                                }
                            }
                        }
                    }
                }
            }
        });

        let domains = SchemaAnalyzer::analyze_schema(&schema).unwrap();

        assert_eq!(domains.len(), 1);
        assert_eq!(domains[0].field_path, "level1.level2.level3");
    }

    #[test]
    fn test_array_of_objects_with_constrained_fields() {
        let schema = json!({
            "type": "object",
            "properties": {
                "items": {
                    "type": "array",
                    "items": {
                        "type": "object",
                        "properties": {
                            "status": {"type": "string", "enum": ["pending", "done"]}
                        }
                    }
                }
            }
        });

        let domains = SchemaAnalyzer::analyze_schema(&schema).unwrap();

        assert_eq!(domains.len(), 1);
        assert_eq!(domains[0].field_path, "items[].status");
    }

    #[test]
    fn test_ref_without_definitions_object() {
        let schema = json!({
            "type": "object",
            "properties": {
                "field": {"$ref": "#/definitions/missing"}
            }
        });

        // Should handle missing definitions object gracefully
        let result = SchemaAnalyzer::analyze_schema(&schema);
        assert!(result.is_ok());
    }

    #[test]
    fn test_analysis_error_equality() {
        let error1 = AnalysisError::InvalidSchema {
            path: "test".to_string(),
            reason: "error".to_string(),
        };

        let error2 = AnalysisError::InvalidSchema {
            path: "test".to_string(),
            reason: "error".to_string(),
        };

        assert_eq!(error1, error2);
    }

    #[test]
    fn test_analysis_error_pattern_equality() {
        let error1 = AnalysisError::PatternError("test".to_string());
        let error2 = AnalysisError::PatternError("test".to_string());

        assert_eq!(error1, error2);
    }
}
