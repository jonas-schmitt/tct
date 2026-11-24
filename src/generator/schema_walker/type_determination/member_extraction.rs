//! Inline member-schema extraction helpers for type determination.
//!
//! This module owns the cases where a member schema becomes a named discovered
//! type instead of an inline Rust primitive: string enums, JSON const wrappers,
//! `type` arrays (also next to a `$ref`), bounded integers, and
//! format-constrained strings, including
//! their schema locations and discovery statistics.

use super::{primitive_type_for_const_value, type_array_type_names};
use crate::TctResult;
use crate::generator::schema_discovery::{
    MemberInfo, MemberType, SchemaLocation, TypeDiscovery, TypeKind,
};
use crate::generator::schema_walker::SchemaWalker;
use crate::schema_utils::local_definition_ref_target;
use crate::utils::to_upper_camel_case;
use serde_json::{Map, Value};

const SUPPORTED_FORMAT_STRINGS: &[&str] = &[
    "int32",
    "int64",
    "double",
    "float",
    "byte",
    "date-time",
    "datetime",
    "date",
    "time",
    "duration",
    "uuid",
    "email",
    "idn-email",
    "uri",
    "uri-reference",
    "uri-template",
    "iri",
    "iri-reference",
    "hostname",
    "idn-hostname",
    "ipv4",
    "ipv6",
    "json-pointer",
    "relative-json-pointer",
    "regex",
];

impl SchemaWalker {
    pub(super) fn extract_inline_string_enum_type(
        &mut self,
        schema: &Map<String, Value>,
        enum_values: &[Value],
        parent_type: &str,
        property_name: Option<&str>,
        description: &str,
    ) -> TctResult<MemberType> {
        let inline_enum_name = if let Some(prop_name) = property_name {
            to_upper_camel_case(&format!("{}_{}", parent_type, prop_name))
        } else {
            format!("{}InlineEnum", parent_type)
        };

        crate::log_debug!(
            "🔍 Extracting inline enum '{}' with {} variants for property '{}'",
            inline_enum_name,
            enum_values.len(),
            property_name.unwrap_or("unnamed")
        );

        let enum_schema_location = SchemaLocation {
            json_path: property_name
                .map(|p| format!("#/{}", p))
                .unwrap_or_else(|| "#/inline_enum".to_string()),
            description: format!(
                "{} for property '{}'",
                description,
                property_name.unwrap_or("unnamed")
            ),
            parent_type: Some(parent_type.to_string()),
            property_name: property_name.map(String::from),
        };

        let mut enum_type_discovery = TypeDiscovery::new(
            inline_enum_name.clone(),
            TypeKind::Enum,
            enum_schema_location,
            schema.clone(),
        );

        self.discover_simple_enum_variants(enum_values, &mut enum_type_discovery)?;
        self.discovery.register_type(enum_type_discovery);
        self.discovery.stats.enums += 1;

        crate::log_debug!(
            "   ✅ Inline enum '{}' extracted with {} variants",
            inline_enum_name,
            enum_values.len()
        );

        Ok(MemberType::TypeReference(inline_enum_name))
    }

    pub(super) fn extract_inline_json_const_type(
        &mut self,
        schema: &Map<String, Value>,
        parent_type: &str,
        property_name: Option<&str>,
    ) -> TctResult<MemberType> {
        let inline_const_name = if let Some(prop_name) = property_name {
            to_upper_camel_case(&format!("{}_{}_const", parent_type, prop_name))
        } else {
            format!("{}Const", parent_type)
        };

        crate::log_debug!(
            "🔍 Extracting inline JSON const '{}' for property '{}'",
            inline_const_name,
            property_name.unwrap_or("unnamed")
        );

        let const_schema_location = SchemaLocation {
            json_path: property_name
                .map(|p| format!("#/{}", p))
                .unwrap_or_else(|| "#/inline_const".to_string()),
            description: format!(
                "JSON const for property '{}'",
                property_name.unwrap_or("unnamed")
            ),
            parent_type: Some(parent_type.to_string()),
            property_name: property_name.map(String::from),
        };

        let const_type_discovery = TypeDiscovery::new(
            inline_const_name.clone(),
            TypeKind::PrimitiveWrapper {
                primitive_type: schema
                    .get("const")
                    .map(primitive_type_for_const_value)
                    .unwrap_or_else(|| "serde_json::Value".to_string()),
            },
            const_schema_location,
            schema.clone(),
        );

        self.discovery.register_type(const_type_discovery);

        crate::log_debug!("   ✅ Inline JSON const '{}' extracted", inline_const_name);

        Ok(MemberType::TypeReference(inline_const_name))
    }

    /// A member whose `type` is an array of JSON types.
    ///
    /// A one-element array is the plain type. A longer array becomes a
    /// generated enum with one variant per listed type, discovered the same way
    /// as a definition with a `type` array: each variant applies the keywords
    /// of its type (properties, items, bounds, format, and the `enum` values of
    /// that type), and `null` is a unit variant. Nullability is part of the
    /// enum, so a required `["integer", "null"]` member still accepts null.
    pub(super) fn extract_type_array_member_type(
        &mut self,
        schema: &Map<String, Value>,
        type_array: &[Value],
        parent_type: &str,
        property_name: Option<&str>,
    ) -> TctResult<MemberType> {
        let json_path = self.build_json_path(Some(parent_type), property_name);
        let type_names = type_array_type_names(type_array, &json_path)?;

        if let [single_type] = type_names.as_slice() {
            let mut single_type_schema = schema.clone();
            single_type_schema.insert("type".to_string(), Value::String(single_type.to_string()));
            return self.determine_member_type(&single_type_schema, parent_type, property_name);
        }

        let enum_name = self.type_array_enum_name(schema, parent_type, property_name);

        crate::log_debug!(
            "🔍 Type array {:?} for property '{}' in '{}' → Enum '{}'",
            type_names,
            property_name.unwrap_or("unnamed"),
            parent_type,
            enum_name
        );

        self.discover_schema_as_type(
            schema,
            &enum_name,
            Some(parent_type.to_string()),
            property_name.map(String::from),
        )?;
        Ok(MemberType::TypeReference(enum_name))
    }

    /// The enum name of a member with a `type` array: `{Parent}{Prop}Enum`, or
    /// `{Parent}TypeArrayEnum` for a member without a property name.
    fn type_array_enum_name(
        &self,
        schema: &Map<String, Value>,
        parent_type: &str,
        property_name: Option<&str>,
    ) -> String {
        let base_name = match property_name {
            Some(prop_name) => format!(
                "{}Enum",
                self.create_nested_type_name(parent_type, Some(prop_name))
            ),
            None => format!("{}TypeArrayEnum", parent_type),
        };
        self.unused_or_same_schema_type_name(base_name, schema)
    }

    /// The types a `$ref` member lists next to the reference, when they are
    /// honoured: `{"$ref": "#/definitions/rule", "type": ["object", "boolean"]}`
    /// where `rule` is an object-shaped definition without a `type` of its own.
    ///
    /// Such a definition compiles to a struct, which holds only objects, while
    /// the definition itself does not restrict the instance type. The sibling
    /// `type` array then names the types the member takes: the struct for
    /// `object`, and a plain variant for each other listed type. A sibling
    /// `type` next to any other reference is not used; the reference alone
    /// decides the member type.
    pub(in crate::generator::schema_walker) fn ref_sibling_type_names<'a>(
        &self,
        schema: &'a Map<String, Value>,
    ) -> Option<Vec<&'a str>> {
        let ref_str = schema.get("$ref").and_then(Value::as_str)?;
        local_definition_ref_target(ref_str)?;
        let type_names: Vec<&str> = schema
            .get("type")
            .and_then(Value::as_array)?
            .iter()
            .filter_map(Value::as_str)
            .collect();
        if type_names.len() < 2 || !type_names.contains(&"object") {
            return None;
        }
        let target = self.resolve_ref(ref_str)?;
        let object_shaped = ["properties", "patternProperties", "additionalProperties"]
            .iter()
            .any(|key| target.contains_key(*key));
        let untyped = [
            "type", "$ref", "enum", "const", "oneOf", "anyOf", "allOf", "not", "if",
        ]
        .iter()
        .all(|key| !target.contains_key(*key));
        (object_shaped && untyped).then_some(type_names)
    }

    /// A `$ref` member whose sibling `type` array is honoured (see
    /// [`Self::ref_sibling_type_names`]): an enum with the referenced struct as
    /// the `object` variant and one plain variant per other listed type.
    pub(super) fn try_extract_ref_with_type_array_member_type(
        &mut self,
        schema: &Map<String, Value>,
        parent_type: &str,
        property_name: Option<&str>,
    ) -> TctResult<Option<MemberType>> {
        let Some(sibling_type_names) = self.ref_sibling_type_names(schema) else {
            return Ok(None);
        };
        let Some(ref_value) = schema.get("$ref") else {
            return Ok(None);
        };
        let json_path = self.build_json_path(Some(parent_type), property_name);
        // Checks the names against the seven JSON Schema type names.
        let listed: Vec<Value> = sibling_type_names
            .iter()
            .map(|name| Value::String((*name).to_string()))
            .collect();
        let type_names = type_array_type_names(&listed, &json_path)?;

        let enum_name = self.type_array_enum_name(schema, parent_type, property_name);
        if self
            .discovery
            .discovered_types
            .get(&enum_name)
            .is_some_and(|existing| existing.schema_object == *schema)
        {
            return Ok(Some(MemberType::TypeReference(enum_name)));
        }

        crate::log_debug!(
            "🔍 $ref with type array {:?} for property '{}' in '{}' → Enum '{}'",
            type_names,
            property_name.unwrap_or("unnamed"),
            parent_type,
            enum_name
        );

        let mut enum_type_discovery = TypeDiscovery::new(
            enum_name.clone(),
            TypeKind::Enum,
            SchemaLocation {
                json_path,
                description: format!("Type {} from schema", enum_name),
                parent_type: Some(parent_type.to_string()),
                property_name: property_name.map(String::from),
            },
            schema.clone(),
        );
        for type_name in type_names {
            let variant_schema = if type_name == "object" {
                Map::from_iter([("$ref".to_string(), ref_value.clone())])
            } else {
                Map::from_iter([("type".to_string(), Value::String(type_name.to_string()))])
            };
            let variant_name = format!("Variant{}", enum_type_discovery.members.len());
            let variant_parent_name = format!("{}{}", enum_name, variant_name);
            let member_type =
                self.determine_member_type(&variant_schema, &variant_parent_name, None)?;
            enum_type_discovery.add_member(MemberInfo {
                name: variant_name,
                member_type,
                required: true,
                is_nullable: false, // Enum variants are never nullable
                schema: variant_schema,
            });
            self.discovery.stats.total_enum_variants += 1;
        }
        self.discovery.register_type(enum_type_discovery);

        Ok(Some(MemberType::TypeReference(enum_name)))
    }

    /// `base_name`, or `base_name` with a numeric suffix when a type with a
    /// different schema was already discovered under that name. Member types
    /// without a property name (array items, map values) share their parent's
    /// name, so two of them in one parent need distinct names, and two
    /// property names can give one type name (`mode` as an enum and
    /// `modeEnum` as an object are both `{Parent}ModeEnum`). Without the
    /// suffix the later type would replace the earlier one under its name.
    /// Discovering the same schema again keeps its name.
    pub(super) fn unused_or_same_schema_type_name(
        &self,
        base_name: String,
        schema: &Map<String, Value>,
    ) -> String {
        let mut candidate = base_name.clone();
        let mut suffix = 2usize;
        while self
            .discovery
            .discovered_types
            .get(&candidate)
            .is_some_and(|existing| existing.schema_object != *schema)
        {
            candidate = format!("{base_name}{suffix}");
            suffix += 1;
        }
        candidate
    }

    pub(super) fn try_extract_bounded_integer_type(
        &mut self,
        schema: &Map<String, Value>,
        parent_type: &str,
        property_name: Option<&str>,
    ) -> Option<MemberType> {
        if schema.get("type").and_then(Value::as_str) != Some("integer") {
            return None;
        }

        let (Some(min), Some(max)) = (
            schema.get("minimum").and_then(Value::as_i64),
            schema.get("maximum").and_then(Value::as_i64),
        ) else {
            return None;
        };

        let bounded_int_name = if let Some(prop_name) = property_name {
            to_upper_camel_case(&format!("{}_{}", parent_type, prop_name))
        } else {
            format!("{}BoundedInt", parent_type)
        };
        let range = max.saturating_sub(min).saturating_add(1);

        crate::log_debug!(
            "🔍 Extracting bounded integer '{}' with range [{}, {}] (size: {}) for property '{}'",
            bounded_int_name,
            min,
            max,
            range,
            property_name.unwrap_or("unnamed")
        );

        let int_schema_location = SchemaLocation {
            json_path: property_name
                .map(|p| format!("#/{}", p))
                .unwrap_or_else(|| "#/bounded_int".to_string()),
            description: format!(
                "Bounded integer [{}, {}] for property '{}'",
                min,
                max,
                property_name.unwrap_or("unnamed")
            ),
            parent_type: Some(parent_type.to_string()),
            property_name: property_name.map(String::from),
        };

        let int_type_discovery = TypeDiscovery::new(
            bounded_int_name.clone(),
            TypeKind::PrimitiveWrapper {
                primitive_type: "i64".to_string(),
            },
            int_schema_location,
            schema.clone(),
        );

        self.discovery.register_type(int_type_discovery);
        self.discovery.stats.bounded_integers += 1;

        crate::log_debug!(
            "   ✅ Bounded integer '{}' extracted with range [{}, {}]",
            bounded_int_name,
            min,
            max
        );

        Some(MemberType::TypeReference(bounded_int_name))
    }

    pub(super) fn try_extract_format_string_type(
        &mut self,
        schema: &Map<String, Value>,
        parent_type: &str,
        property_name: Option<&str>,
    ) -> Option<MemberType> {
        if schema.get("type").and_then(Value::as_str) != Some("string") {
            return None;
        }
        let format_str = schema.get("format").and_then(Value::as_str)?;
        if !SUPPORTED_FORMAT_STRINGS.contains(&format_str) {
            return None;
        }

        let format_string_name = if let Some(prop_name) = property_name {
            to_upper_camel_case(&format!("{}_{}", parent_type, prop_name))
        } else {
            format!("{}FormatString", parent_type)
        };

        crate::log_debug!(
            "🔍 Extracting format-constrained string '{}' (format: '{}') for property '{}'",
            format_string_name,
            format_str,
            property_name.unwrap_or("unnamed")
        );

        let format_schema_location = SchemaLocation {
            json_path: property_name
                .map(|p| format!("#/{}", p))
                .unwrap_or_else(|| "#/format_string".to_string()),
            description: format!(
                "Format-constrained string (format: '{}') for property '{}'",
                format_str,
                property_name.unwrap_or("unnamed")
            ),
            parent_type: Some(parent_type.to_string()),
            property_name: property_name.map(String::from),
        };

        let format_type_discovery = TypeDiscovery::new(
            format_string_name.clone(),
            TypeKind::PrimitiveWrapper {
                primitive_type: "String".to_string(),
            },
            format_schema_location,
            schema.clone(),
        );

        self.discovery.register_type(format_type_discovery);
        self.discovery.stats.format_constrained_strings += 1;

        crate::log_debug!(
            "   ✅ Format-constrained string '{}' extracted (format: '{}')",
            format_string_name,
            format_str
        );

        Some(MemberType::TypeReference(format_string_name))
    }
}
