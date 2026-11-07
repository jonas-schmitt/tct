//! Generated support for schema fields that preserve arbitrary JSON values.
//!
//! These renderers add `JsonTokenEncoder` implementations for
//! `serde_json::Number` and `serde_json::Value`, keeping validation-only or
//! catch-all schema paths compatible with the same standalone tokenizer module
//! used by fully specialized generated types.

use proc_macro2::TokenStream;
use quote::quote;

pub(super) fn render_json_types() -> TokenStream {
    quote! {
        impl JsonTokenEncoder for serde_json::Number {
            fn encode(&self, tokens: &mut Vec<u32>) {
                if let Some(int_val) = self.as_i64() {
                    tokens.push(0u32);
                    int_val.encode(tokens);
                } else if let Some(uint_val) = self.as_u64() {
                    tokens.push(2u32);
                    uint_val.encode(tokens);
                } else if let Some(float_val) = self.as_f64() {
                    tokens.push(1u32);
                    float_val.encode(tokens);
                } else {
                    tokens.push(1u32);
                    0.0f64.encode(tokens);
                }
            }
            fn decode(tokens: &mut impl Iterator<Item = u32>) -> Result<Self, DecodeError> {
                let marker = tokens.next().ok_or(DecodeError::UnexpectedEndOfStream {
                    context: EOFContext::new("serde_json::Number", None, "type marker (0=int, 1=float, 2=uint)")
                })?;
                match marker % 3 {
                    0u32 => {
                        let int_val = i64::decode(tokens)?;
                        Ok(serde_json::Number::from(int_val))
                    }
                    1u32 => {
                        let float_val = f64::decode(tokens)?;
                        serde_json::Number::from_f64(float_val)
                            .ok_or_else(|| DecodeError::InvalidData("Invalid float value".to_string()))
                    }
                    _ => {
                        let uint_val = u64::decode(tokens)?;
                        Ok(serde_json::Number::from(uint_val))
                    }
                }
            }
            fn decode_strict(tokens: &mut impl Iterator<Item = u32>) -> Result<Self, DecodeError> {
                let marker = tokens.next().ok_or(DecodeError::UnexpectedEndOfStream {
                    context: EOFContext::new("serde_json::Number", Some("marker"), "type marker (0=int, 1=float, 2=uint)")
                })?;
                match marker {
                    0u32 => {
                        let int_val = i64::decode_strict(tokens)?;
                        Ok(serde_json::Number::from(int_val))
                    }
                    1u32 => {
                        let float_val = f64::decode_strict(tokens)?;
                        serde_json::Number::from_f64(float_val)
                            .ok_or_else(|| DecodeError::InvalidData("Invalid float value".to_string()))
                    }
                    2u32 => {
                        let uint_val = u64::decode_strict(tokens)?;
                        Ok(serde_json::Number::from(uint_val))
                    }
                    token => Err(DecodeError::InvalidTokenInContext {
                        token,
                        context: EOFContext::new(
                            "serde_json::Number",
                            Some("marker"),
                            "number marker 0, 1, or 2",
                        ),
                    }),
                }
            }
            fn token_count(&self) -> usize {
                if let Some(int_val) = self.as_i64() {
                    1 + int_val.token_count()
                } else if let Some(uint_val) = self.as_u64() {
                    1 + uint_val.token_count()
                } else if let Some(float_val) = self.as_f64() {
                    1 + float_val.token_count()
                } else {
                    1 + 0.0f64.token_count()
                }
            }
        }

        impl JsonTokenEncoder for serde_json::Value {
            fn encode(&self, tokens: &mut Vec<u32>) {
                match self {
                    serde_json::Value::Null => { tokens.push(0); }
                    serde_json::Value::Bool(b) => { tokens.push(1); b.encode(tokens); }
                    serde_json::Value::Number(n) => { tokens.push(2); n.encode(tokens); }
                    serde_json::Value::String(s) => { tokens.push(3); s.encode(tokens); }
                    serde_json::Value::Array(arr) => { tokens.push(4); arr.encode(tokens); }
                    serde_json::Value::Object(obj) => {
                        tokens.push(5);
                        encode_json_object_map(obj, tokens);
                    }
                }
            }
            fn decode(tokens: &mut impl Iterator<Item = u32>) -> Result<Self, DecodeError> {
                let type_tag = tokens.next().ok_or(DecodeError::UnexpectedEndOfStream {
                    context: EOFContext::new("serde_json::Value", None, "type tag (0-5)")
                })?;
                match type_tag % 6 {
                    0 => Ok(serde_json::Value::Null),
                    1 => Ok(serde_json::Value::Bool(bool::decode(tokens)?)),
                    2 => Ok(serde_json::Value::Number(serde_json::Number::decode(tokens)?)),
                    3 => Ok(serde_json::Value::String(String::decode(tokens)?)),
                    4 => Ok(serde_json::Value::Array(Vec::<serde_json::Value>::decode(tokens)?)),
                    _ => {
                        let map = rustc_hash::FxHashMap::<String, serde_json::Value>::decode(tokens)?;
                        let obj: serde_json::Map<String, serde_json::Value> = map.into_iter().collect();
                        Ok(serde_json::Value::Object(obj))
                    }
                }
            }
            fn decode_strict(tokens: &mut impl Iterator<Item = u32>) -> Result<Self, DecodeError> {
                let type_tag = tokens.next().ok_or(DecodeError::UnexpectedEndOfStream {
                    context: EOFContext::new("serde_json::Value", Some("type_tag"), "type tag (0-5)")
                })?;
                match type_tag {
                    0 => Ok(serde_json::Value::Null),
                    1 => Ok(serde_json::Value::Bool(bool::decode_strict(tokens)?)),
                    2 => Ok(serde_json::Value::Number(serde_json::Number::decode_strict(tokens)?)),
                    3 => Ok(serde_json::Value::String(String::decode_strict(tokens)?)),
                    4 => Ok(serde_json::Value::Array(Vec::<serde_json::Value>::decode_strict(tokens)?)),
                    5 => {
                        let map = rustc_hash::FxHashMap::<String, serde_json::Value>::decode_strict(tokens)?;
                        let obj: serde_json::Map<String, serde_json::Value> = map.into_iter().collect();
                        Ok(serde_json::Value::Object(obj))
                    }
                    token => Err(DecodeError::InvalidTokenInContext {
                        token,
                        context: EOFContext::new(
                            "serde_json::Value",
                            Some("type_tag"),
                            "value type tag 0..=5",
                        ),
                    }),
                }
            }
            fn token_count(&self) -> usize {
                match self {
                    serde_json::Value::Null => 1,
                    serde_json::Value::Bool(b) => 1 + b.token_count(),
                    serde_json::Value::Number(n) => 1 + n.token_count(),
                    serde_json::Value::String(s) => 1 + s.token_count(),
                    serde_json::Value::Array(arr) => 1 + arr.token_count(),
                    serde_json::Value::Object(obj) => {
                        1 + json_object_map_token_count(obj)
                    }
                }
            }
        }

        fn encode_json_object_map(
            obj: &serde_json::Map<String, serde_json::Value>,
            tokens: &mut Vec<u32>,
        ) {
            variable_base::encode_u32_fixed_base(obj.len() as u32, tokens);

            let map: rustc_hash::FxHashMap<&String, &serde_json::Value> =
                obj.iter().collect();
            for (key, value) in map.iter() {
                (*key).encode(tokens);
                (*value).encode(tokens);
            }
        }

        fn json_object_map_token_count(
            obj: &serde_json::Map<String, serde_json::Value>,
        ) -> usize {
            variable_base::token_count_u32_fixed_base(obj.len() as u32)
                + obj
                    .iter()
                    .map(|(key, value)| key.token_count() + value.token_count())
                    .sum::<usize>()
        }
    }
}
