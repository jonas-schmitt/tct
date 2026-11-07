//! Utility functions for common JSON Schema traversal patterns
//!
//! This module consolidates duplicate schema parsing logic to follow DRY principles
//! and provide consistent schema handling across the codebase.

use crate::error::{ProcessingPhase, TctError, TctResult};
use serde_json::{Map, Value};

/// Default ceiling for recursive schema traversal.
///
/// This is intentionally high enough for ordinary real-world schemas while
/// still rejecting pathological schemas before they can exhaust the call stack.
pub const DEFAULT_MAX_SCHEMA_TRAVERSAL_DEPTH: usize = 512;

fn json_pointer_child_path(parent: &str, segment: &str) -> String {
    if parent == "#" {
        format!("#/{}", escape_json_pointer_segment(segment))
    } else {
        format!("{}/{}", parent, escape_json_pointer_segment(segment))
    }
}

/// Build a structured depth-limit error for schema traversal.
pub fn schema_traversal_depth_error(
    schema_path: impl Into<String>,
    depth: usize,
    max_depth: usize,
) -> TctError {
    TctError::SchemaProcessing {
        message: format!(
            "schema traversal depth {} exceeds configured maximum {}",
            depth, max_depth
        ),
        schema_path: schema_path.into(),
        suggestion: "Reduce schema nesting, pre-resolve pathological generated schemas, or raise TctCompiler::with_max_schema_depth(...) only for trusted schemas.".to_string(),
        phase: ProcessingPhase::Analysis,
    }
}

/// Validate that a schema's raw JSON tree fits within the traversal ceiling.
///
/// This uses an explicit stack rather than recursion so it can reject
/// pathological schemas before recursive discovery code touches them.
pub fn validate_schema_traversal_depth(schema: &Value, max_depth: usize) -> TctResult<()> {
    let mut stack = vec![(schema, "#".to_string(), 1usize)];

    while let Some((value, path, depth)) = stack.pop() {
        if depth > max_depth {
            return Err(schema_traversal_depth_error(path, depth, max_depth));
        }

        match value {
            Value::Object(obj) => {
                for (key, child) in obj {
                    stack.push((child, json_pointer_child_path(&path, key), depth + 1));
                }
            }
            Value::Array(items) => {
                for (index, child) in items.iter().enumerate() {
                    stack.push((
                        child,
                        json_pointer_child_path(&path, &index.to_string()),
                        depth + 1,
                    ));
                }
            }
            _ => {}
        }
    }

    Ok(())
}

/// A trait for types that can traverse JSON schemas recursively
pub trait SchemaTraversal {
    /// The result type returned by schema processing
    type Result;

    /// Process a schema object and return the result
    fn process_schema(&mut self, schema: &Map<String, Value>, path: &str) -> Self::Result;

    /// Combine results from multiple schema traversals (e.g., for arrays or compositions)
    fn combine_results(&self, results: Vec<Self::Result>) -> Self::Result;
}

/// Traverse a schema recursively using the provided processor
pub fn traverse_schema<T: SchemaTraversal>(
    schema: &Value,
    processor: &mut T,
    current_path: &str,
) -> T::Result {
    let schema_obj = match schema.as_object() {
        Some(obj) => obj,
        None => return processor.process_schema(&Map::new(), current_path),
    };

    // Process current schema
    let current_result = processor.process_schema(schema_obj, current_path);

    let mut sub_results = vec![current_result];

    // Traverse properties
    if let Some(properties) = schema_obj.get("properties").and_then(|p| p.as_object()) {
        for (prop_name, prop_schema) in properties {
            let prop_path = if current_path.is_empty() {
                prop_name.clone()
            } else {
                format!("{}.{}", current_path, prop_name)
            };
            let result = traverse_schema(prop_schema, processor, &prop_path);
            sub_results.push(result);
        }
    }

    // Traverse array items
    if let Some(items) = schema_obj.get("items") {
        let items_path = if current_path.is_empty() {
            "[]".to_string()
        } else {
            format!("{}[]", current_path)
        };
        let result = traverse_schema(items, processor, &items_path);
        sub_results.push(result);
    }

    // Traverse oneOf/anyOf variants
    if let Some(one_of) = schema_obj.get("oneOf").and_then(|v| v.as_array()) {
        for (i, variant) in one_of.iter().enumerate() {
            let variant_path = if current_path.is_empty() {
                format!("oneOf[{}]", i)
            } else {
                format!("{}.oneOf[{}]", current_path, i)
            };
            let result = traverse_schema(variant, processor, &variant_path);
            sub_results.push(result);
        }
    }

    if let Some(any_of) = schema_obj.get("anyOf").and_then(|v| v.as_array()) {
        for (i, variant) in any_of.iter().enumerate() {
            let variant_path = if current_path.is_empty() {
                format!("anyOf[{}]", i)
            } else {
                format!("{}.anyOf[{}]", current_path, i)
            };
            let result = traverse_schema(variant, processor, &variant_path);
            sub_results.push(result);
        }
    }

    // Traverse allOf composition
    if let Some(all_of) = schema_obj.get("allOf").and_then(|v| v.as_array()) {
        for (i, composed) in all_of.iter().enumerate() {
            let composed_path = if current_path.is_empty() {
                format!("allOf[{}]", i)
            } else {
                format!("{}.allOf[{}]", current_path, i)
            };
            let result = traverse_schema(composed, processor, &composed_path);
            sub_results.push(result);
        }
    }

    processor.combine_results(sub_results)
}

/// Extract schema properties safely
pub fn get_schema_properties(schema: &Map<String, Value>) -> Option<&Map<String, Value>> {
    schema.get("properties").and_then(|p| p.as_object())
}

/// Extract schema items safely
pub fn get_schema_items(schema: &Map<String, Value>) -> Option<&Value> {
    schema.get("items")
}

/// Extract schema type safely
pub fn get_schema_type(schema: &Map<String, Value>) -> Option<&str> {
    schema.get("type").and_then(|t| t.as_str())
}

/// Extract enum values safely
pub fn get_enum_values(schema: &Map<String, Value>) -> Option<&Vec<Value>> {
    schema.get("enum").and_then(|e| e.as_array())
}

/// Extract integer constraints safely
pub fn get_integer_constraints(schema: &Map<String, Value>) -> (Option<i64>, Option<i64>) {
    let min = schema.get("minimum").and_then(|v| v.as_i64());
    let max = schema.get("maximum").and_then(|v| v.as_i64());
    (min, max)
}

/// Extract string pattern safely
pub fn get_string_pattern(schema: &Map<String, Value>) -> Option<&str> {
    schema.get("pattern").and_then(|p| p.as_str())
}

/// Escape one JSON Pointer path segment per RFC 6901.
pub fn escape_json_pointer_segment(segment: &str) -> String {
    segment.replace('~', "~0").replace('/', "~1")
}

/// Unescape one JSON Pointer path segment per RFC 6901.
///
/// Invalid escape sequences return `None` instead of silently resolving a
/// different path.
pub fn unescape_json_pointer_segment(segment: &str) -> Option<String> {
    let mut result = String::with_capacity(segment.len());
    let mut chars = segment.chars();

    while let Some(ch) = chars.next() {
        if ch != '~' {
            result.push(ch);
            continue;
        }

        match chars.next()? {
            '0' => result.push('~'),
            '1' => result.push('/'),
            _ => return None,
        }
    }

    Some(result)
}

/// Decode a local JSON Schema `$ref` into JSON Pointer path segments.
pub fn local_ref_segments(ref_path: &str) -> Option<Vec<String>> {
    if ref_path == "#" {
        return Some(Vec::new());
    }

    let stripped = ref_path.strip_prefix("#/")?;
    stripped
        .split('/')
        .map(unescape_json_pointer_segment)
        .collect()
}

/// Decode a local `#/definitions/...` reference target for type-name derivation.
pub fn local_definition_ref_target(ref_path: &str) -> Option<String> {
    let segments = local_ref_segments(ref_path)?;
    if segments.first().map(String::as_str) != Some("definitions") || segments.len() < 2 {
        return None;
    }
    Some(segments[1..].join("/"))
}

/// Resolve a local JSON Schema `$ref` against a root schema.
///
/// This mirrors the schema walker's supported reference semantics: `#` resolves
/// to the root object, `#/...` traverses object keys and array indices, and
/// external or missing references return `None`.
pub fn resolve_local_ref<'a>(root: &'a Value, ref_path: &str) -> Option<&'a Value> {
    if ref_path == "#" {
        return root.as_object().map(|_| root);
    }

    let mut current = root;

    for part in local_ref_segments(ref_path)? {
        if let Some(obj) = current.as_object()
            && let Some(next) = obj.get(&part)
        {
            current = next;
            continue;
        }

        if let Some(arr) = current.as_array()
            && let Ok(idx) = part.parse::<usize>()
            && let Some(next) = arr.get(idx)
        {
            current = next;
            continue;
        }

        return None;
    }

    current.as_object().map(|_| current)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    struct TestProcessor {
        count: usize,
    }

    impl SchemaTraversal for TestProcessor {
        type Result = usize;

        fn process_schema(&mut self, schema: &Map<String, Value>, _path: &str) -> Self::Result {
            if schema.contains_key("type") {
                self.count += 1;
            }
            1
        }

        fn combine_results(&self, results: Vec<Self::Result>) -> Self::Result {
            results.iter().sum()
        }
    }

    #[test]
    fn test_schema_traversal() {
        let schema = json!({
            "type": "object",
            "properties": {
                "name": {"type": "string"},
                "age": {"type": "integer"}
            }
        });

        let mut processor = TestProcessor { count: 0 };
        let result = traverse_schema(&schema, &mut processor, "");

        // Should traverse root + 2 properties = 3 schema objects
        assert_eq!(result, 3);
        assert_eq!(processor.count, 3); // All have type field
    }

    #[test]
    fn test_validate_schema_traversal_depth_accepts_exact_limit() {
        let schema = json!({
            "type": "object",
            "properties": {
                "name": { "type": "string" }
            }
        });

        assert!(validate_schema_traversal_depth(&schema, 4).is_ok());
    }

    #[test]
    fn test_validate_schema_traversal_depth_rejects_over_limit_with_path() {
        let schema = json!({
            "type": "object",
            "properties": {
                "user": {
                    "type": "object",
                    "properties": {
                        "name": { "type": "string" }
                    }
                }
            }
        });

        let err = validate_schema_traversal_depth(&schema, 5).unwrap_err();
        let err_text = err.to_string();
        assert!(
            err_text.contains("schema traversal depth 6 exceeds configured maximum 5"),
            "unexpected error: {}",
            err_text
        );
        assert!(
            err_text.contains("#/properties/user/properties/name"),
            "unexpected error path: {}",
            err_text
        );
    }

    #[test]
    fn test_schema_utilities() {
        let schema_obj = json!({
            "type": "object",
            "properties": {"test": {"type": "string"}},
            "items": {"type": "number"}
        })
        .as_object()
        .unwrap()
        .clone();

        assert_eq!(get_schema_type(&schema_obj), Some("object"));
        assert!(get_schema_properties(&schema_obj).is_some());
        assert!(get_schema_items(&schema_obj).is_some());
        assert!(get_enum_values(&schema_obj).is_none());
    }

    #[test]
    fn test_resolve_local_ref_root_nested_and_indexed_paths() {
        let schema = json!({
            "type": "object",
            "properties": {
                "user": {
                    "type": "object",
                    "properties": {
                        "name": {"type": "string"}
                    }
                }
            },
            "oneOf": [
                {"type": "boolean"},
                {"enum": ["a", "b"]}
            ]
        });

        assert_eq!(
            resolve_local_ref(&schema, "#")
                .and_then(Value::as_object)
                .and_then(|obj| obj.get("type"))
                .and_then(Value::as_str),
            Some("object")
        );
        assert_eq!(
            resolve_local_ref(&schema, "#/properties/user/properties/name")
                .and_then(Value::as_object)
                .and_then(|obj| obj.get("type"))
                .and_then(Value::as_str),
            Some("string")
        );
        assert_eq!(
            resolve_local_ref(&schema, "#/oneOf/1")
                .and_then(Value::as_object)
                .and_then(|obj| obj.get("enum"))
                .and_then(Value::as_array)
                .map(Vec::len),
            Some(2)
        );
    }

    #[test]
    fn test_json_pointer_segment_escape_round_trip() {
        assert_eq!(escape_json_pointer_segment("a/b~c"), "a~1b~0c");
        assert_eq!(
            unescape_json_pointer_segment("a~1b~0c"),
            Some("a/b~c".to_string())
        );
        assert_eq!(unescape_json_pointer_segment("~01"), Some("~1".to_string()));
        assert_eq!(unescape_json_pointer_segment("bad~2escape"), None);
        assert_eq!(unescape_json_pointer_segment("bad~"), None);
    }

    #[test]
    fn test_local_ref_segments_decodes_escaped_pointer_parts() {
        assert_eq!(
            local_ref_segments("#/properties/a~1b/definitions/tilde~0name"),
            Some(vec![
                "properties".to_string(),
                "a/b".to_string(),
                "definitions".to_string(),
                "tilde~name".to_string(),
            ])
        );
        assert_eq!(local_ref_segments("#"), Some(Vec::new()));
        assert_eq!(local_ref_segments("https://example.com/schema.json"), None);
        assert_eq!(local_ref_segments("#/properties/bad~2escape"), None);
    }

    #[test]
    fn test_local_definition_ref_target_decodes_escaped_names() {
        assert_eq!(
            local_definition_ref_target("#/definitions/a~1b~0c"),
            Some("a/b~c".to_string())
        );
        assert_eq!(local_definition_ref_target("#/properties/a~1b"), None);
    }

    #[test]
    fn test_resolve_local_ref_decodes_escaped_pointer_segments() {
        let schema = json!({
            "type": "object",
            "properties": {
                "a/b": {"type": "string"},
                "tilde~key": {"type": "integer"},
                "~1": {"type": "boolean"},
                "": {"type": "null"}
            }
        });

        assert_eq!(
            resolve_local_ref(&schema, "#/properties/a~1b")
                .and_then(Value::as_object)
                .and_then(|obj| obj.get("type"))
                .and_then(Value::as_str),
            Some("string")
        );
        assert_eq!(
            resolve_local_ref(&schema, "#/properties/tilde~0key")
                .and_then(Value::as_object)
                .and_then(|obj| obj.get("type"))
                .and_then(Value::as_str),
            Some("integer")
        );
        assert_eq!(
            resolve_local_ref(&schema, "#/properties/~01")
                .and_then(Value::as_object)
                .and_then(|obj| obj.get("type"))
                .and_then(Value::as_str),
            Some("boolean")
        );
        assert_eq!(
            resolve_local_ref(&schema, "#/properties/")
                .and_then(Value::as_object)
                .and_then(|obj| obj.get("type"))
                .and_then(Value::as_str),
            Some("null")
        );
        assert!(resolve_local_ref(&schema, "#/properties/a~2b").is_none());
    }

    #[test]
    fn test_resolve_local_ref_rejects_external_missing_and_scalar_targets() {
        let schema = json!({
            "type": "object",
            "properties": {
                "name": {"type": "string"}
            }
        });

        assert!(resolve_local_ref(&schema, "https://example.com/schema.json").is_none());
        assert!(resolve_local_ref(&schema, "#/properties/missing").is_none());
        assert!(resolve_local_ref(&schema, "#/type").is_none());
    }
}
