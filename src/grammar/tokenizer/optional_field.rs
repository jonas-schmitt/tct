//! Generated three-state optional-field support for standalone tokenizer output.
//!
//! JSON Schema optional fields need to distinguish absent properties, explicit
//! `null`, and present values. This renderer keeps that support isolated from
//! the primitive and collection renderers while preserving the same generated
//! tokenizer module surface.

use proc_macro2::TokenStream;
use quote::quote;

pub(super) fn render_optional_field_type() -> TokenStream {
    quote! {
        /// Three-state optional field type for nullable fields in JSON schemas
        #[derive(Debug, Clone, PartialEq, Eq, Hash, Default)]
        pub enum OptionalField<T> {
            #[default]
            Absent,
            Null,
            Present(T),
        }

        impl<T> OptionalField<T> {
            pub fn is_absent(&self) -> bool {
                matches!(self, OptionalField::Absent)
            }
            pub fn is_null(&self) -> bool {
                matches!(self, OptionalField::Null)
            }
            pub fn is_present(&self) -> bool {
                matches!(self, OptionalField::Present(_))
            }
        }

        impl<T: serde::Serialize> serde::Serialize for OptionalField<T> {
            fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
            where
                S: serde::Serializer,
            {
                match self {
                    OptionalField::Absent => serializer.serialize_none(),
                    OptionalField::Null => {
                        use serde_json::Value;
                        Value::Null.serialize(serializer)
                    }
                    OptionalField::Present(value) => value.serialize(serializer),
                }
            }
        }

        impl<'de, T: serde::Deserialize<'de>> serde::Deserialize<'de> for OptionalField<T> {
            fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                let opt: Option<T> = Option::deserialize(deserializer)?;
                match opt {
                    Some(value) => Ok(OptionalField::Present(value)),
                    None => Ok(OptionalField::Null),
                }
            }
        }

        pub fn deserialize_option_reject_null<'de, D, T>(deserializer: D) -> Result<Option<T>, D::Error>
        where
            D: serde::Deserializer<'de>,
            T: serde::Deserialize<'de>,
        {
            let value = <serde_json::Value as serde::Deserialize>::deserialize(deserializer)?;
            if value.is_null() {
                return Err(serde::de::Error::custom(
                    "explicit null is not allowed for non-nullable optional field",
                ));
            }
            <T as serde::Deserialize>::deserialize(value)
                .map(Some)
                .map_err(serde::de::Error::custom)
        }

        impl<T: JsonTokenEncoder> JsonTokenEncoder for OptionalField<T> {
            fn encode(&self, tokens: &mut Vec<u32>) {
                match self {
                    OptionalField::Absent => { tokens.push(0); }
                    OptionalField::Null => { tokens.push(1); }
                    OptionalField::Present(value) => {
                        tokens.push(2);
                        value.encode(tokens);
                    }
                }
            }
            fn decode(tokens: &mut impl Iterator<Item = u32>) -> Result<Self, DecodeError> {
                let state_flag = tokens.next().ok_or(DecodeError::UnexpectedEndOfStream {
                    context: EOFContext::new("OptionalField<T>", None, "state flag (0=absent, 1=null, 2=present)")
                })?;
                match state_flag % 3 {
                    0 => Ok(OptionalField::Absent),
                    1 => Ok(OptionalField::Null),
                    2 => {
                        let value = T::decode(tokens)?;
                        Ok(OptionalField::Present(value))
                    }
                    _ => unreachable!("Modulo 3 ensures valid range 0-2"),
                }
            }
            fn decode_strict(tokens: &mut impl Iterator<Item = u32>) -> Result<Self, DecodeError> {
                let state_flag = tokens.next().ok_or(DecodeError::UnexpectedEndOfStream {
                    context: EOFContext::new("OptionalField<T>", None, "state flag (0=absent, 1=null, 2=present)")
                })?;
                match state_flag {
                    0 => Ok(OptionalField::Absent),
                    1 => Ok(OptionalField::Null),
                    2 => {
                        let value = T::decode_strict(tokens)?;
                        Ok(OptionalField::Present(value))
                    }
                    token => Err(DecodeError::InvalidTokenInContext {
                        token,
                        context: EOFContext::new("OptionalField<T>", None, "state flag 0, 1, or 2"),
                    }),
                }
            }
            fn token_count(&self) -> usize {
                match self {
                    OptionalField::Absent => 1,
                    OptionalField::Null => 1,
                    OptionalField::Present(value) => 1 + value.token_count(),
                }
            }
        }
    }
}
