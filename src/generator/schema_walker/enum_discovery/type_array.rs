//! Type-array variant discovery: one enum variant per JSON type listed in
//! `"type": [...]`, shared by definitions and members.

use crate::generator::schema_discovery::{MemberInfo, TypeDiscovery};
use crate::generator::schema_walker::SchemaWalker;
use crate::generator::schema_walker::type_determination::{
    json_value_has_type, type_array_type_names,
};
use crate::{TctError, TctResult};
use serde_json::{Map, Value};

impl SchemaWalker {
    /// Discover type array variants: {"type": ["object", "string"]}
    /// Per JSON Schema spec, type can be an array of type strings
    ///
    /// Each listed type becomes one variant whose schema is the parent schema
    /// with that single type, so the keywords of that type (properties, items,
    /// bounds, format) apply to it. An `enum` keeps, per variant, only the
    /// values of that variant's type: `["string", "null"]` with
    /// `"enum": ["a", null]` is a string enum of `a` plus a null variant. A
    /// listed type with no value in a non-empty `enum` admits no instance and
    /// gets no variant.
    pub(super) fn discover_type_array_variants(
        &mut self,
        type_array: &[Value],
        schema: &Map<String, Value>,
        type_discovery: &mut TypeDiscovery,
    ) -> TctResult<()> {
        crate::log_debug!(
            "   🔍 Processing type array with {} types for '{}'",
            type_array.len(),
            type_discovery.type_name
        );

        let json_path = type_discovery.schema_location.json_path.clone();
        let type_names = type_array_type_names(type_array, &json_path)?;
        let enum_values = schema
            .get("enum")
            .and_then(Value::as_array)
            .filter(|values| !values.is_empty());

        for type_str in type_names {
            // Create a schema for this specific type
            // Inherit all properties from parent schema except "type"
            let mut variant_schema = schema.clone();
            variant_schema.insert("type".to_string(), Value::String(type_str.to_string()));

            if let Some(values) = enum_values {
                let admitted: Vec<Value> = values
                    .iter()
                    .filter(|value| json_value_has_type(value, type_str))
                    .cloned()
                    .collect();
                if admitted.is_empty() {
                    crate::log_debug!(
                        "     • Type '{}' has no value in the enum; no variant",
                        type_str
                    );
                    continue;
                }
                variant_schema.insert("enum".to_string(), Value::Array(admitted));
            }

            let index = type_discovery.members.len();
            crate::log_debug!("     • Type array element {}: '{}'", index, type_str);
            let variant_name = format!("Variant{}", index);

            // Use variant-specific parent name to generate unique nested types
            // This prevents different variants from overwriting each other
            let variant_parent_name = format!("{}{}", type_discovery.type_name, variant_name);

            // Use determine_member_type to properly discover nested types
            // This handles objects, arrays, and all other types consistently
            let member_type =
                self.determine_member_type(&variant_schema, &variant_parent_name, None)?;
            crate::log_debug!("     ✅ Variant {} member type: {:?}", index, member_type);

            let member_info = MemberInfo {
                name: variant_name.clone(),
                member_type,
                required: true,
                is_nullable: false, // Enum variants are never nullable
                schema: variant_schema,
            };

            type_discovery.add_member(member_info);
            self.discovery.stats.total_enum_variants += 1;
            crate::log_debug!(
                "     ✅ Type array variant '{}' added successfully",
                variant_name
            );
        }

        if type_discovery.members.is_empty() {
            return Err(TctError::schema_validation(
                format!(
                    "the enum of '{}' contains no value of the types listed in `type` ({})",
                    type_discovery.type_name,
                    type_array
                        .iter()
                        .map(Value::to_string)
                        .collect::<Vec<_>>()
                        .join(", ")
                ),
                json_path,
                "No instance can satisfy both `type` and `enum`; list the enum values' types in `type` or fix the enum",
            ));
        }
        Ok(())
    }
}
