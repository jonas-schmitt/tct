//! Implements the JsonTokenEncoder for primitive Rust types.

use super::{DEFAULT_MAX_COLLECTION_LENGTH, DecodeError, EOFContext, JsonTokenEncoder};
use crate::generator::additional_properties_analyzer::AdditionalValue;
use rustc_hash::FxHashMap;

#[inline]
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
            Some(token) => {
                let normalized_token = token % 2;
                match normalized_token {
                    0 => Ok(false),
                    1 => Ok(true),
                    _ => unreachable!("Modulo 2 operation should ensure valid token range"),
                }
            }
            None => Err(DecodeError::UnexpectedEndOfStream {
                context: EOFContext::new("bool", None, "boolean value"),
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
                context: EOFContext::new("bool", None, "boolean value"),
            }),
        }
    }

    fn token_count(&self) -> usize {
        1 // bool always uses exactly 1 token
    }
}

impl JsonTokenEncoder for i32 {
    fn encode(&self, tokens: &mut Vec<u32>) {
        // Use multi-token encoding with default base (256)
        // Note: Generated code (grammar.rs) uses configurable encoding_base
        // This library implementation uses DEFAULT_BASE for testing/fallback
        crate::base256::encode_u32(*self as u32, tokens, crate::base256::DEFAULT_BASE);
    }

    fn decode(tokens: &mut impl Iterator<Item = u32>) -> Result<Self, DecodeError> {
        // Use multi-token decoding with default base (256)
        // Note: Generated code uses configurable encoding_base
        Ok(crate::base256::decode_u32(tokens, "i32", crate::base256::DEFAULT_BASE)? as i32)
    }

    fn decode_strict(tokens: &mut impl Iterator<Item = u32>) -> Result<Self, DecodeError> {
        Ok(crate::base256::decode_u32_strict(tokens, "i32", crate::base256::DEFAULT_BASE)? as i32)
    }

    fn token_count(&self) -> usize {
        // Use multi-token token count with default base (256)
        // Note: Generated code uses configurable encoding_base
        crate::base256::token_count_u32(*self as u32, crate::base256::DEFAULT_BASE)
    }
}

impl JsonTokenEncoder for i64 {
    fn encode(&self, tokens: &mut Vec<u32>) {
        // Use base-256 multi-token encoding to respect vocabulary bounds
        // Encode as 8 bytes directly (each byte 0-255 fits in vocabulary)
        let bytes = self.to_le_bytes();

        // Push all 8 bytes as tokens (each 0-255)
        for byte in bytes {
            tokens.push(byte as u32);
        }
    }

    fn decode(tokens: &mut impl Iterator<Item = u32>) -> Result<Self, DecodeError> {
        // Read 8 bytes
        let mut bytes = [0u8; 8];
        for byte in &mut bytes {
            let byte_token = tokens.next().ok_or(DecodeError::UnexpectedEndOfStream {
                context: EOFContext::new("i64", Some("byte"), "byte value"),
            })?;
            *byte = (byte_token % 256) as u8;
        }

        Ok(i64::from_le_bytes(bytes))
    }

    fn decode_strict(tokens: &mut impl Iterator<Item = u32>) -> Result<Self, DecodeError> {
        let mut bytes = [0u8; 8];
        for byte in &mut bytes {
            *byte = decode_strict_byte_token(tokens, "i64")?;
        }

        Ok(i64::from_le_bytes(bytes))
    }

    fn token_count(&self) -> usize {
        8 // i64 always uses exactly 8 tokens (one per byte)
    }
}

impl JsonTokenEncoder for u64 {
    fn encode(&self, tokens: &mut Vec<u32>) {
        let bytes = self.to_le_bytes();
        for byte in bytes {
            tokens.push(byte as u32);
        }
    }

    fn decode(tokens: &mut impl Iterator<Item = u32>) -> Result<Self, DecodeError> {
        let mut bytes = [0u8; 8];
        for byte in &mut bytes {
            let byte_token = tokens.next().ok_or(DecodeError::UnexpectedEndOfStream {
                context: EOFContext::new("u64", Some("byte"), "byte value"),
            })?;
            *byte = (byte_token % 256) as u8;
        }

        Ok(u64::from_le_bytes(bytes))
    }

    fn decode_strict(tokens: &mut impl Iterator<Item = u32>) -> Result<Self, DecodeError> {
        let mut bytes = [0u8; 8];
        for byte in &mut bytes {
            *byte = decode_strict_byte_token(tokens, "u64")?;
        }

        Ok(u64::from_le_bytes(bytes))
    }

    fn token_count(&self) -> usize {
        8
    }
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
    /// Create a JSON integer from a signed value.
    #[must_use]
    pub fn from_i64(value: i64) -> Self {
        Self(JsonIntegerRepr::Signed(value))
    }

    /// Create a JSON integer from an unsigned value.
    #[must_use]
    pub fn from_u64(value: u64) -> Self {
        if let Ok(signed) = i64::try_from(value) {
            Self::from_i64(signed)
        } else {
            Self(JsonIntegerRepr::Unsigned(value))
        }
    }

    /// Convert a serde_json number into a lossless integer if it is represented
    /// as a signed or unsigned integer.
    #[must_use]
    pub fn from_number(number: &serde_json::Number) -> Option<Self> {
        if let Some(value) = number.as_i64() {
            Some(Self::from_i64(value))
        } else {
            number.as_u64().map(Self::from_u64)
        }
    }

    /// Return this value as a serde_json number.
    #[must_use]
    pub fn to_number(self) -> serde_json::Number {
        match self.0 {
            JsonIntegerRepr::Signed(value) => serde_json::Number::from(value),
            JsonIntegerRepr::Unsigned(value) => serde_json::Number::from(value),
        }
    }

    /// Return a signed representation when the value fits.
    #[must_use]
    pub fn as_i64(self) -> Option<i64> {
        match self.0 {
            JsonIntegerRepr::Signed(value) => Some(value),
            JsonIntegerRepr::Unsigned(value) => i64::try_from(value).ok(),
        }
    }

    /// Return an unsigned representation when the value is non-negative.
    #[must_use]
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
        Self::from_number(&number).ok_or_else(|| serde::de::Error::custom("expected JSON integer"))
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
            context: EOFContext::new("JsonInteger", Some("marker"), "signed/unsigned marker"),
        })?;
        if marker % 2 == 0 {
            i64::decode(tokens).map(Self::from_i64)
        } else {
            u64::decode(tokens).map(Self::from_u64)
        }
    }

    fn decode_strict(tokens: &mut impl Iterator<Item = u32>) -> Result<Self, DecodeError> {
        let marker = tokens.next().ok_or(DecodeError::UnexpectedEndOfStream {
            context: EOFContext::new("JsonInteger", Some("marker"), "signed/unsigned marker"),
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

impl JsonTokenEncoder for f64 {
    fn encode(&self, tokens: &mut Vec<u32>) {
        // Use base-256 multi-token encoding to respect vocabulary bounds
        // Encode as 8 bytes directly (each byte 0-255 fits in vocabulary)
        let bytes = self.to_bits().to_le_bytes();

        // Push all 8 bytes as tokens (each 0-255)
        for byte in bytes {
            tokens.push(byte as u32);
        }
    }

    fn decode(tokens: &mut impl Iterator<Item = u32>) -> Result<Self, DecodeError> {
        // Read 8 bytes
        let mut bytes = [0u8; 8];
        for byte in &mut bytes {
            let byte_token = tokens.next().ok_or(DecodeError::UnexpectedEndOfStream {
                context: EOFContext::new("f64", Some("byte"), "byte value"),
            })?;
            *byte = (byte_token % 256) as u8;
        }

        let bits = u64::from_le_bytes(bytes);
        Ok(f64::from_bits(bits))
    }

    fn decode_strict(tokens: &mut impl Iterator<Item = u32>) -> Result<Self, DecodeError> {
        let mut bytes = [0u8; 8];
        for byte in &mut bytes {
            *byte = decode_strict_byte_token(tokens, "f64")?;
        }

        let bits = u64::from_le_bytes(bytes);
        Ok(f64::from_bits(bits))
    }

    fn token_count(&self) -> usize {
        8 // f64 always uses exactly 8 tokens (one per byte)
    }
}

// JsonTokenEncoder implementation for serde_json::Number
// Preserves integer vs float representation during round-trip
impl JsonTokenEncoder for serde_json::Number {
    fn encode(&self, tokens: &mut Vec<u32>) {
        // Try to encode as integer first (preserves original type)
        if let Some(int_val) = self.as_i64() {
            // Marker: 0 = integer
            tokens.push(0u32);
            // Encode as i64 payload after the numeric kind marker.
            int_val.encode(tokens);
        } else if let Some(uint_val) = self.as_u64() {
            // Marker: 2 = unsigned integer
            tokens.push(2u32);
            // Encode as u64 payload after the numeric kind marker.
            uint_val.encode(tokens);
        } else if let Some(float_val) = self.as_f64() {
            // Marker: 1 = float
            tokens.push(1u32);
            // Encode as f64 payload after the numeric kind marker.
            float_val.encode(tokens);
        } else {
            // Fallback: treat as f64 (shouldn't happen with valid JSON numbers)
            tokens.push(1u32);
            0.0f64.encode(tokens);
        }
    }

    fn decode(tokens: &mut impl Iterator<Item = u32>) -> Result<Self, DecodeError> {
        // Read marker
        let marker = tokens.next().ok_or(DecodeError::UnexpectedEndOfStream {
            context: EOFContext::new(
                "serde_json::Number",
                Some("marker"),
                "integer/float/unsigned marker",
            ),
        })?;
        match marker % 3 {
            0u32 => {
                // Integer: decode as i64 and construct Number
                let int_val = i64::decode(tokens)?;
                Ok(serde_json::Number::from(int_val))
            }
            1u32 => {
                // Float: decode as f64 and construct Number
                let float_val = f64::decode(tokens)?;
                serde_json::Number::from_f64(float_val).ok_or_else(|| {
                    DecodeError::InvalidData("Invalid float value (NaN or Inf)".to_string())
                })
            }
            _ => {
                // Unsigned integer: decode as u64 and construct Number
                let uint_val = u64::decode(tokens)?;
                Ok(serde_json::Number::from(uint_val))
            }
        }
    }

    fn decode_strict(tokens: &mut impl Iterator<Item = u32>) -> Result<Self, DecodeError> {
        let marker = tokens.next().ok_or(DecodeError::UnexpectedEndOfStream {
            context: EOFContext::new(
                "serde_json::Number",
                Some("marker"),
                "integer/float/unsigned marker",
            ),
        })?;

        match marker {
            0u32 => {
                let int_val = i64::decode_strict(tokens)?;
                Ok(serde_json::Number::from(int_val))
            }
            1u32 => {
                let float_val = f64::decode_strict(tokens)?;
                serde_json::Number::from_f64(float_val).ok_or_else(|| {
                    DecodeError::InvalidData("Invalid float value (NaN or Inf)".to_string())
                })
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

impl JsonTokenEncoder for String {
    /// Encodes a UTF-8 string into tokens.
    ///
    /// # UTF-8 Byte-Level Encoding
    ///
    /// This implementation encodes strings as **raw UTF-8 bytes (0-255)**:
    /// 1. **Full UTF-8 support**: All valid UTF-8 strings can be encoded
    /// 2. **Byte-level representation**: Direct byte-to-token mapping
    /// 3. **End-marker encoding**: Uses token 256 to signal end of string
    /// 4. **LLM-friendly**: Model learns string boundaries naturally
    ///
    /// # Vocabulary Budget Allocation
    ///
    /// - String bytes: **257 tokens** (0-255 for bytes, 256 for END_STRING_MARKER)
    /// - Structural tokens: Schema-specific (enums, types, etc.)
    /// - Optional: BPE compression layer on top
    ///
    /// # Character Coverage
    ///
    /// **All UTF-8 bytes supported (0-255)**:
    /// - ASCII: `a-z`, `A-Z`, `0-9`, symbols, whitespace
    /// - Extended: Accented characters, symbols (bytes 128-255)
    /// - Multi-byte UTF-8: Emoji, non-Latin scripts (encoded as byte sequences)
    ///
    /// # Performance Characteristics
    ///
    /// **Encoding**: O(n) where n = string byte length
    /// - Single pass: direct byte-to-token conversion
    /// - No validation needed (all UTF-8 bytes are valid)
    ///
    /// **Decoding**: O(n) where n = token count
    /// - Direct token-to-byte conversion
    /// - UTF-8 validation on final string construction
    ///
    /// # Correctness Guarantee
    ///
    /// The encoder preserves UTF-8 validity:
    /// - Input: Valid UTF-8 string (guaranteed by Rust's `String` type)
    /// - Encoding: Byte sequence preservation
    /// - Decoding: UTF-8 validation on reconstruction
    /// - Output: Valid UTF-8 string (or error if corrupted)
    #[inline(always)]
    fn encode(&self, tokens: &mut Vec<u32>) {
        // Encode as UTF-8 bytes: byte values (0-255) + END_STRING_MARKER
        // End-marker encoding is more LLM-friendly: model learns when to stop naturally
        const END_STRING_MARKER: u32 = 256; // Token 256 (first token after byte range)

        let len = self.len();
        tokens.reserve(len + 1); // Pre-allocate: bytes + end marker

        // Direct byte-to-token copy (compiler auto-vectorizes for large strings)
        tokens.extend(self.as_bytes().iter().map(|&b| b as u32));

        // Emit END_STRING_MARKER to signal end of string
        tokens.push(END_STRING_MARKER);
    }

    /// Decodes UTF-8 string from tokens using end-marker encoding.
    ///
    /// # Returns
    ///
    /// - `Ok(String)` if all tokens are valid UTF-8 bytes (0-255) followed by END_STRING_MARKER (256)
    /// - `Err(DecodeError::InvalidToken)` if any token is invalid (>= 256 for content, != 256 for end)
    /// - `Err(DecodeError::UnexpectedEndOfStream)` if token stream ends before END_STRING_MARKER
    /// - `Err(DecodeError::InvalidUtf8)` if byte sequence is not valid UTF-8
    ///
    /// # Correctness Guarantee
    ///
    /// This decoder validates UTF-8 correctness:
    /// - All bytes must be in range 0-255 (checked)
    /// - Byte sequence must form valid UTF-8 (validated by `String::from_utf8`)
    /// - Invalid UTF-8 sequences are rejected with clear error
    ///
    /// # Algorithm (End-Marker Encoding)
    ///
    /// 1. Read UTF-8 byte tokens (0-255) until END_STRING_MARKER (256) encountered
    /// 2. END_STRING_MARKER signals end of string (not included in result)
    /// 3. Convert collected bytes to String (with UTF-8 validation)
    ///
    /// This encoding is LLM-friendly: model learns string boundaries naturally without predicting exact length upfront.
    fn decode(tokens: &mut impl Iterator<Item = u32>) -> Result<Self, DecodeError> {
        const END_STRING_MARKER: u32 = 256; // Token 256 (first token after byte range)

        let mut bytes = Vec::new();

        loop {
            let token = tokens.next().ok_or(DecodeError::UnexpectedEndOfStream {
                context: EOFContext::new(
                    "String",
                    Some("byte or end marker"),
                    "UTF-8 byte or END_STRING_MARKER (256)",
                ),
            })?;

            // Check for END_STRING_MARKER
            if token == END_STRING_MARKER {
                break; // End of string reached
            }

            // Validate UTF-8 byte range (0-255)
            if token > 255 {
                return Err(DecodeError::InvalidToken(token));
            }

            // Convert to byte (safe: 0-255 guaranteed by validation)
            bytes.push(token as u8);
        }

        // Convert UTF-8 bytes to String with validation
        // This may fail if the byte sequence is not valid UTF-8
        String::from_utf8(bytes).map_err(|_| DecodeError::Utf8Error)
    }

    fn token_count(&self) -> usize {
        // End-marker encoding: bytes + END_STRING_MARKER
        // No length prefix needed, just 1 token per byte + 1 end marker
        self.len() + 1 // bytes + END_STRING_MARKER
    }
}

// ============================================================================
// Partial Decode Functions for Streaming
// ============================================================================

/// Decode a string from a potentially truncated token stream.
///
/// Unlike `String::decode()`, this function returns partial results when the
/// token stream ends before END_STRING_MARKER. This is essential for streaming
/// LLM generation where we want to show partial text as it's generated.
///
/// # Returns
///
/// - `PartialStringResult` with decoded bytes and completion status.
/// - For incomplete streams, returns bytes decoded so far with `complete: false`.
/// - Uses `String::from_utf8_lossy` for partial UTF-8 sequences.
pub fn decode_string_prefix(tokens: &mut impl Iterator<Item = u32>) -> super::PartialStringResult {
    use super::{END_STRING_MARKER, PartialStringResult};

    let mut bytes = Vec::new();

    loop {
        match tokens.next() {
            Some(token) if token == END_STRING_MARKER => {
                // Complete string found
                let value = String::from_utf8(bytes.clone())
                    .unwrap_or_else(|_| String::from_utf8_lossy(&bytes).to_string());
                return PartialStringResult {
                    value,
                    complete: true,
                    bytes_decoded: bytes.len(),
                };
            }
            Some(token) if token <= 255 => {
                // Valid UTF-8 byte
                bytes.push(token as u8);
            }
            Some(_) | None => {
                // Stream ended or invalid token - return partial result
                let value = String::from_utf8_lossy(&bytes).to_string();
                return PartialStringResult {
                    value,
                    complete: false,
                    bytes_decoded: bytes.len(),
                };
            }
        }
    }
}

/// Decode a vector from a potentially truncated token stream.
///
/// Unlike `Vec::decode()`, this function returns partial results when not all
/// elements can be decoded. This is useful for streaming decode of arrays.
///
/// # Returns
///
/// - `PartialVecResult` with elements decoded and completion status.
/// - If length prefix is missing, returns empty vec with `expected_length: None`.
/// - If some elements decode, returns them with `complete: false`.
pub fn decode_vec_prefix<T: super::JsonTokenEncoder>(
    tokens: &mut impl Iterator<Item = u32>,
) -> super::PartialVecResult<T> {
    use super::PartialVecResult;

    // Try to decode length
    let length = match crate::base256::decode_u32(tokens, "Vec<T>", crate::base256::DEFAULT_BASE) {
        Ok(len) => len as usize,
        Err(_) => {
            return PartialVecResult {
                items: vec![],
                expected_length: None,
                complete: false,
            };
        }
    };

    let mut items = Vec::with_capacity(length.min(1024)); // Cap allocation for safety

    for _ in 0..length {
        match T::decode(tokens) {
            Ok(item) => items.push(item),
            Err(_) => {
                return PartialVecResult {
                    items,
                    expected_length: Some(length),
                    complete: false,
                };
            }
        }
    }

    PartialVecResult {
        items,
        expected_length: Some(length),
        complete: true,
    }
}

impl<T: JsonTokenEncoder> JsonTokenEncoder for Option<T> {
    fn encode(&self, tokens: &mut Vec<u32>) {
        match self {
            Some(value) => {
                tokens.push(1); // Presence flag
                value.encode(tokens);
            }
            None => {
                tokens.push(0); // Absence flag
            }
        }
    }

    fn decode(tokens: &mut impl Iterator<Item = u32>) -> Result<Self, DecodeError> {
        match tokens.next() {
            Some(token) => {
                let normalized_token = token % 2;
                match normalized_token {
                    0 => Ok(None),
                    1 => Ok(Some(T::decode(tokens)?)),
                    _ => unreachable!("Modulo 2 operation should ensure valid token range"),
                }
            }
            None => Err(DecodeError::UnexpectedEndOfStream {
                context: EOFContext::new("Option<T>", Some("presence"), "presence flag"),
            }),
        }
    }

    fn decode_strict(tokens: &mut impl Iterator<Item = u32>) -> Result<Self, DecodeError> {
        match tokens.next() {
            Some(0) => Ok(None),
            Some(1) => Ok(Some(T::decode_strict(tokens)?)),
            Some(token) => Err(DecodeError::InvalidTokenInContext {
                token,
                context: EOFContext::new("Option<T>", Some("presence"), "presence flag 0 or 1"),
            }),
            None => Err(DecodeError::UnexpectedEndOfStream {
                context: EOFContext::new("Option<T>", Some("presence"), "presence flag"),
            }),
        }
    }

    fn token_count(&self) -> usize {
        match self {
            Some(value) => 1 + value.token_count(), // 1 for presence flag + tokens for value
            None => 1,                              // 1 for absence flag
        }
    }
}

impl<T: JsonTokenEncoder> JsonTokenEncoder for Vec<T> {
    fn encode(&self, tokens: &mut Vec<u32>) {
        // Encode length using multi-token encoding with default base (256)
        // Note: Generated code uses configurable encoding_base via grammar.rs
        crate::base256::encode_u32(self.len() as u32, tokens, crate::base256::DEFAULT_BASE);

        // Encode all items
        for item in self {
            item.encode(tokens);
        }
    }

    fn decode(tokens: &mut impl Iterator<Item = u32>) -> Result<Self, DecodeError> {
        // Decode length using multi-token decoding with default base (256)
        // Note: Generated code uses configurable encoding_base
        let length = crate::base256::decode_u32(tokens, "Vec<T>", crate::base256::DEFAULT_BASE)?;

        let len = usize::try_from(length).map_err(|_| DecodeError::InvalidToken(length))?;
        if len > DEFAULT_MAX_COLLECTION_LENGTH {
            return Err(DecodeError::InvalidData(format!(
                "Vec length {} exceeds maximum {}",
                len, DEFAULT_MAX_COLLECTION_LENGTH
            )));
        }
        let mut vec = Vec::with_capacity(len);
        for _ in 0..len {
            vec.push(T::decode(tokens)?);
        }
        Ok(vec)
    }

    fn decode_strict(tokens: &mut impl Iterator<Item = u32>) -> Result<Self, DecodeError> {
        let length =
            crate::base256::decode_u32_strict(tokens, "Vec<T>", crate::base256::DEFAULT_BASE)?;

        let len = usize::try_from(length).map_err(|_| DecodeError::InvalidToken(length))?;
        if len > DEFAULT_MAX_COLLECTION_LENGTH {
            return Err(DecodeError::InvalidData(format!(
                "Vec length {} exceeds maximum {}",
                len, DEFAULT_MAX_COLLECTION_LENGTH
            )));
        }
        let mut vec = Vec::with_capacity(len);
        for _ in 0..len {
            vec.push(T::decode_strict(tokens)?);
        }
        Ok(vec)
    }

    fn token_count(&self) -> usize {
        // Calculate length encoding size using multi-token with default base (256)
        // Note: Generated code uses configurable encoding_base
        let length_tokens =
            crate::base256::token_count_u32(self.len() as u32, crate::base256::DEFAULT_BASE);

        length_tokens + self.iter().map(|item| item.token_count()).sum::<usize>()
    }
}

// HashMap implementation for JsonTokenEncoder
impl<K, V> JsonTokenEncoder for FxHashMap<K, V>
where
    K: JsonTokenEncoder + std::cmp::Eq + std::hash::Hash,
    V: JsonTokenEncoder,
{
    fn encode(&self, tokens: &mut Vec<u32>) {
        // Encode length using multi-token encoding with default base (256)
        // Note: Generated code uses configurable encoding_base via grammar.rs
        crate::base256::encode_u32(self.len() as u32, tokens, crate::base256::DEFAULT_BASE);

        // Encode each key-value pair
        for (key, value) in self.iter() {
            key.encode(tokens);
            value.encode(tokens);
        }
    }

    fn decode(tokens: &mut impl Iterator<Item = u32>) -> Result<Self, DecodeError> {
        // Decode length using multi-token decoding with default base (256)
        // Note: Generated code uses configurable encoding_base
        let length =
            crate::base256::decode_u32(tokens, "FxHashMap<K,V>", crate::base256::DEFAULT_BASE)?;

        let len = usize::try_from(length).map_err(|_| DecodeError::InvalidToken(length))?;
        if len > DEFAULT_MAX_COLLECTION_LENGTH {
            return Err(DecodeError::InvalidData(format!(
                "HashMap length {} exceeds maximum {}",
                len, DEFAULT_MAX_COLLECTION_LENGTH
            )));
        }

        let mut map = FxHashMap::default();

        // Decode each key-value pair
        for _ in 0..len {
            let key = K::decode(tokens)?;
            let value = V::decode(tokens)?;
            map.insert(key, value);
        }

        Ok(map)
    }

    fn decode_strict(tokens: &mut impl Iterator<Item = u32>) -> Result<Self, DecodeError> {
        let length = crate::base256::decode_u32_strict(
            tokens,
            "FxHashMap<K,V>",
            crate::base256::DEFAULT_BASE,
        )?;

        let len = usize::try_from(length).map_err(|_| DecodeError::InvalidToken(length))?;
        if len > DEFAULT_MAX_COLLECTION_LENGTH {
            return Err(DecodeError::InvalidData(format!(
                "HashMap length {} exceeds maximum {}",
                len, DEFAULT_MAX_COLLECTION_LENGTH
            )));
        }

        let mut map = FxHashMap::default();
        for _ in 0..len {
            let key = K::decode_strict(tokens)?;
            let value = V::decode_strict(tokens)?;
            map.insert(key, value);
        }

        Ok(map)
    }

    fn token_count(&self) -> usize {
        // Calculate length encoding size using multi-token with default base (256)
        // Note: Generated code uses configurable encoding_base
        let length_tokens =
            crate::base256::token_count_u32(self.len() as u32, crate::base256::DEFAULT_BASE);

        length_tokens
            + self
                .iter()
                .map(|(k, v)| k.token_count() + v.token_count())
                .sum::<usize>()
    }
}

// Implementation for Box<T> where T: JsonTokenEncoder
// This enables Box wrapping for recursive types without losing JsonTokenEncoder trait
impl<T: JsonTokenEncoder> JsonTokenEncoder for Box<T> {
    fn encode(&self, tokens: &mut Vec<u32>) {
        self.as_ref().encode(tokens);
    }

    fn decode(tokens: &mut impl Iterator<Item = u32>) -> Result<Self, DecodeError> {
        T::decode(tokens).map(Box::new)
    }

    fn decode_strict(tokens: &mut impl Iterator<Item = u32>) -> Result<Self, DecodeError> {
        T::decode_strict(tokens).map(Box::new)
    }

    fn token_count(&self) -> usize {
        self.as_ref().token_count()
    }
}

// Implementation for unit type () - represents null/void values
impl JsonTokenEncoder for () {
    fn encode(&self, tokens: &mut Vec<u32>) {
        // Unit type encodes as a single null token (0)
        tokens.push(0);
    }

    fn decode(tokens: &mut impl Iterator<Item = u32>) -> Result<Self, DecodeError> {
        // Expect a null token (0)
        match tokens.next() {
            Some(0) => Ok(()),
            Some(token) => Err(DecodeError::InvalidData(format!(
                "Expected null token (0) for unit type, got {}",
                token
            ))),
            None => Err(DecodeError::UnexpectedEndOfStream {
                context: EOFContext::new("()", None, "null marker"),
            }),
        }
    }

    fn token_count(&self) -> usize {
        1 // Always exactly one token for null
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn encode_decode_test<T: JsonTokenEncoder + std::fmt::Debug + PartialEq>(original: T) {
        let mut tokens = Vec::new();
        original.encode(&mut tokens);

        let mut iter = tokens.into_iter();
        let decoded = T::decode(&mut iter).unwrap();

        assert_eq!(original, decoded);
        assert_eq!(iter.next(), None, "Iterator should be fully consumed");
    }

    #[test]
    fn test_bool() {
        encode_decode_test(true);
        encode_decode_test(false);
    }

    #[test]
    fn test_i64() {
        encode_decode_test(0i64);
        encode_decode_test(1234567890123456789i64);
        encode_decode_test(-1234567890123456789i64);
        encode_decode_test(i64::MAX);
        encode_decode_test(i64::MIN);
    }

    #[test]
    fn test_f64() {
        encode_decode_test(0.0f64);
        encode_decode_test(123.456e78);
        encode_decode_test(-123.456e-78);
        encode_decode_test(f64::MAX);
        encode_decode_test(f64::MIN);
    }

    #[test]
    fn test_string() {
        encode_decode_test("hello world".to_string());
        encode_decode_test("".to_string());
        encode_decode_test("test123".to_string()); // ASCII-only
    }

    #[test]
    fn test_option() {
        encode_decode_test(Some("hello".to_string()));
        encode_decode_test(None::<String>);
        encode_decode_test(Some(123i64));
        encode_decode_test(None::<i64>);
    }

    #[test]
    fn test_vec() {
        encode_decode_test(vec![1i64, 2, 3, 4, 5]);
        encode_decode_test(Vec::<i64>::new());
        encode_decode_test(vec!["hello".to_string(), "world".to_string()]);
    }

    #[test]
    fn test_nested_vec_option() {
        let data: Vec<Option<String>> =
            vec![Some("first".to_string()), None, Some("third".to_string())];
        encode_decode_test(data);
    }

    #[test]
    fn test_hashmap_string_string() {
        let mut data = FxHashMap::default();
        data.insert("key1".to_string(), "value1".to_string());
        data.insert("key2".to_string(), "value2".to_string());
        encode_decode_test(data);
    }

    #[test]
    fn test_hashmap_empty() {
        let data: FxHashMap<String, i32> = FxHashMap::default();
        encode_decode_test(data);
    }

    #[test]
    fn test_serde_json_value() {
        use serde_json::Value;

        // Test null
        encode_decode_test(Value::Null);

        // Test bool
        encode_decode_test(Value::Bool(true));
        encode_decode_test(Value::Bool(false));

        // Test number
        encode_decode_test(Value::Number(serde_json::Number::from(42)));
        encode_decode_test(Value::Number(
            serde_json::Number::from_f64(std::f64::consts::PI).unwrap(),
        ));

        // Test string
        encode_decode_test(Value::String("hello world".to_string()));

        // Test array
        encode_decode_test(Value::Array(vec![
            Value::String("item1".to_string()),
            Value::Number(serde_json::Number::from(123)),
            Value::Bool(true),
        ]));

        // Test object
        let mut map = serde_json::Map::new();
        map.insert("key1".to_string(), Value::String("value1".to_string()));
        map.insert(
            "key2".to_string(),
            Value::Number(serde_json::Number::from(456)),
        );
        encode_decode_test(Value::Object(map));
    }
}

fn encode_json_object_map(obj: &serde_json::Map<String, serde_json::Value>, tokens: &mut Vec<u32>) {
    crate::base256::encode_u32(obj.len() as u32, tokens, crate::base256::DEFAULT_BASE);

    let map: FxHashMap<&String, &serde_json::Value> = obj.iter().collect();
    for (key, value) in map.iter() {
        (*key).encode(tokens);
        (*value).encode(tokens);
    }
}

fn json_object_map_token_count(obj: &serde_json::Map<String, serde_json::Value>) -> usize {
    crate::base256::token_count_u32(obj.len() as u32, crate::base256::DEFAULT_BASE)
        + obj
            .iter()
            .map(|(key, value)| key.token_count() + value.token_count())
            .sum::<usize>()
}

// Implementation for serde_json::Value to support additionalProperties: true
impl JsonTokenEncoder for serde_json::Value {
    fn encode(&self, tokens: &mut Vec<u32>) {
        match self {
            serde_json::Value::Null => {
                tokens.push(0); // Type tag for null
            }
            serde_json::Value::Bool(b) => {
                tokens.push(1); // Type tag for bool
                b.encode(tokens);
            }
            serde_json::Value::Number(n) => {
                tokens.push(2); // Type tag for number
                if let Some(i) = n.as_i64() {
                    tokens.push(0); // Subtype: i64
                    i.encode(tokens);
                } else if let Some(u) = n.as_u64() {
                    tokens.push(2); // Subtype: u64
                    u.encode(tokens);
                } else if let Some(f) = n.as_f64() {
                    tokens.push(1); // Subtype: f64
                    f.encode(tokens);
                } else {
                    // Fallback: encode as string
                    tokens.push(3); // Subtype: string representation
                    n.to_string().encode(tokens);
                }
            }
            serde_json::Value::String(s) => {
                tokens.push(3); // Type tag for string
                s.encode(tokens);
            }
            serde_json::Value::Array(arr) => {
                tokens.push(4); // Type tag for array
                arr.encode(tokens);
            }
            serde_json::Value::Object(obj) => {
                tokens.push(5); // Type tag for object
                encode_json_object_map(obj, tokens);
            }
        }
    }

    fn decode(tokens: &mut impl Iterator<Item = u32>) -> Result<Self, DecodeError> {
        let type_tag = tokens.next().ok_or(DecodeError::UnexpectedEndOfStream {
            context: EOFContext::new("serde_json::Value", Some("type_tag"), "value type"),
        })?;

        match type_tag % 6 {
            0 => Ok(serde_json::Value::Null),
            1 => {
                let b = bool::decode(tokens)?;
                Ok(serde_json::Value::Bool(b))
            }
            2 => Ok(serde_json::Value::Number(serde_json::Number::decode(
                tokens,
            )?)),
            3 => {
                let s = String::decode(tokens)?;
                Ok(serde_json::Value::String(s))
            }
            4 => {
                let arr = Vec::<serde_json::Value>::decode(tokens)?;
                Ok(serde_json::Value::Array(arr))
            }
            _ => {
                // Tag 5 is object; the wildcard keeps the modulo match exhaustive.
                let map = FxHashMap::<String, serde_json::Value>::decode(tokens)?;
                let obj: serde_json::Map<String, serde_json::Value> = map.into_iter().collect();
                Ok(serde_json::Value::Object(obj))
            }
        }
    }

    fn decode_strict(tokens: &mut impl Iterator<Item = u32>) -> Result<Self, DecodeError> {
        let type_tag = tokens.next().ok_or(DecodeError::UnexpectedEndOfStream {
            context: EOFContext::new("serde_json::Value", Some("type_tag"), "value type"),
        })?;

        match type_tag {
            0 => Ok(serde_json::Value::Null),
            1 => {
                let b = bool::decode_strict(tokens)?;
                Ok(serde_json::Value::Bool(b))
            }
            2 => Ok(serde_json::Value::Number(
                serde_json::Number::decode_strict(tokens)?,
            )),
            3 => {
                let s = String::decode_strict(tokens)?;
                Ok(serde_json::Value::String(s))
            }
            4 => {
                let arr = Vec::<serde_json::Value>::decode_strict(tokens)?;
                Ok(serde_json::Value::Array(arr))
            }
            5 => {
                let map = FxHashMap::<String, serde_json::Value>::decode_strict(tokens)?;
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
            serde_json::Value::Null => 1,                      // 1 for type tag
            serde_json::Value::Bool(b) => 1 + b.token_count(), // 1 for type tag + bool tokens
            serde_json::Value::Number(n) => {
                if let Some(i) = n.as_i64() {
                    2 + i.token_count() // 1 for type tag + 1 for subtype + i64 tokens
                } else if let Some(u) = n.as_u64() {
                    2 + u.token_count() // 1 for type tag + 1 for subtype + u64 tokens
                } else if let Some(f) = n.as_f64() {
                    2 + f.token_count() // 1 for type tag + 1 for subtype + f64 tokens
                } else {
                    2 + n.to_string().token_count() // 1 for type tag + 1 for subtype + string tokens
                }
            }
            serde_json::Value::String(s) => 1 + s.token_count(), // 1 for type tag + string tokens
            serde_json::Value::Array(arr) => 1 + arr.token_count(), // 1 for type tag + array tokens
            serde_json::Value::Object(obj) => {
                // 1 for type tag + object key/value tokens
                1 + json_object_map_token_count(obj)
            }
        }
    }
}

impl JsonTokenEncoder for AdditionalValue {
    fn encode(&self, tokens: &mut Vec<u32>) {
        match self {
            AdditionalValue::String(s) => {
                tokens.push(0); // Type tag for string
                s.encode(tokens);
            }
            AdditionalValue::Number(n) => {
                tokens.push(1); // Type tag for number
                n.encode(tokens);
            }
            AdditionalValue::Bool(b) => {
                tokens.push(2); // Type tag for boolean
                b.encode(tokens);
            }
            AdditionalValue::Null => {
                tokens.push(3); // Type tag for null
            }
        }
    }

    fn decode(tokens: &mut impl Iterator<Item = u32>) -> Result<Self, DecodeError> {
        let type_tag = tokens.next().ok_or(DecodeError::UnexpectedEndOfStream {
            context: EOFContext::new("AdditionalValue", Some("type_tag"), "property type"),
        })?;

        match type_tag % 4 {
            // Only 4 variants: 0-3
            0 => {
                let s = String::decode(tokens)?;
                Ok(AdditionalValue::String(s))
            }
            1 => {
                let n = f64::decode(tokens)?;
                Ok(AdditionalValue::Number(n))
            }
            2 => {
                let b = bool::decode(tokens)?;
                Ok(AdditionalValue::Bool(b))
            }
            3 => Ok(AdditionalValue::Null),
            _ => unreachable!("Modulo 4 operation ensures valid range"),
        }
    }

    fn decode_strict(tokens: &mut impl Iterator<Item = u32>) -> Result<Self, DecodeError> {
        let type_tag = tokens.next().ok_or(DecodeError::UnexpectedEndOfStream {
            context: EOFContext::new("AdditionalValue", Some("type_tag"), "property type"),
        })?;

        match type_tag {
            0 => {
                let s = String::decode_strict(tokens)?;
                Ok(AdditionalValue::String(s))
            }
            1 => {
                let n = f64::decode_strict(tokens)?;
                Ok(AdditionalValue::Number(n))
            }
            2 => {
                let b = bool::decode_strict(tokens)?;
                Ok(AdditionalValue::Bool(b))
            }
            3 => Ok(AdditionalValue::Null),
            token => Err(DecodeError::InvalidTokenInContext {
                token,
                context: EOFContext::new(
                    "AdditionalValue",
                    Some("type_tag"),
                    "property type tag 0..=3",
                ),
            }),
        }
    }

    fn token_count(&self) -> usize {
        match self {
            AdditionalValue::String(s) => 1 + s.token_count(), // 1 for type tag + string tokens
            AdditionalValue::Number(n) => 1 + n.token_count(), // 1 for type tag + number tokens
            AdditionalValue::Bool(b) => 1 + b.token_count(),   // 1 for type tag + bool tokens
            AdditionalValue::Null => 1,                        // 1 for type tag only
        }
    }
}

#[cfg(test)]
mod token_count_tests {
    use super::*;

    #[test]
    fn test_token_count_accuracy() {
        // Test basic types
        assert_eq!(true.token_count(), 1);
        assert_eq!(false.token_count(), 1);
        assert_eq!(42i32.token_count(), 2); // width(0) + 1 digit
        assert_eq!(42i64.token_count(), 8); // 8 bytes
        assert_eq!(std::f64::consts::PI.token_count(), 8); // 8 bytes

        // Test string (end-marker encoding, no length prefix)
        let s = "hello".to_string();
        assert_eq!(s.token_count(), 6); // 5 bytes + END_STRING_MARKER

        // Test Option
        let some_val = Some(42i32);
        let none_val: Option<i32> = None;
        assert_eq!(some_val.token_count(), 3); // 1 for flag + 2 for i32
        assert_eq!(none_val.token_count(), 1); // 1 for absence flag

        // Test Vec
        let vec = vec![1i32, 2i32, 3i32];
        // length: width(0) + 1 digit = 2 tokens
        // items: 3 * 2 = 6 tokens
        assert_eq!(vec.token_count(), 8);

        // Test HashMap
        let mut map = FxHashMap::default();
        map.insert("key1".to_string(), "value1".to_string());
        map.insert("key2".to_string(), "value2".to_string());
        // length: width(0) + 1 digit = 2 tokens
        // "key1" = 4 bytes + END = 5, "value1" = 6 bytes + END = 7
        // "key2" = 4 bytes + END = 5, "value2" = 6 bytes + END = 7
        // = 2 + (5 + 7) + (5 + 7) = 2 + 12 + 12 = 26
        assert_eq!(map.token_count(), 26);

        // Test AdditionalValue
        assert_eq!(AdditionalValue::Null.token_count(), 1);
        assert_eq!(AdditionalValue::Bool(true).token_count(), 2); // 1 tag + 1 bool
        assert_eq!(
            AdditionalValue::Number(std::f64::consts::PI).token_count(),
            9
        ); // 1 tag + 8 f64
        assert_eq!(AdditionalValue::String("test".to_string()).token_count(), 6);
        // 1 tag + 5 string
    }

    #[test]
    fn test_token_count_vs_actual_encoding() {
        // Verify that token_count matches actual encoding for basic types
        let test_string = "hello world".to_string();
        let mut tokens = Vec::new();
        test_string.encode(&mut tokens);
        assert_eq!(test_string.token_count(), tokens.len());

        // Test with Option
        let some_val = Some("test".to_string());
        let mut tokens = Vec::new();
        some_val.encode(&mut tokens);
        assert_eq!(some_val.token_count(), tokens.len());

        // Test with Vec
        let vec_val = vec!["a".to_string(), "bb".to_string()];
        let mut tokens = Vec::new();
        vec_val.encode(&mut tokens);
        assert_eq!(vec_val.token_count(), tokens.len());

        // Test with AdditionalValue
        let add_val = AdditionalValue::String("abc".to_string());
        let mut tokens = Vec::new();
        add_val.encode(&mut tokens);
        assert_eq!(add_val.token_count(), tokens.len());
    }
}

// ========== Additional Coverage Tests ==========
#[cfg(test)]
mod additional_coverage_tests {
    use super::*;

    // ========== i32 Tests ==========
    #[test]
    fn test_i32_encode_decode() {
        encode_decode_test(0i32);
        encode_decode_test(42i32);
        encode_decode_test(-42i32);
        encode_decode_test(i32::MAX);
        encode_decode_test(i32::MIN);
    }

    #[test]
    fn test_i32_token_count() {
        // Base-256 encoding: width indicator + digits
        assert_eq!(0i32.token_count(), 2); // width=0 + 1 digit
        assert_eq!(255i32.token_count(), 2); // width=0 + 1 digit
        assert_eq!(256i32.token_count(), 3); // width=1 + 2 digits
        assert_eq!(8080i32.token_count(), 3); // width=1 + 2 digits (port number)
        assert_eq!(65535i32.token_count(), 3); // width=1 + 2 digits
        assert_eq!(65536i32.token_count(), 4); // width=2 + 3 digits
        assert_eq!(i32::MAX.token_count(), 5); // width=3 + 4 digits
        assert_eq!(i32::MIN.token_count(), 5); // width=3 + 4 digits (as u32)
    }

    #[test]
    fn test_i32_port_numbers_respect_vocab_bounds() {
        // Test that common port numbers encode with all tokens < 256
        let port_numbers = vec![80, 443, 3000, 3306, 5432, 6379, 8080, 8888, 27017, 65535];

        for port in port_numbers {
            let mut tokens = Vec::new();
            port.encode(&mut tokens);

            // Verify all tokens are within vocabulary bounds (0-255)
            for (i, &token) in tokens.iter().enumerate() {
                assert!(
                    token < 256,
                    "Port {} produced token {} at position {}, exceeds VOCAB_SIZE",
                    port,
                    token,
                    i
                );
            }

            // Verify round-trip
            let mut token_iter = tokens.into_iter();
            let decoded = i32::decode(&mut token_iter).expect("Decode should succeed");
            assert_eq!(decoded, port, "Port number should round-trip correctly");
        }
    }

    // ========== Box<T> Tests ==========
    #[test]
    fn test_box_encode_decode() {
        let boxed_value = Box::new("hello".to_string());
        encode_decode_test(boxed_value);
    }

    #[test]
    fn test_box_i64() {
        let boxed = Box::new(123456789i64);
        encode_decode_test(boxed);
    }

    #[test]
    fn test_box_token_count() {
        let boxed = Box::new("test".to_string());
        assert_eq!(boxed.token_count(), "test".to_string().token_count());
    }

    // ========== Unit type () Tests ==========
    #[test]
    fn test_unit_encode_decode() {
        encode_decode_test(());
    }

    #[test]
    fn test_unit_token_count() {
        assert_eq!(().token_count(), 1);
    }

    #[test]
    fn test_unit_decode_rejects_non_zero_token() {
        let mut iter = vec![5u32].into_iter();
        let result = <()>::decode(&mut iter);
        assert!(result.is_err());
        match result {
            Err(DecodeError::InvalidData(msg)) => {
                assert!(msg.contains("Expected null token (0)"));
            }
            _ => panic!("Expected InvalidData error"),
        }
    }

    // ========== Error Path Tests ==========
    #[test]
    fn test_bool_decode_unexpected_end_of_stream() {
        let mut empty_iter = vec![].into_iter();
        let result = bool::decode(&mut empty_iter);
        assert!(matches!(
            result,
            Err(DecodeError::UnexpectedEndOfStream { .. })
        ));
    }

    #[test]
    fn test_bool_decode_normalizes_tokens() {
        // Token 0 → false
        let mut iter = vec![0u32].into_iter();
        assert!(!bool::decode(&mut iter).unwrap());

        // Token 1 → true
        let mut iter = vec![1u32].into_iter();
        assert!(bool::decode(&mut iter).unwrap());

        // Token 2 (2 % 2 = 0) → false
        let mut iter = vec![2u32].into_iter();
        assert!(!bool::decode(&mut iter).unwrap());

        // Token 3 (3 % 2 = 1) → true
        let mut iter = vec![3u32].into_iter();
        assert!(bool::decode(&mut iter).unwrap());
    }

    #[test]
    fn test_i32_decode_unexpected_end_of_stream() {
        let mut empty_iter = vec![].into_iter();
        let result = i32::decode(&mut empty_iter);
        assert!(matches!(
            result,
            Err(DecodeError::UnexpectedEndOfStream { .. })
        ));
    }

    #[test]
    fn test_i64_decode_unexpected_end_of_stream_low() {
        let mut iter = vec![].into_iter();
        let result = i64::decode(&mut iter);
        assert!(matches!(
            result,
            Err(DecodeError::UnexpectedEndOfStream { .. })
        ));
    }

    #[test]
    fn test_i64_decode_unexpected_end_of_stream_high() {
        let mut iter = vec![100u32].into_iter(); // Only low, missing high
        let result = i64::decode(&mut iter);
        assert!(matches!(
            result,
            Err(DecodeError::UnexpectedEndOfStream { .. })
        ));
    }

    #[test]
    fn test_f64_decode_unexpected_end_of_stream_low() {
        let mut iter = vec![].into_iter();
        let result = f64::decode(&mut iter);
        assert!(matches!(
            result,
            Err(DecodeError::UnexpectedEndOfStream { .. })
        ));
    }

    #[test]
    fn test_f64_decode_unexpected_end_of_stream_high() {
        let mut iter = vec![100u32].into_iter(); // Only low, missing high
        let result = f64::decode(&mut iter);
        assert!(matches!(
            result,
            Err(DecodeError::UnexpectedEndOfStream { .. })
        ));
    }

    #[test]
    fn test_string_decode_unexpected_end_of_stream_length() {
        let mut empty_iter = vec![].into_iter();
        let result = String::decode(&mut empty_iter);
        assert!(matches!(
            result,
            Err(DecodeError::UnexpectedEndOfStream { .. })
        ));
    }

    #[test]
    fn test_string_decode_unexpected_end_of_stream_bytes() {
        let mut iter = vec![5u32, 65u32, 66u32].into_iter(); // Length=5, but only 2 bytes
        let result = String::decode(&mut iter);
        assert!(matches!(
            result,
            Err(DecodeError::UnexpectedEndOfStream { .. })
        ));
    }

    #[test]
    fn test_string_decode_ascii_validation() {
        // Test UTF-8 byte validation (end-marker encoding)

        // Valid ASCII character
        let mut iter = vec![65u32, 256].into_iter(); // 'A' + END_MARKER
        let result = String::decode(&mut iter);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "A");

        // Valid ASCII at boundary (127 = DEL)
        let mut iter = vec![127u32, 256].into_iter(); // DEL + END_MARKER
        let result = String::decode(&mut iter);
        assert!(result.is_ok());

        // Empty string (just END_MARKER)
        let mut iter = vec![256u32].into_iter(); // Just END_MARKER
        let result = String::decode(&mut iter);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "");

        // Invalid: token >= 256 (beyond UTF-8 byte range) - rejected by token validation
        let mut iter = vec![300u32, 256].into_iter();
        let result = String::decode(&mut iter);
        assert!(matches!(result, Err(DecodeError::InvalidToken(300))));

        // Invalid: token value 500 - rejected by token validation
        let mut iter = vec![500u32, 256].into_iter();
        let result = String::decode(&mut iter);
        assert!(matches!(result, Err(DecodeError::InvalidToken(500))));

        // Note: Standalone bytes 128-255 are accepted by token validation (0-255 range),
        // but may fail UTF-8 validation if they don't form a valid sequence.
        // For example, byte 128 in isolation is a continuation byte and is invalid UTF-8.
        // Valid UTF-8 sequences are tested in round-trip tests with real UTF-8 strings.
    }

    #[test]
    fn test_string_decode_ascii_round_trip() {
        // Test that ASCII strings round-trip correctly

        // Simple ASCII string
        let original = "Hello World";
        let mut tokens = Vec::new();
        original.to_string().encode(&mut tokens);
        let mut iter = tokens.into_iter();
        let decoded = String::decode(&mut iter).unwrap();
        assert_eq!(decoded, original);

        // ASCII with numbers and symbols
        let original = "test-123_v4.5.0@main";
        let mut tokens = Vec::new();
        original.to_string().encode(&mut tokens);
        let mut iter = tokens.into_iter();
        let decoded = String::decode(&mut iter).unwrap();
        assert_eq!(decoded, original);

        // ASCII with special characters
        let original = "actions/checkout@v4";
        let mut tokens = Vec::new();
        original.to_string().encode(&mut tokens);
        let mut iter = tokens.into_iter();
        let decoded = String::decode(&mut iter).unwrap();
        assert_eq!(decoded, original);
    }

    #[test]
    fn test_string_encode_rejects_non_ascii() {
        // With UTF-8 encoding, non-ASCII is now accepted and round-trips correctly
        let non_ascii = "Café"; // Contains é (195, 169 in UTF-8)
        let mut tokens = Vec::new();
        non_ascii.to_string().encode(&mut tokens);

        // Verify encoding succeeded
        assert!(!tokens.is_empty());

        // Verify round-trip
        let mut iter = tokens.into_iter();
        let result = String::decode(&mut iter);
        assert!(result.is_ok(), "UTF-8 string should decode successfully");
        assert_eq!(result.unwrap(), "Café");
    }

    #[test]
    fn test_option_decode_unexpected_end_of_stream() {
        let mut empty_iter = vec![].into_iter();
        let result = <Option<i32>>::decode(&mut empty_iter);
        assert!(matches!(
            result,
            Err(DecodeError::UnexpectedEndOfStream { .. })
        ));
    }

    #[test]
    fn test_option_decode_normalizes_tokens() {
        // Token 0 → None
        let mut iter = vec![0u32].into_iter();
        assert_eq!(<Option<i32>>::decode(&mut iter).unwrap(), None);

        // Token 1 → Some (requires value: width + digit)
        let mut iter = vec![1u32, 0u32, 42u32].into_iter();
        assert_eq!(<Option<i32>>::decode(&mut iter).unwrap(), Some(42));

        // Token 2 (2 % 2 = 0) → None
        let mut iter = vec![2u32].into_iter();
        assert_eq!(<Option<i32>>::decode(&mut iter).unwrap(), None);

        // Token 3 (3 % 2 = 1) → Some
        let mut iter = vec![3u32, 0u32, 99u32].into_iter();
        assert_eq!(<Option<i32>>::decode(&mut iter).unwrap(), Some(99));
    }

    #[test]
    fn test_option_some_decode_inner_fails() {
        // Presence flag=1 (Some), but inner decode fails (missing value)
        let mut iter = vec![1u32].into_iter();
        let result = <Option<i32>>::decode(&mut iter);
        assert!(matches!(
            result,
            Err(DecodeError::UnexpectedEndOfStream { .. })
        ));
    }

    #[test]
    fn test_vec_decode_unexpected_end_of_stream_length() {
        let mut empty_iter = vec![].into_iter();
        let result = <Vec<i32>>::decode(&mut empty_iter);
        assert!(matches!(
            result,
            Err(DecodeError::UnexpectedEndOfStream { .. })
        ));
    }

    #[test]
    fn test_vec_decode_unexpected_end_of_stream_items() {
        let mut iter = vec![3u32, 1u32, 2u32].into_iter(); // Length=3, but only 2 items
        let result = <Vec<i32>>::decode(&mut iter);
        assert!(matches!(
            result,
            Err(DecodeError::UnexpectedEndOfStream { .. })
        ));
    }

    #[test]
    fn test_hashmap_decode_unexpected_end_of_stream_length() {
        let mut empty_iter = vec![].into_iter();
        let result = <FxHashMap<String, i32>>::decode(&mut empty_iter);
        assert!(matches!(
            result,
            Err(DecodeError::UnexpectedEndOfStream { .. })
        ));
    }

    #[test]
    fn test_hashmap_decode_unexpected_end_of_stream_key() {
        let mut iter = vec![1u32].into_iter(); // Length=1, missing key
        let result = <FxHashMap<String, i32>>::decode(&mut iter);
        assert!(matches!(
            result,
            Err(DecodeError::UnexpectedEndOfStream { .. })
        ));
    }

    #[test]
    fn test_hashmap_decode_unexpected_end_of_stream_value() {
        let mut iter = vec![1u32, 4u32, 116, 101, 115, 116].into_iter(); // Length=1, key="test", missing value
        let result = <FxHashMap<String, i32>>::decode(&mut iter);
        assert!(matches!(
            result,
            Err(DecodeError::UnexpectedEndOfStream { .. })
        ));
    }

    #[test]
    fn test_unit_decode_unexpected_end_of_stream() {
        let mut empty_iter = vec![].into_iter();
        let result = <()>::decode(&mut empty_iter);
        assert!(matches!(
            result,
            Err(DecodeError::UnexpectedEndOfStream { .. })
        ));
    }

    // ========== JsonInteger Tests ==========
    #[test]
    fn test_json_integer_signed_and_unsigned_round_trip() {
        for original in [
            JsonInteger::from_i64(i64::MIN),
            JsonInteger::from_i64(-1),
            JsonInteger::from_i64(0),
            JsonInteger::from_u64(u64::MAX),
        ] {
            let mut tokens = Vec::new();
            original.encode(&mut tokens);
            assert_eq!(original.token_count(), tokens.len());

            let mut iter = tokens.into_iter();
            let decoded = JsonInteger::decode(&mut iter).unwrap();
            assert_eq!(decoded, original);
            assert_eq!(decoded.to_number(), original.to_number());
            assert_eq!(iter.next(), None, "Iterator should be fully consumed");
        }
    }

    #[test]
    fn test_json_integer_rejects_fractional_number() {
        let fractional = serde_json::Number::from_f64(1.25).unwrap();
        assert_eq!(JsonInteger::from_number(&fractional), None);
    }

    // ========== serde_json::Number Tests ==========
    #[test]
    fn test_serde_json_number_integer() {
        let num = serde_json::Number::from(12345);
        encode_decode_test(num);
    }

    #[test]
    fn test_serde_json_number_unsigned_integer() {
        let original = serde_json::Number::from(u64::MAX);
        let mut tokens = Vec::new();
        original.encode(&mut tokens);

        let mut iter = tokens.into_iter();
        let decoded = serde_json::Number::decode(&mut iter).unwrap();

        assert_eq!(decoded, original);
        assert_eq!(decoded.as_u64(), Some(u64::MAX));
        assert_eq!(iter.next(), None, "Iterator should be fully consumed");
    }

    #[test]
    fn test_serde_json_number_float() {
        let num = serde_json::Number::from_f64(std::f64::consts::PI).unwrap();
        encode_decode_test(num);
    }

    #[test]
    fn test_serde_json_number_token_count_matches_encoded_length() {
        for num in [
            serde_json::Number::from(12345),
            serde_json::Number::from_f64(std::f64::consts::PI).unwrap(),
        ] {
            let mut tokens = Vec::new();
            num.encode(&mut tokens);
            assert_eq!(num.token_count(), tokens.len());
        }
    }

    #[test]
    fn test_serde_json_number_decode_unexpected_end_of_stream_marker() {
        let mut empty_iter = vec![].into_iter();
        let result = serde_json::Number::decode(&mut empty_iter);
        assert!(matches!(
            result,
            Err(DecodeError::UnexpectedEndOfStream { .. })
        ));
    }

    #[test]
    fn test_serde_json_number_decode_modulo_marker() {
        // With modulo-based decoding, marker 5 becomes valid (5 % 3 = 2 = u64)
        // This should decode as u64 but will fail due to missing payload.
        let mut iter = vec![5u32].into_iter(); // 5 % 3 = 2 (u64)
        let result = serde_json::Number::decode(&mut iter);
        // Should fail with EOF trying to read the unsigned integer payload.
        assert!(matches!(
            result,
            Err(DecodeError::UnexpectedEndOfStream { .. })
        ));
    }

    #[test]
    fn test_serde_json_number_decode_strict_rejects_modulo_marker() {
        let mut recovery_tokens = vec![5u32]; // 5 % 3 = 2 (u64)
        42u64.encode(&mut recovery_tokens);

        let mut iter = recovery_tokens.clone().into_iter();
        let recovered = serde_json::Number::decode(&mut iter).unwrap();
        assert_eq!(recovered, serde_json::Number::from(42u64));
        assert_eq!(iter.next(), None);

        let mut iter = recovery_tokens.into_iter();
        let result = serde_json::Number::decode_strict(&mut iter);
        assert!(matches!(
            result,
            Err(DecodeError::InvalidTokenInContext { token: 5, context })
                if context.type_name == "serde_json::Number"
                    && context.field_name == Some("marker")
                    && context.expected == "number marker 0, 1, or 2"
        ));
    }

    #[test]
    fn test_serde_json_number_decode_integer_missing_value() {
        let mut iter = vec![0u32].into_iter(); // Marker=0 (integer), missing i64
        let result = serde_json::Number::decode(&mut iter);
        assert!(matches!(
            result,
            Err(DecodeError::UnexpectedEndOfStream { .. })
        ));
    }

    #[test]
    fn test_serde_json_number_decode_float_missing_value() {
        let mut iter = vec![1u32].into_iter(); // Marker=1 (float), missing f64
        let result = serde_json::Number::decode(&mut iter);
        assert!(matches!(
            result,
            Err(DecodeError::UnexpectedEndOfStream { .. })
        ));
    }

    #[test]
    fn test_serde_json_number_decode_float_nan_rejected() {
        // Encode NaN as f64 bytes and try to decode
        let nan_bytes = f64::NAN.to_bits().to_le_bytes();
        let mut tokens = vec![1u32]; // Marker=1 (float)
        tokens.extend(nan_bytes.iter().map(|&b| b as u32));
        let mut iter = tokens.into_iter();
        let result = serde_json::Number::decode(&mut iter);
        assert!(matches!(result, Err(DecodeError::InvalidData(_))));
    }

    #[test]
    fn test_serde_json_number_decode_float_inf_rejected() {
        // Encode Infinity as f64 bytes and try to decode
        let inf_bytes = f64::INFINITY.to_bits().to_le_bytes();
        let mut tokens = vec![1u32]; // Marker=1 (float)
        tokens.extend(inf_bytes.iter().map(|&b| b as u32));
        let mut iter = tokens.into_iter();
        let result = serde_json::Number::decode(&mut iter);
        assert!(matches!(result, Err(DecodeError::InvalidData(_))));
    }

    // ========== serde_json::Value Tests ==========
    #[test]
    fn test_serde_json_value_null() {
        encode_decode_test(serde_json::Value::Null);
    }

    #[test]
    fn test_serde_json_value_bool_true() {
        encode_decode_test(serde_json::Value::Bool(true));
    }

    #[test]
    fn test_serde_json_value_bool_false() {
        encode_decode_test(serde_json::Value::Bool(false));
    }

    #[test]
    fn test_serde_json_value_number_integer() {
        encode_decode_test(serde_json::Value::Number(serde_json::Number::from(999)));
    }

    #[test]
    fn test_serde_json_value_number_unsigned_integer() {
        let original = serde_json::Value::Number(serde_json::Number::from(u64::MAX));
        let mut tokens = Vec::new();
        original.encode(&mut tokens);

        let mut iter = tokens.into_iter();
        let decoded = serde_json::Value::decode(&mut iter).unwrap();

        assert_eq!(decoded, original);
        assert_eq!(
            decoded.as_u64(),
            Some(u64::MAX),
            "decoded value should remain a JSON unsigned integer"
        );
        assert_eq!(iter.next(), None, "Iterator should be fully consumed");
    }

    #[test]
    fn test_serde_json_value_number_float() {
        encode_decode_test(serde_json::Value::Number(
            serde_json::Number::from_f64(std::f64::consts::E).unwrap(),
        ));
    }

    #[test]
    fn test_serde_json_value_string() {
        encode_decode_test(serde_json::Value::String("test value".to_string()));
    }

    #[test]
    fn test_serde_json_value_array() {
        encode_decode_test(serde_json::Value::Array(vec![
            serde_json::Value::Null,
            serde_json::Value::Bool(true),
            serde_json::Value::Number(serde_json::Number::from(42)),
        ]));
    }

    #[test]
    fn test_serde_json_value_object() {
        let mut map = serde_json::Map::new();
        map.insert(
            "key".to_string(),
            serde_json::Value::String("val".to_string()),
        );
        encode_decode_test(serde_json::Value::Object(map));
    }

    #[test]
    fn test_serde_json_value_object_encode_matches_legacy_hashmap_order_without_value_clone() {
        let original = serde_json::json!({
            "alpha": {"nested": true, "items": [1, "two"]},
            "beta": 18446744073709551615u64,
            "gamma": null
        });
        let serde_json::Value::Object(obj) = &original else {
            panic!("test fixture should be an object");
        };

        let mut tokens = Vec::new();
        original.encode(&mut tokens);

        let legacy_map: FxHashMap<String, serde_json::Value> = obj
            .iter()
            .map(|(key, value)| (key.clone(), value.clone()))
            .collect();
        let mut legacy_tokens = vec![5];
        legacy_map.encode(&mut legacy_tokens);

        assert_eq!(
            tokens, legacy_tokens,
            "borrowed object encoding must preserve the previous FxHashMap token order"
        );
        assert_eq!(original.token_count(), tokens.len());

        let mut iter = tokens.into_iter();
        let decoded = serde_json::Value::decode(&mut iter).unwrap();
        assert_eq!(decoded, original);
        assert_eq!(iter.next(), None, "Iterator should be fully consumed");
    }

    #[test]
    fn test_serde_json_value_decode_unexpected_end_of_stream() {
        let mut empty_iter = vec![].into_iter();
        let result = serde_json::Value::decode(&mut empty_iter);
        assert!(matches!(
            result,
            Err(DecodeError::UnexpectedEndOfStream { .. })
        ));
    }

    #[test]
    fn test_serde_json_value_decode_type_tag_uses_modulo_recovery() {
        // 11 % 6 = 5, so this recovers as an empty object.
        let mut iter = vec![11u32, 0u32, 0u32].into_iter();
        let result = serde_json::Value::decode(&mut iter).unwrap();
        assert_eq!(result, serde_json::json!({}));
        assert_eq!(iter.next(), None);
    }

    #[test]
    fn test_serde_json_value_decode_strict_rejects_modulo_type_tag() {
        // 7 % 6 = 1, so recovery decodes this as a bool.
        let recovery_tokens = vec![7u32, 1u32];
        let mut iter = recovery_tokens.clone().into_iter();
        let recovered = serde_json::Value::decode(&mut iter).unwrap();
        assert_eq!(recovered, serde_json::json!(true));
        assert_eq!(iter.next(), None);

        let mut iter = recovery_tokens.into_iter();
        let result = serde_json::Value::decode_strict(&mut iter);
        assert!(matches!(
            result,
            Err(DecodeError::InvalidTokenInContext { token: 7, context })
                if context.type_name == "serde_json::Value"
                    && context.field_name == Some("type_tag")
                    && context.expected == "value type tag 0..=5"
        ));
    }

    #[test]
    fn test_serde_json_value_decode_number_missing_subtype() {
        let mut iter = vec![2u32].into_iter(); // Type tag=2 (number), missing subtype
        let result = serde_json::Value::decode(&mut iter);
        assert!(matches!(
            result,
            Err(DecodeError::UnexpectedEndOfStream { .. })
        ));
    }

    #[test]
    fn test_serde_json_value_decode_number_subtype_uses_number_recovery() {
        // Type tag 2 is number; marker 5 % 3 = 2, so this decodes a u64.
        let mut tokens = vec![2u32, 5u32];
        42u64.encode(&mut tokens);
        let mut iter = tokens.into_iter();
        let result = serde_json::Value::decode(&mut iter).unwrap();
        assert_eq!(result, serde_json::json!(42u64));
        assert_eq!(iter.next(), None);
    }

    #[test]
    fn test_serde_json_value_decode_number_i64_missing_value() {
        let mut iter = vec![2u32, 0u32].into_iter(); // Type=2, subtype=0 (i64), missing value
        let result = serde_json::Value::decode(&mut iter);
        assert!(matches!(
            result,
            Err(DecodeError::UnexpectedEndOfStream { .. })
        ));
    }

    #[test]
    fn test_serde_json_value_decode_number_f64_missing_value() {
        let mut iter = vec![2u32, 1u32].into_iter(); // Type=2, subtype=1 (f64), missing value
        let result = serde_json::Value::decode(&mut iter);
        assert!(matches!(
            result,
            Err(DecodeError::UnexpectedEndOfStream { .. })
        ));
    }

    #[test]
    fn test_serde_json_value_decode_number_f64_nan_rejected() {
        let nan_bytes = f64::NAN.to_bits().to_le_bytes();
        let mut tokens = vec![2u32, 1u32]; // Type=2, subtype=1
        tokens.extend(nan_bytes.iter().map(|&b| b as u32));
        let mut iter = tokens.into_iter();
        let result = serde_json::Value::decode(&mut iter);
        assert!(matches!(result, Err(DecodeError::InvalidData(_))));
    }

    #[test]
    fn test_serde_json_value_decode_number_string_missing_value() {
        let mut iter = vec![2u32, 3u32].into_iter(); // Type=2, subtype=3 (string), missing string
        let result = serde_json::Value::decode(&mut iter);
        assert!(matches!(
            result,
            Err(DecodeError::UnexpectedEndOfStream { .. })
        ));
    }

    #[test]
    fn test_serde_json_value_decode_string_missing_value() {
        let mut iter = vec![3u32].into_iter(); // Type tag=3 (string), missing string
        let result = serde_json::Value::decode(&mut iter);
        assert!(matches!(
            result,
            Err(DecodeError::UnexpectedEndOfStream { .. })
        ));
    }

    #[test]
    fn test_serde_json_value_decode_array_missing_value() {
        let mut iter = vec![4u32].into_iter(); // Type tag=4 (array), missing array
        let result = serde_json::Value::decode(&mut iter);
        assert!(matches!(
            result,
            Err(DecodeError::UnexpectedEndOfStream { .. })
        ));
    }

    #[test]
    fn test_serde_json_value_decode_object_missing_value() {
        let mut iter = vec![5u32].into_iter(); // Type tag=5 (object), missing object
        let result = serde_json::Value::decode(&mut iter);
        assert!(matches!(
            result,
            Err(DecodeError::UnexpectedEndOfStream { .. })
        ));
    }

    // ========== AdditionalValue Tests ==========
    #[test]
    fn test_additional_value_string() {
        encode_decode_test(AdditionalValue::String("test".to_string()));
    }

    #[test]
    fn test_additional_value_number() {
        encode_decode_test(AdditionalValue::Number(123.456));
    }

    #[test]
    fn test_additional_value_bool_true() {
        encode_decode_test(AdditionalValue::Bool(true));
    }

    #[test]
    fn test_additional_value_bool_false() {
        encode_decode_test(AdditionalValue::Bool(false));
    }

    #[test]
    fn test_additional_value_null() {
        encode_decode_test(AdditionalValue::Null);
    }

    #[test]
    fn test_additional_value_decode_unexpected_end_of_stream() {
        let mut empty_iter = vec![].into_iter();
        let result = AdditionalValue::decode(&mut empty_iter);
        assert!(matches!(
            result,
            Err(DecodeError::UnexpectedEndOfStream { .. })
        ));
    }

    #[test]
    fn test_additional_value_decode_normalizes_type_tag() {
        // Type tag 0 → String (end-marker encoding: bytes + END_STRING_MARKER)
        let mut iter = vec![0u32, 116, 101, 115, 116, 256].into_iter(); // "test"
        let result = AdditionalValue::decode(&mut iter).unwrap();
        assert!(matches!(result, AdditionalValue::String(_)));

        // Type tag 4 (4 % 4 = 0) → String (end-marker encoding)
        let mut iter = vec![4u32, 97, 256].into_iter(); // "a"
        let result = AdditionalValue::decode(&mut iter).unwrap();
        assert!(matches!(result, AdditionalValue::String(_)));

        // Type tag 1 → Number
        let mut iter = vec![1u32, 0, 0, 0, 0, 0, 0, 0, 0].into_iter(); // Number (requires 8 f64 tokens)
        let result = AdditionalValue::decode(&mut iter);
        assert!(result.is_ok());

        // Type tag 5 (5 % 4 = 1) → Number
        let mut iter = vec![5u32, 0, 0, 0, 0, 0, 0, 0, 0].into_iter();
        let result = AdditionalValue::decode(&mut iter);
        assert!(result.is_ok());

        // Type tag 2 → Bool
        let mut iter = vec![2u32, 1].into_iter();
        let result = AdditionalValue::decode(&mut iter).unwrap();
        assert!(matches!(result, AdditionalValue::Bool(true)));

        // Type tag 6 (6 % 4 = 2) → Bool
        let mut iter = vec![6u32, 0].into_iter();
        let result = AdditionalValue::decode(&mut iter).unwrap();
        assert!(matches!(result, AdditionalValue::Bool(false)));

        // Type tag 3 → Null
        let mut iter = vec![3u32].into_iter();
        let result = AdditionalValue::decode(&mut iter).unwrap();
        assert!(matches!(result, AdditionalValue::Null));

        // Type tag 7 (7 % 4 = 3) → Null
        let mut iter = vec![7u32].into_iter();
        let result = AdditionalValue::decode(&mut iter).unwrap();
        assert!(matches!(result, AdditionalValue::Null));
    }

    #[test]
    fn test_additional_value_decode_strict_rejects_modulo_type_tag() {
        // 6 % 4 = 2, so recovery decodes this as a bool.
        let recovery_tokens = vec![6u32, 1u32];
        let mut iter = recovery_tokens.clone().into_iter();
        let recovered = AdditionalValue::decode(&mut iter).unwrap();
        assert!(matches!(recovered, AdditionalValue::Bool(true)));
        assert_eq!(iter.next(), None);

        let mut iter = recovery_tokens.into_iter();
        let result = AdditionalValue::decode_strict(&mut iter);
        assert!(matches!(
            result,
            Err(DecodeError::InvalidTokenInContext { token: 6, context })
                if context.type_name == "AdditionalValue"
                    && context.field_name == Some("type_tag")
                    && context.expected == "property type tag 0..=3"
        ));
    }

    #[test]
    fn test_additional_value_decode_string_missing_value() {
        let mut iter = vec![0u32].into_iter(); // Type tag=0 (string), missing string
        let result = AdditionalValue::decode(&mut iter);
        assert!(matches!(
            result,
            Err(DecodeError::UnexpectedEndOfStream { .. })
        ));
    }

    #[test]
    fn test_additional_value_decode_number_missing_value() {
        let mut iter = vec![1u32].into_iter(); // Type tag=1 (number), missing f64
        let result = AdditionalValue::decode(&mut iter);
        assert!(matches!(
            result,
            Err(DecodeError::UnexpectedEndOfStream { .. })
        ));
    }

    #[test]
    fn test_additional_value_decode_bool_missing_value() {
        let mut iter = vec![2u32].into_iter(); // Type tag=2 (bool), missing bool
        let result = AdditionalValue::decode(&mut iter);
        assert!(matches!(
            result,
            Err(DecodeError::UnexpectedEndOfStream { .. })
        ));
    }

    // Helper function for encode/decode testing
    fn encode_decode_test<T: JsonTokenEncoder + std::fmt::Debug + PartialEq>(original: T) {
        let mut tokens = Vec::new();
        original.encode(&mut tokens);

        let mut iter = tokens.into_iter();
        let decoded = T::decode(&mut iter).unwrap();

        assert_eq!(original, decoded);
        assert_eq!(iter.next(), None, "Iterator should be fully consumed");
    }
}

// ========== Partial Decode Tests ==========
#[cfg(test)]
mod partial_decode_tests {
    use super::*;
    use crate::tokenizer::{END_STRING_MARKER, PartialStringResult, PartialVecResult};

    // ========== String Partial Decode Tests ==========

    #[test]
    fn test_partial_string_complete() {
        // Complete string: "hello" + END_MARKER
        let tokens: Vec<u32> = vec![104, 101, 108, 108, 111, END_STRING_MARKER];
        let mut iter = tokens.into_iter();

        let result = decode_string_prefix(&mut iter);

        assert_eq!(
            result,
            PartialStringResult {
                value: "hello".to_string(),
                complete: true,
                bytes_decoded: 5,
            }
        );
    }

    #[test]
    fn test_partial_string_truncated() {
        // Truncated string: "hel" (missing "lo" and END_MARKER)
        let tokens: Vec<u32> = vec![104, 101, 108];
        let mut iter = tokens.into_iter();

        let result = decode_string_prefix(&mut iter);

        assert_eq!(
            result,
            PartialStringResult {
                value: "hel".to_string(),
                complete: false,
                bytes_decoded: 3,
            }
        );
    }

    #[test]
    fn test_partial_string_empty_stream() {
        // Empty token stream
        let tokens: Vec<u32> = vec![];
        let mut iter = tokens.into_iter();

        let result = decode_string_prefix(&mut iter);

        assert_eq!(
            result,
            PartialStringResult {
                value: "".to_string(),
                complete: false,
                bytes_decoded: 0,
            }
        );
    }

    #[test]
    fn test_partial_string_empty_complete() {
        // Empty string (just END_MARKER)
        let tokens: Vec<u32> = vec![END_STRING_MARKER];
        let mut iter = tokens.into_iter();

        let result = decode_string_prefix(&mut iter);

        assert_eq!(
            result,
            PartialStringResult {
                value: "".to_string(),
                complete: true,
                bytes_decoded: 0,
            }
        );
    }

    #[test]
    fn test_partial_string_invalid_token_stops() {
        // Invalid token (> 255, != END_MARKER) stops decode
        let tokens: Vec<u32> = vec![104, 101, 108, 300]; // "hel" then invalid
        let mut iter = tokens.into_iter();

        let result = decode_string_prefix(&mut iter);

        assert_eq!(
            result,
            PartialStringResult {
                value: "hel".to_string(),
                complete: false,
                bytes_decoded: 3,
            }
        );
    }

    #[test]
    fn test_partial_string_github_action_truncated() {
        // Real-world example: "actions/check" (truncated from "actions/checkout@v4")
        let partial = "actions/check";
        let tokens: Vec<u32> = partial.bytes().map(|b| b as u32).collect();
        let mut iter = tokens.into_iter();

        let result = decode_string_prefix(&mut iter);

        assert_eq!(result.value, "actions/check");
        assert!(!result.complete);
        assert_eq!(result.bytes_decoded, 13);
    }

    // ========== Vec Partial Decode Tests ==========

    #[test]
    fn test_partial_vec_complete() {
        // Complete vec: length=3, three i32 values
        // i32 encoding: [width, digit] for small values
        let mut tokens = Vec::new();
        let vec = vec![1i32, 2i32, 3i32];
        vec.encode(&mut tokens);

        let mut iter = tokens.into_iter();
        let result: PartialVecResult<i32> = decode_vec_prefix(&mut iter);

        assert_eq!(result.items, vec![1, 2, 3]);
        assert_eq!(result.expected_length, Some(3));
        assert!(result.complete);
    }

    #[test]
    fn test_partial_vec_truncated_elements() {
        // Vec with length=3 but only 2 complete elements
        let mut tokens = Vec::new();
        let vec = vec![1i32, 2i32, 3i32];
        vec.encode(&mut tokens);

        // Remove last element's tokens (2 tokens for small i32)
        tokens.truncate(tokens.len() - 2);

        let mut iter = tokens.into_iter();
        let result: PartialVecResult<i32> = decode_vec_prefix(&mut iter);

        assert_eq!(result.items, vec![1, 2]);
        assert_eq!(result.expected_length, Some(3));
        assert!(!result.complete);
    }

    #[test]
    fn test_partial_vec_no_length() {
        // Empty stream (no length token)
        let tokens: Vec<u32> = vec![];
        let mut iter = tokens.into_iter();

        let result: PartialVecResult<i32> = decode_vec_prefix(&mut iter);

        assert!(result.items.is_empty());
        assert_eq!(result.expected_length, None);
        assert!(!result.complete);
    }

    #[test]
    fn test_partial_vec_length_only() {
        // Only length token, no elements
        // Length encoding: [width=0, digit=3] for length=3
        let tokens: Vec<u32> = vec![0, 3];
        let mut iter = tokens.into_iter();

        let result: PartialVecResult<i32> = decode_vec_prefix(&mut iter);

        assert!(result.items.is_empty());
        assert_eq!(result.expected_length, Some(3));
        assert!(!result.complete);
    }

    #[test]
    fn test_partial_vec_empty_complete() {
        // Empty vec (length=0)
        let tokens: Vec<u32> = vec![0, 0]; // width=0, digit=0
        let mut iter = tokens.into_iter();

        let result: PartialVecResult<i32> = decode_vec_prefix(&mut iter);

        assert!(result.items.is_empty());
        assert_eq!(result.expected_length, Some(0));
        assert!(result.complete);
    }

    #[test]
    fn test_partial_vec_of_strings_truncated() {
        // Vec of strings, partially decoded
        let mut tokens = Vec::new();
        let vec = vec!["hello".to_string(), "world".to_string()];
        vec.encode(&mut tokens);

        // Keep only length + first string + partial second string
        // First string is: h(104), e(101), l(108), l(108), o(111), END(256)
        // Truncate in middle of second string
        let truncate_point = 2 + 6 + 3; // length(2) + first_string(6) + partial_second(3)
        tokens.truncate(truncate_point);

        let mut iter = tokens.into_iter();
        let result: PartialVecResult<String> = decode_vec_prefix(&mut iter);

        assert_eq!(result.items.len(), 1);
        assert_eq!(result.items[0], "hello");
        assert_eq!(result.expected_length, Some(2));
        assert!(!result.complete);
    }
}
