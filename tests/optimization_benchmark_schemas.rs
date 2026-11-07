// Optimization benchmark test schema and data generators
// Provides standardized test cases for performance benchmarking across optimization levels
// Used by optimization benchmark tests to ensure consistent test data

use serde_json::{Value, json};

/// Create simple schema for basic performance testing
pub fn create_simple_schema() -> Value {
    json!({
        "title": "Simple",
        "type": "object",
        "properties": {
            "name": { "type": "string" },
            "active": { "type": "boolean" }
        },
        "required": ["name"]
    })
}

/// Create simple test data matching simple schema
pub fn create_simple_data() -> Value {
    json!({
        "name": "Simple Test Item",
        "active": true
    })
}

/// Create medium complexity schema with enums and nested objects
pub fn create_medium_schema() -> Value {
    json!({
        "title": "Medium",
        "type": "object",
        "properties": {
            "id": { "type": "string" },
            "status": { "enum": ["pending", "active", "completed", "cancelled"] },
            "metadata": {
                "type": "object",
                "properties": {
                    "created_at": { "type": "string" },
                    "updated_at": { "type": "string" },
                    "priority": { "type": "integer" }
                }
            },
            "tags": {
                "type": "array",
                "items": { "type": "string" }
            }
        },
        "required": ["id", "status"]
    })
}

/// Create medium complexity test data matching medium schema
pub fn create_medium_data() -> Value {
    json!({
        "id": "medium-test-12345",
        "status": "active",
        "metadata": {
            "created_at": "2024-01-15T10:30:00Z",
            "updated_at": "2024-01-15T10:30:00Z",
            "priority": 5
        },
        "tags": ["important", "test", "benchmark"]
    })
}

/// Create complex nested schema with multiple levels and arrays
pub fn create_complex_schema() -> Value {
    json!({
        "title": "Complex",
        "type": "object",
        "properties": {
            "order_id": { "type": "string" },
            "customer": {
                "type": "object",
                "properties": {
                    "id": { "type": "string" },
                    "email": { "type": "string" },
                    "preferences": {
                        "type": "object",
                        "properties": {
                            "notifications": { "type": "boolean" },
                            "theme": { "enum": ["light", "dark", "auto"] }
                        }
                    }
                },
                "required": ["id", "email"]
            },
            "items": {
                "type": "array",
                "items": {
                    "type": "object",
                    "properties": {
                        "sku": { "type": "string" },
                        "quantity": { "type": "integer" },
                        "price": { "type": "number" },
                        "category": { "enum": ["electronics", "books", "clothing", "home"] }
                    },
                    "required": ["sku", "quantity", "price", "category"]
                }
            },
            "shipping": {
                "type": "object",
                "properties": {
                    "method": { "enum": ["standard", "express", "overnight"] },
                    "address": {
                        "type": "object",
                        "properties": {
                            "street": { "type": "string" },
                            "city": { "type": "string" },
                            "country": { "type": "string" },
                            "postal_code": { "type": "string" }
                        },
                        "required": ["street", "city", "country"]
                    }
                },
                "required": ["method", "address"]
            }
        },
        "required": ["order_id", "customer", "items"]
    })
}

/// Create complex test data matching complex schema
pub fn create_complex_data() -> Value {
    json!({
        "order_id": "ORD-2024-001234",
        "customer": {
            "id": "CUST-78901",
            "email": "customer@example.com",
            "preferences": {
                "notifications": true,
                "theme": "dark"
            }
        },
        "items": [
            {
                "sku": "ELEC-001",
                "quantity": 1,
                "price": 299.99,
                "category": "electronics"
            },
            {
                "sku": "BOOK-005",
                "quantity": 2,
                "price": 24.99,
                "category": "books"
            }
        ],
        "shipping": {
            "method": "express",
            "address": {
                "street": "123 Main St",
                "city": "New York",
                "country": "USA",
                "postal_code": "10001"
            }
        }
    })
}

/// Create enum-heavy schema to showcase optimization effectiveness
pub fn create_enum_heavy_schema() -> Value {
    json!({
        "title": "EnumHeavy",
        "type": "object",
        "properties": {
            "status": { "enum": ["draft", "pending", "approved", "rejected", "cancelled"] },
            "priority": { "enum": ["low", "medium", "high", "critical"] },
            "category": { "enum": ["bug", "feature", "improvement", "documentation", "test"] },
            "severity": { "enum": ["trivial", "minor", "major", "critical", "blocker"] },
            "environment": { "enum": ["dev", "staging", "production"] },
            "browser": { "enum": ["chrome", "firefox", "safari", "edge"] },
            "os": { "enum": ["windows", "macos", "linux", "mobile"] },
            "framework": { "enum": ["react", "vue", "angular", "svelte"] }
        },
        "required": ["status", "priority", "category"]
    })
}

/// Create enum-heavy test data matching enum-heavy schema
pub fn create_enum_heavy_data() -> Value {
    json!({
        "status": "approved",
        "priority": "high",
        "category": "feature",
        "severity": "major",
        "environment": "production",
        "browser": "chrome",
        "os": "linux",
        "framework": "react"
    })
}

/// Get all benchmark test cases as (name, schema, data) tuples
pub fn get_all_benchmark_test_cases() -> Vec<(&'static str, Value, Value)> {
    vec![
        ("Simple", create_simple_schema(), create_simple_data()),
        ("Medium", create_medium_schema(), create_medium_data()),
        ("Complex", create_complex_schema(), create_complex_data()),
        (
            "EnumHeavy",
            create_enum_heavy_schema(),
            create_enum_heavy_data(),
        ),
    ]
}
