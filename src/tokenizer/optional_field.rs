//! `OptionalField<T>` - Three-state optional type for explicit null preservation
//!
//! This module provides `OptionalField<T>`, which distinguishes between three states:
//! - Absent: Field not present in JSON
//! - Null: Field explicitly set to null in JSON
//! - Present(T): Field has a value in JSON
//!
//! This is necessary because serde's `Option<T>` cannot distinguish between
//! "field absent" and "field = null", both deserialize to None.

use super::{DecodeError, EOFContext, JsonTokenEncoder};
use serde::{Deserialize, Deserializer, Serialize, Serializer};

/// Three-state optional field type for nullable fields in JSON schemas
///
/// # Serialization Behavior
/// - `Absent` → field omitted from JSON
/// - `Null` → field serialized as `null`
/// - `Present(value)` → field serialized as `value`
///
/// # Deserialization Behavior
/// - Field not present → `Absent`
/// - Field is `null` → `Null`
/// - Field has value → `Present(value)`
///
/// # Token Encoding
/// - `Absent` → single token `0`
/// - `Null` → single token `1`
/// - `Present(value)` → token `2` followed by value's tokens
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default)]
pub enum OptionalField<T> {
    /// Field is not present in the JSON
    #[default]
    Absent,
    /// Field is explicitly set to null in the JSON
    Null,
    /// Field has a value in the JSON
    Present(T),
}

impl<T> OptionalField<T> {
    /// Returns `true` if the field is `Absent`
    pub fn is_absent(&self) -> bool {
        matches!(self, OptionalField::Absent)
    }

    /// Returns `true` if the field is `Null`
    pub fn is_null(&self) -> bool {
        matches!(self, OptionalField::Null)
    }

    /// Returns `true` if the field is `Present`
    pub fn is_present(&self) -> bool {
        matches!(self, OptionalField::Present(_))
    }

    /// Returns the contained value, if present
    pub fn as_ref(&self) -> OptionalField<&T> {
        match self {
            OptionalField::Absent => OptionalField::Absent,
            OptionalField::Null => OptionalField::Null,
            OptionalField::Present(value) => OptionalField::Present(value),
        }
    }

    /// Converts from `OptionalField<T>` to `Option<Option<T>>`
    ///
    /// - `Absent` → `None`
    /// - `Null` → `Some(None)`
    /// - `Present(value)` → `Some(Some(value))`
    pub fn into_option_option(self) -> Option<Option<T>> {
        match self {
            OptionalField::Absent => None,
            OptionalField::Null => Some(None),
            OptionalField::Present(value) => Some(Some(value)),
        }
    }
}

// Serde serialization: control how the field appears in JSON
impl<T: Serialize> Serialize for OptionalField<T> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match self {
            // Absent should skip the field entirely (handled by skip_serializing_if)
            // This branch won't be called due to skip_serializing_if, but provide for completeness
            OptionalField::Absent => serializer.serialize_none(),
            // Null must serialize as explicit JSON null
            // Using serde_json::Value::Null ensures it's emitted in the output
            OptionalField::Null => {
                use serde_json::Value;
                Value::Null.serialize(serializer)
            }
            // Present serializes the value directly
            OptionalField::Present(value) => value.serialize(serializer),
        }
    }
}

// Serde deserialization: distinguish between absent, null, and present
impl<'de, T: Deserialize<'de>> Deserialize<'de> for OptionalField<T> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        // Deserialize as Option<T>:
        // - Some(value) means field was present with a value
        // - None means field was present but null
        // Note: Field absence is handled at the struct level with #[serde(default)]
        let opt: Option<T> = Option::deserialize(deserializer)?;
        match opt {
            Some(value) => Ok(OptionalField::Present(value)),
            None => Ok(OptionalField::Null),
        }
    }
}

// JsonTokenEncoder implementation for tokenization
impl<T: JsonTokenEncoder> JsonTokenEncoder for OptionalField<T> {
    fn encode(&self, tokens: &mut Vec<u32>) {
        match self {
            OptionalField::Absent => {
                tokens.push(0); // State flag: absent
            }
            OptionalField::Null => {
                tokens.push(1); // State flag: null
            }
            OptionalField::Present(value) => {
                tokens.push(2); // State flag: present
                value.encode(tokens); // Encode the value
            }
        }
    }

    fn decode(tokens: &mut impl Iterator<Item = u32>) -> Result<Self, DecodeError> {
        let state_flag = tokens.next().ok_or(DecodeError::UnexpectedEndOfStream {
            context: EOFContext::new("OptionalField<T>", Some("state_flag"), "presence state"),
        })?;

        match state_flag % 3 {
            // Use modulo for safety
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
            context: EOFContext::new("OptionalField<T>", Some("state_flag"), "presence state"),
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
                context: EOFContext::new(
                    "OptionalField<T>",
                    Some("state_flag"),
                    "presence state 0, 1, or 2",
                ),
            }),
        }
    }

    fn token_count(&self) -> usize {
        match self {
            OptionalField::Absent => 1, // Just the state flag
            OptionalField::Null => 1,   // Just the state flag
            OptionalField::Present(value) => 1 + value.token_count(), // State flag + value
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_optional_field_serialization() {
        // Absent serializes to null (but should be skipped by skip_serializing_if)
        let absent: OptionalField<String> = OptionalField::Absent;
        let json = serde_json::to_string(&absent).unwrap();
        assert_eq!(json, "null");

        // Null serializes to null
        let null: OptionalField<String> = OptionalField::Null;
        let json = serde_json::to_string(&null).unwrap();
        assert_eq!(json, "null");

        // Present serializes to the value
        let present: OptionalField<String> = OptionalField::Present("hello".to_string());
        let json = serde_json::to_string(&present).unwrap();
        assert_eq!(json, "\"hello\"");
    }

    #[test]
    fn test_optional_field_deserialization() {
        // Deserialize null -> Null
        let null: OptionalField<String> = serde_json::from_str("null").unwrap();
        assert_eq!(null, OptionalField::Null);

        // Deserialize value -> Present
        let present: OptionalField<String> = serde_json::from_str("\"hello\"").unwrap();
        assert_eq!(present, OptionalField::Present("hello".to_string()));
    }

    #[test]
    fn test_optional_field_token_encoding() {
        // Test Absent
        let absent: OptionalField<String> = OptionalField::Absent;
        let mut tokens = Vec::new();
        absent.encode(&mut tokens);
        assert_eq!(tokens, vec![0]);
        assert_eq!(absent.token_count(), 1);

        let mut iter = tokens.into_iter();
        let decoded = OptionalField::<String>::decode(&mut iter).unwrap();
        assert_eq!(decoded, OptionalField::Absent);

        // Test Null
        let null: OptionalField<String> = OptionalField::Null;
        let mut tokens = Vec::new();
        null.encode(&mut tokens);
        assert_eq!(tokens, vec![1]);
        assert_eq!(null.token_count(), 1);

        let mut iter = tokens.into_iter();
        let decoded = OptionalField::<String>::decode(&mut iter).unwrap();
        assert_eq!(decoded, OptionalField::Null);

        // Test Present
        let present = OptionalField::Present("test".to_string());
        let mut tokens = Vec::new();
        present.encode(&mut tokens);
        // Should be: [2, 't', 'e', 's', 't', 256] (2 = present flag, then chars, then END_STRING_MARKER)
        assert_eq!(tokens[0], 2); // State flag
        assert_eq!(tokens[1], 't' as u32); // First char (end-marker encoding)
        assert_eq!(tokens[tokens.len() - 1], 256); // END_STRING_MARKER
        assert_eq!(present.token_count(), 6); // 1 (flag) + 4 (chars) + 1 (END_STRING_MARKER)

        let mut iter = tokens.into_iter();
        let decoded = OptionalField::<String>::decode(&mut iter).unwrap();
        assert_eq!(decoded, OptionalField::Present("test".to_string()));
    }

    // ========== Helper Method Tests ==========

    #[test]
    fn test_is_absent() {
        let absent: OptionalField<String> = OptionalField::Absent;
        assert!(absent.is_absent());
        assert!(!absent.is_null());
        assert!(!absent.is_present());
    }

    #[test]
    fn test_is_null() {
        let null: OptionalField<String> = OptionalField::Null;
        assert!(!null.is_absent());
        assert!(null.is_null());
        assert!(!null.is_present());
    }

    #[test]
    fn test_is_present() {
        let present = OptionalField::Present(42);
        assert!(!present.is_absent());
        assert!(!present.is_null());
        assert!(present.is_present());
    }

    #[test]
    fn test_as_ref_absent() {
        let absent: OptionalField<String> = OptionalField::Absent;
        let ref_field = absent.as_ref();
        assert!(ref_field.is_absent());
    }

    #[test]
    fn test_as_ref_null() {
        let null: OptionalField<String> = OptionalField::Null;
        let ref_field = null.as_ref();
        assert!(ref_field.is_null());
    }

    #[test]
    fn test_as_ref_present() {
        let present = OptionalField::Present(String::from("hello"));
        let ref_field = present.as_ref();
        assert!(ref_field.is_present());
        assert_eq!(ref_field, OptionalField::Present(&String::from("hello")));
    }

    #[test]
    fn test_into_option_option_absent() {
        let absent: OptionalField<i32> = OptionalField::Absent;
        let option_option = absent.into_option_option();
        assert_eq!(option_option, None);
    }

    #[test]
    fn test_into_option_option_null() {
        let null: OptionalField<i32> = OptionalField::Null;
        let option_option = null.into_option_option();
        assert_eq!(option_option, Some(None));
    }

    #[test]
    fn test_into_option_option_present() {
        let present = OptionalField::Present(42);
        let option_option = present.into_option_option();
        assert_eq!(option_option, Some(Some(42)));
    }

    // ========== Default Trait Test ==========

    #[test]
    fn test_default() {
        let default: OptionalField<String> = OptionalField::default();
        assert!(default.is_absent());
    }

    // ========== Trait Derivation Tests ==========

    #[test]
    fn test_clone() {
        let present = OptionalField::Present(vec![1, 2, 3]);
        let cloned = present.clone();
        assert_eq!(present, cloned);
    }

    #[test]
    fn test_partial_eq() {
        let present1 = OptionalField::Present(42);
        let present2 = OptionalField::Present(42);
        let present3 = OptionalField::Present(43);

        assert_eq!(present1, present2);
        assert_ne!(present1, present3);
    }

    #[test]
    fn test_debug() {
        let present = OptionalField::Present(42);
        let debug_str = format!("{:?}", present);
        assert!(debug_str.contains("Present"));
        assert!(debug_str.contains("42"));
    }

    #[test]
    fn test_hash() {
        use std::collections::HashSet;
        let mut set = HashSet::new();

        set.insert(OptionalField::Absent::<i32>);
        set.insert(OptionalField::Null::<i32>);
        set.insert(OptionalField::Present(42));

        assert_eq!(set.len(), 3);
        assert!(set.contains(&OptionalField::Absent));
        assert!(set.contains(&OptionalField::Null));
        assert!(set.contains(&OptionalField::Present(42)));
    }

    // ========== Decode Error Tests ==========

    #[test]
    fn test_decode_unexpected_end_of_stream() {
        let tokens: Vec<u32> = vec![]; // Empty stream
        let mut iter = tokens.into_iter();
        let result = OptionalField::<String>::decode(&mut iter);
        assert!(result.is_err());
    }

    #[test]
    fn test_decode_modulo_safety() {
        // Test with state_flag values that should map to 0, 1, 2 via modulo 3
        let test_cases = vec![
            (0, OptionalField::Absent),
            (3, OptionalField::Absent), // 3 % 3 = 0
            (6, OptionalField::Absent), // 6 % 3 = 0
            (1, OptionalField::Null),
            (4, OptionalField::Null), // 4 % 3 = 1
            (7, OptionalField::Null), // 7 % 3 = 1
        ];

        for (state_flag, expected) in test_cases {
            let tokens = vec![state_flag];
            let mut iter = tokens.into_iter();
            let decoded = OptionalField::<String>::decode(&mut iter).unwrap();
            assert_eq!(decoded, expected, "Failed for state_flag={}", state_flag);
        }
    }
}
