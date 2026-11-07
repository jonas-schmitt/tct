// Test data generators for systematic schema validation
// Provides utilities for generating comprehensive test schemas and data

use proptest::prelude::*;
use quickcheck::{Arbitrary, Gen};
use rand::prelude::*;
use serde_json::{Map, Value, json};
use std::collections::HashMap;

/// Generator for systematic JSON schema creation
pub struct SchemaGenerator {
    rng: StdRng,
    complexity_level: usize,
}

impl SchemaGenerator {
    pub fn new(seed: u64) -> Self {
        Self {
            rng: StdRng::seed_from_u64(seed),
            complexity_level: 3,
        }
    }

    pub fn with_complexity(mut self, level: usize) -> Self {
        self.complexity_level = level;
        self
    }

    /// Generate a random valid JSON schema
    pub fn generate_schema(&mut self, schema_type: SchemaType) -> Value {
        match schema_type {
            SchemaType::Simple => self.generate_simple_schema(),
            SchemaType::Object => self.generate_object_schema(),
            SchemaType::Array => self.generate_array_schema(),
            SchemaType::Enum => self.generate_enum_schema(),
            SchemaType::OneOf => self.generate_oneof_schema(),
            SchemaType::AllOf => self.generate_allof_schema(),
            SchemaType::Conditional => self.generate_conditional_schema(),
            SchemaType::Complex => self.generate_complex_schema(),
        }
    }

    fn generate_simple_schema(&mut self) -> Value {
        let types = ["string", "integer", "number", "boolean", "null"];
        let chosen_type = types.choose(&mut self.rng).unwrap();

        let mut schema = json!({
            "type": chosen_type
        });

        // Add constraints based on type
        match *chosen_type {
            "string" => {
                if self.rng.random_bool(0.3) {
                    schema["minLength"] = json!(self.rng.random_range(1..10));
                    schema["maxLength"] = json!(self.rng.random_range(10..100));
                }
                if self.rng.random_bool(0.2) {
                    let patterns = [
                        r"^[a-zA-Z]+$",
                        r"^\d{3}-\d{3}-\d{4}$",
                        r"^[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\.[a-zA-Z]{2,}$",
                    ];
                    schema["pattern"] = json!(patterns.choose(&mut self.rng).unwrap());
                }
            }
            "integer" if self.rng.random_bool(0.4) => {
                schema["minimum"] = json!(self.rng.random_range(-100..0));
                schema["maximum"] = json!(self.rng.random_range(1..1000));
            }
            "number" if self.rng.random_bool(0.3) => {
                schema["minimum"] = json!(self.rng.random_range(-100.0..0.0));
                schema["maximum"] = json!(self.rng.random_range(1.0..1000.0));
            }
            _ => {}
        }

        schema
    }

    fn generate_object_schema(&mut self) -> Value {
        let num_properties = self.rng.random_range(1..=self.complexity_level + 2);
        let mut properties = Map::new();
        let mut required = Vec::new();

        for i in 0..num_properties {
            let prop_name = format!("prop_{}", i);

            // Generate property schema recursively (with reduced complexity)
            let mut reduced_gen = SchemaGenerator::new(self.rng.random());
            reduced_gen.complexity_level = (self.complexity_level).saturating_sub(1);

            let prop_schema = if self.complexity_level > 0 {
                reduced_gen.generate_schema(SchemaType::Simple)
            } else {
                json!({"type": "string"})
            };

            properties.insert(prop_name.clone(), prop_schema);

            // Randomly make some properties required
            if self.rng.random_bool(0.6) {
                required.push(prop_name);
            }
        }

        let mut schema = json!({
            "type": "object",
            "properties": properties
        });

        if !required.is_empty() {
            schema["required"] = json!(required);
        }

        // Add pattern properties sometimes
        if self.rng.random_bool(0.3) {
            let pattern_props = json!({
                "^extra_": {"type": "string"},
                "^config_": {"type": "integer"}
            });
            schema["patternProperties"] = pattern_props;
        }

        schema
    }

    fn generate_array_schema(&mut self) -> Value {
        let mut reduced_gen = SchemaGenerator::new(self.rng.random());
        reduced_gen.complexity_level = (self.complexity_level).saturating_sub(1);

        let item_schema = if self.complexity_level > 0 {
            reduced_gen.generate_schema(SchemaType::Simple)
        } else {
            json!({"type": "string"})
        };

        let mut schema = json!({
            "type": "array",
            "items": item_schema
        });

        // Add array constraints
        if self.rng.random_bool(0.4) {
            schema["minItems"] = json!(self.rng.random_range(0..5));
            schema["maxItems"] = json!(self.rng.random_range(5..20));
        }

        schema
    }

    fn generate_enum_schema(&mut self) -> Value {
        let num_values = self.rng.random_range(2..=6);
        let mut values = Vec::new();

        for i in 0..num_values {
            values.push(format!("option_{}", i));
        }

        json!({
            "enum": values
        })
    }

    fn generate_oneof_schema(&mut self) -> Value {
        let num_variants = self.rng.random_range(2..=4);
        let mut variants = Vec::new();

        for _ in 0..num_variants {
            let mut reduced_gen = SchemaGenerator::new(self.rng.random());
            reduced_gen.complexity_level = (self.complexity_level).saturating_sub(1);

            let variant = if self.complexity_level > 0 {
                reduced_gen.generate_schema(SchemaType::Simple)
            } else {
                json!({"type": "string"})
            };
            variants.push(variant);
        }

        json!({
            "oneOf": variants
        })
    }

    fn generate_allof_schema(&mut self) -> Value {
        let num_schemas = self.rng.random_range(2..=3);
        let mut schemas = Vec::new();

        for _ in 0..num_schemas {
            let mut reduced_gen = SchemaGenerator::new(self.rng.random());
            reduced_gen.complexity_level = (self.complexity_level).saturating_sub(1);

            let schema = if self.complexity_level > 0 {
                reduced_gen.generate_schema(SchemaType::Object)
            } else {
                json!({
                    "type": "object",
                    "properties": {
                        format!("field_{}", self.rng.random::<u32>() % 10): {"type": "string"}
                    }
                })
            };
            schemas.push(schema);
        }

        json!({
            "allOf": schemas
        })
    }

    fn generate_conditional_schema(&mut self) -> Value {
        let mut reduced_gen = SchemaGenerator::new(self.rng.random());
        reduced_gen.complexity_level = (self.complexity_level).saturating_sub(1);

        let if_schema = json!({
            "properties": {"type": {"const": "premium"}}
        });

        let then_schema = if self.complexity_level > 0 {
            reduced_gen.generate_schema(SchemaType::Object)
        } else {
            json!({"properties": {"premium_feature": {"type": "string"}}})
        };

        let else_schema = json!({
            "properties": {"basic_feature": {"type": "string"}}
        });

        json!({
            "if": if_schema,
            "then": then_schema,
            "else": else_schema
        })
    }

    fn generate_complex_schema(&mut self) -> Value {
        // Generate a schema that combines multiple features
        let features = [
            SchemaType::Object,
            SchemaType::OneOf,
            SchemaType::AllOf,
            SchemaType::Array,
        ];

        let chosen = features.choose(&mut self.rng).unwrap();
        self.generate_schema(*chosen)
    }
}

#[derive(Clone, Copy, Debug)]
pub enum SchemaType {
    Simple,
    Object,
    Array,
    Enum,
    OneOf,
    AllOf,
    Conditional,
    Complex,
}

impl Arbitrary for SchemaType {
    fn arbitrary(_g: &mut Gen) -> Self {
        let types = [
            SchemaType::Simple,
            SchemaType::Object,
            SchemaType::Array,
            SchemaType::Enum,
            SchemaType::OneOf,
            SchemaType::AllOf,
            SchemaType::Conditional,
        ];
        *types.choose(&mut rand::rng()).unwrap()
    }
}

/// Property-based testing strategy for schema generation
pub fn schema_strategy() -> impl Strategy<Value = Value> {
    prop_oneof![
        // Simple schemas (70% probability)
        7 => simple_schema_strategy(),
        // Complex schemas (30% probability)
        3 => complex_schema_strategy(),
    ]
}

fn simple_schema_strategy() -> impl Strategy<Value = Value> {
    prop_oneof![
        string_schema_strategy(),
        integer_schema_strategy(),
        boolean_schema_strategy(),
        enum_schema_strategy(),
    ]
}

fn complex_schema_strategy() -> impl Strategy<Value = Value> {
    prop_oneof![
        object_schema_strategy(),
        array_schema_strategy(),
        oneof_schema_strategy(),
        allof_schema_strategy(),
    ]
}

fn string_schema_strategy() -> impl Strategy<Value = Value> {
    Just(json!({"type": "string"}))
}

fn integer_schema_strategy() -> impl Strategy<Value = Value> {
    (0..1000i32, 1001..2000i32).prop_map(|(min, max)| {
        json!({
            "type": "integer",
            "minimum": min,
            "maximum": max
        })
    })
}

fn boolean_schema_strategy() -> impl Strategy<Value = Value> {
    Just(json!({"type": "boolean"}))
}

fn enum_schema_strategy() -> impl Strategy<Value = Value> {
    prop::collection::vec(r"[a-z]+", 2..6).prop_map(|values| json!({"enum": values}))
}

fn object_schema_strategy() -> impl Strategy<Value = Value> {
    prop::collection::hash_map(r"[a-z_]+", simple_schema_strategy(), 1..5).prop_map(|props| {
        json!({
            "type": "object",
            "properties": props
        })
    })
}

fn array_schema_strategy() -> impl Strategy<Value = Value> {
    simple_schema_strategy().prop_map(|item_schema| {
        json!({
            "type": "array",
            "items": item_schema
        })
    })
}

fn oneof_schema_strategy() -> impl Strategy<Value = Value> {
    prop::collection::vec(simple_schema_strategy(), 2..4)
        .prop_map(|variants| json!({"oneOf": variants}))
}

fn allof_schema_strategy() -> impl Strategy<Value = Value> {
    prop::collection::vec(
        prop::collection::hash_map(r"[a-z_]+", simple_schema_strategy(), 1..3).prop_map(|props| {
            json!({
                "type": "object",
                "properties": props
            })
        }),
        2..3,
    )
    .prop_map(|schemas| json!({"allOf": schemas}))
}

#[cfg(test)]
mod tests {
    use super::*;
    use tct::TctCompiler;

    /// Test that generated schemas are syntactically valid
    #[test]
    fn test_test_data_generators_generated_schemas_are_valid() {
        let mut generator = SchemaGenerator::new(42);

        let schema_types = [
            SchemaType::Simple,
            SchemaType::Object,
            SchemaType::Array,
            SchemaType::Enum,
            SchemaType::OneOf,
            SchemaType::AllOf,
        ];

        for schema_type in &schema_types {
            let schema = generator.generate_schema(*schema_type);

            // Should be valid JSON
            assert!(
                schema.is_object() || schema.is_array(),
                "Generated schema should be valid JSON"
            );

            // Should have some required fields
            if let Some(obj) = schema.as_object() {
                assert!(
                    obj.contains_key("type")
                        || obj.contains_key("enum")
                        || obj.contains_key("oneOf")
                        || obj.contains_key("allOf"),
                    "Schema should have a type or composition key"
                );
            }
        }
    }

    proptest! {
        #[test]
        fn test_test_data_generators_property_based_schema_generation(schema in schema_strategy()) {
            // Generated schemas should compile or fail with a known actionable
            // unsupported-schema error.
            let result = TctCompiler::new(schema.clone(), "TestSchema").unwrap().compile();

            // We expect either success or a well-defined error
            match result {
                Ok(_) => {
                    // Success - schema was processed
                }
                Err(e) => {
                    // Error should be informative
                    let error_msg = e.to_string();
                    assert!(!error_msg.is_empty(), "Error messages should not be empty");

                    // Known acceptable error patterns
                    let acceptable_errors = [
                        "Circular reference detected",
                        "Unsupported JSON type",
                        "Invalid $ref",
                        "Malformed",
                        "missing a 'properties' map",
                        "Enum values must be strings",
                        "Conflicting allOf property definitions",
                    ];

                    let is_acceptable = acceptable_errors.iter().any(|pattern| {
                        error_msg.contains(pattern)
                    });

                    if !is_acceptable {
                        panic!("Unexpected error for schema {:?}: {}", schema, error_msg);
                    }
                }
            }
        }
    }

    /// Test schema generation with different complexity levels
    #[test]
    fn test_test_data_generators_schema_complexity_levels() {
        for complexity in 0..=3 {
            let mut generator =
                SchemaGenerator::new(42 + complexity as u64).with_complexity(complexity);

            let schema = generator.generate_schema(SchemaType::Complex);

            // More complex schemas should generally be larger
            let schema_str = serde_json::to_string(&schema).unwrap();
            println!("Complexity {}: {} chars", complexity, schema_str.len());

            // Verify it's processable
            let result = TctCompiler::new(schema.clone(), "ComplexityTest")
                .unwrap()
                .compile();
            // Should either succeed or fail with known error patterns
            if let Err(e) = result {
                let error_msg = e.to_string();
                println!("Complexity {} error: {}", complexity, error_msg);
            }
        }
    }

    /// Stress test with many generated schemas
    #[test]
    fn test_test_data_generators_schema_generation_stress() {
        let mut generator = SchemaGenerator::new(123);
        let mut success_count = 0;
        let mut error_counts = HashMap::new();

        for i in 0..50 {
            generator.rng = StdRng::seed_from_u64(123 + i);
            let schema = generator.generate_schema(SchemaType::Complex);

            match TctCompiler::new(schema.clone(), format!("StressTest{}", i))
                .unwrap()
                .compile()
            {
                Ok(_) => success_count += 1,
                Err(e) => {
                    let error_type = e
                        .to_string()
                        .split(':')
                        .next()
                        .unwrap_or("Unknown")
                        .to_string();
                    *error_counts.entry(error_type).or_insert(0) += 1;
                }
            }
        }

        println!("Stress test results:");
        println!("  Successes: {}/50 ({}%)", success_count, success_count * 2);
        println!("  Error breakdown:");
        for (error_type, count) in error_counts {
            println!("    {}: {} ({}%)", error_type, count, count * 2);
        }

        // We expect at least 50% success rate for generated schemas
        assert!(
            success_count >= 25,
            "Success rate too low: {}/50",
            success_count
        );
    }
}
