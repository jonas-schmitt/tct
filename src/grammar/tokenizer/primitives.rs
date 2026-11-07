//! Rendering for generated primitive tokenizer implementations.

use proc_macro2::TokenStream;
use quote::quote;

pub(super) fn render_primitive_impls() -> TokenStream {
    quote! {
        #[inline(always)]
        fn decode_strict_byte_token(
            tokens: &mut impl Iterator<Item = u32>,
            type_name: &'static str,
        ) -> Result<u8, DecodeError> {
            let byte_token = tokens.next().ok_or(DecodeError::UnexpectedEndOfStream {
                context: EOFContext::new(type_name, Some("byte"), "byte value"),
            })?;
            if byte_token <= u8::MAX as u32 {
                Ok(byte_token as u8)
            } else {
                Err(DecodeError::InvalidTokenInContext {
                    token: byte_token,
                    context: EOFContext::new(type_name, Some("byte"), "byte token 0..=255"),
                })
            }
        }

        impl JsonTokenEncoder for bool {
            fn encode(&self, tokens: &mut Vec<u32>) {
                tokens.push(if *self { 1 } else { 0 });
            }
            fn decode(tokens: &mut impl Iterator<Item = u32>) -> Result<Self, DecodeError> {
                match tokens.next() {
                    Some(token) => Ok((token % 2) == 1),
                    None => Err(DecodeError::UnexpectedEndOfStream {
                        context: EOFContext::new("bool", None, "boolean token")
                    }),
                }
            }
            fn decode_strict(tokens: &mut impl Iterator<Item = u32>) -> Result<Self, DecodeError> {
                match tokens.next() {
                    Some(0) => Ok(false),
                    Some(1) => Ok(true),
                    Some(token) => Err(DecodeError::InvalidTokenInContext {
                        token,
                        context: EOFContext::new("bool", None, "boolean token 0 or 1"),
                    }),
                    None => Err(DecodeError::UnexpectedEndOfStream {
                        context: EOFContext::new("bool", None, "boolean token")
                    }),
                }
            }
            fn token_count(&self) -> usize { 1 }
        }

        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
        enum JsonIntegerRepr {
            Signed(i64),
            Unsigned(u64),
        }

        /// Lossless JSON Schema `integer` value used for unbounded generated integers.
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
        pub struct JsonInteger(JsonIntegerRepr);

        impl JsonInteger {
            pub fn from_i64(value: i64) -> Self {
                Self(JsonIntegerRepr::Signed(value))
            }

            pub fn from_u64(value: u64) -> Self {
                if let Ok(signed) = i64::try_from(value) {
                    Self::from_i64(signed)
                } else {
                    Self(JsonIntegerRepr::Unsigned(value))
                }
            }

            pub fn from_number(number: &serde_json::Number) -> Option<Self> {
                if let Some(value) = number.as_i64() {
                    Some(Self::from_i64(value))
                } else {
                    number.as_u64().map(Self::from_u64)
                }
            }

            pub fn to_number(self) -> serde_json::Number {
                match self.0 {
                    JsonIntegerRepr::Signed(value) => serde_json::Number::from(value),
                    JsonIntegerRepr::Unsigned(value) => serde_json::Number::from(value),
                }
            }

            pub fn as_i64(self) -> Option<i64> {
                match self.0 {
                    JsonIntegerRepr::Signed(value) => Some(value),
                    JsonIntegerRepr::Unsigned(value) => i64::try_from(value).ok(),
                }
            }

            pub fn as_u64(self) -> Option<u64> {
                match self.0 {
                    JsonIntegerRepr::Signed(value) => u64::try_from(value).ok(),
                    JsonIntegerRepr::Unsigned(value) => Some(value),
                }
            }
        }

        impl Default for JsonInteger {
            fn default() -> Self {
                Self::from_i64(0)
            }
        }

        impl From<i64> for JsonInteger {
            fn from(value: i64) -> Self {
                Self::from_i64(value)
            }
        }

        impl From<u64> for JsonInteger {
            fn from(value: u64) -> Self {
                Self::from_u64(value)
            }
        }

        impl PartialOrd for JsonInteger {
            fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
                Some(self.cmp(other))
            }
        }

        impl Ord for JsonInteger {
            fn cmp(&self, other: &Self) -> std::cmp::Ordering {
                let lhs = match self.0 {
                    JsonIntegerRepr::Signed(value) => i128::from(value),
                    JsonIntegerRepr::Unsigned(value) => i128::from(value),
                };
                let rhs = match other.0 {
                    JsonIntegerRepr::Signed(value) => i128::from(value),
                    JsonIntegerRepr::Unsigned(value) => i128::from(value),
                };
                lhs.cmp(&rhs)
            }
        }

        impl serde::Serialize for JsonInteger {
            fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
            where
                S: serde::Serializer,
            {
                serde::Serialize::serialize(&self.to_number(), serializer)
            }
        }

        impl<'de> serde::Deserialize<'de> for JsonInteger {
            fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                let number = <serde_json::Number as serde::Deserialize>::deserialize(deserializer)?;
                Self::from_number(&number)
                    .ok_or_else(|| serde::de::Error::custom("expected JSON integer"))
            }
        }

        impl JsonTokenEncoder for JsonInteger {
            fn encode(&self, tokens: &mut Vec<u32>) {
                match self.0 {
                    JsonIntegerRepr::Signed(value) => {
                        tokens.push(0);
                        value.encode(tokens);
                    }
                    JsonIntegerRepr::Unsigned(value) => {
                        tokens.push(1);
                        value.encode(tokens);
                    }
                }
            }

            fn decode(tokens: &mut impl Iterator<Item = u32>) -> Result<Self, DecodeError> {
                let marker = tokens.next().ok_or(DecodeError::UnexpectedEndOfStream {
                    context: EOFContext::new("JsonInteger", Some("marker"), "signed/unsigned marker")
                })?;
                if marker % 2 == 0 {
                    i64::decode(tokens).map(Self::from_i64)
                } else {
                    u64::decode(tokens).map(Self::from_u64)
                }
            }

            fn decode_strict(tokens: &mut impl Iterator<Item = u32>) -> Result<Self, DecodeError> {
                let marker = tokens.next().ok_or(DecodeError::UnexpectedEndOfStream {
                    context: EOFContext::new("JsonInteger", Some("marker"), "signed/unsigned marker")
                })?;
                match marker {
                    0 => i64::decode_strict(tokens).map(Self::from_i64),
                    1 => u64::decode_strict(tokens).map(Self::from_u64),
                    token => Err(DecodeError::InvalidTokenInContext {
                        token,
                        context: EOFContext::new(
                            "JsonInteger",
                            Some("marker"),
                            "integer signed/unsigned marker 0 or 1",
                        ),
                    }),
                }
            }

            fn token_count(&self) -> usize {
                match self.0 {
                    JsonIntegerRepr::Signed(value) => 1 + value.token_count(),
                    JsonIntegerRepr::Unsigned(value) => 1 + value.token_count(),
                }
            }
        }
    }
}
