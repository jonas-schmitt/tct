//! JSON const wrapper renderer for generated type aliases.
//!
//! This module emits zero-sized wrapper structs for schemas whose value is
//! fixed by a JSON `const`. The generated serde implementation accepts only
//! the embedded const value, while `JsonTokenEncoder` emits and consumes no
//! payload tokens because the schema itself determines the value. These wrappers
//! let const-valued fields participate in ordinary Rust types, validation, and
//! token-count accounting without allocating runtime representation state.

use crate::TctResult;
use crate::generator::generation_output::TypeCodeGeneration;
use crate::generator::schema_discovery::TypeDiscovery;
use quote::{format_ident, quote};
use serde_json::Value;

pub(super) fn generate_json_const_wrapper(
    type_discovery: &TypeDiscovery,
    const_value: &Value,
) -> TctResult<TypeCodeGeneration> {
    let type_name = &type_discovery.type_name;
    let type_ident = format_ident!("{}", type_name);
    let const_json =
        serde_json::to_string(const_value).expect("failed to serialize const schema value");

    crate::log_debug!(
        "🔧 Generating JSON const wrapper for '{}' (const={})",
        type_name,
        const_json
    );

    let definition_tokens = quote! {
        #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
        pub struct #type_ident;
    };

    let implementation_tokens = quote! {
        impl #type_ident {
            pub const VALUE_JSON: &'static str = #const_json;

            /// Create the only valid value for this const schema.
            pub fn new() -> Self {
                Self
            }

            /// Get the schema const value.
            pub fn value(&self) -> serde_json::Value {
                Self::const_value()
            }

            /// Get the schema const value while consuming the wrapper.
            pub fn into_value(self) -> serde_json::Value {
                Self::const_value()
            }

            fn const_value() -> serde_json::Value {
                serde_json::from_str(Self::VALUE_JSON)
                    .expect("embedded JSON const value must be valid")
            }
        }

        impl serde::Serialize for #type_ident {
            fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
            where
                S: serde::Serializer,
            {
                let value = Self::const_value();
                serde::Serialize::serialize(&value, serializer)
            }
        }

        impl<'de> serde::Deserialize<'de> for #type_ident {
            fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                let value =
                    <serde_json::Value as serde::Deserialize>::deserialize(deserializer)?;
                let expected = Self::const_value();
                if value == expected {
                    Ok(Self)
                } else {
                    Err(serde::de::Error::custom(format!(
                        "expected const JSON {}, got {}",
                        expected,
                        value
                    )))
                }
            }
        }

        impl crate::tokenizer::JsonTokenEncoder for #type_ident {
            fn encode(&self, _tokens: &mut Vec<u32>) {
                // JSON const: no payload tokens are needed.
            }

            fn decode(_tokens: &mut impl Iterator<Item = u32>) -> Result<Self, crate::tokenizer::DecodeError> {
                Ok(Self)
            }

            fn token_count(&self) -> usize {
                0
            }
        }
    };

    let type_generation =
        TypeCodeGeneration::new_struct(type_name.clone(), definition_tokens, implementation_tokens)
            .with_token_encoder_impl(true);

    crate::log_debug!("✅ JSON const wrapper '{}' generated", type_name);
    Ok(type_generation)
}
