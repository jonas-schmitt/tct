//! Inline member-schema extraction helpers for type determination.
//!
//! This module owns the cases where a member schema becomes a named discovered
//! type instead of an inline Rust primitive: string enums, JSON const wrappers,
//! bounded integers, and format-constrained strings, including their schema
//! locations and discovery statistics.

use super::primitive_type_for_const_value;
use crate::TctResult;
use crate::generator::schema_discovery::{MemberType, SchemaLocation, TypeDiscovery, TypeKind};
use crate::generator::schema_walker::SchemaWalker;
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
