//! Defines the core trait for encoding and decoding schema-driven types.

pub mod optional_field;
pub mod primitives;

pub use primitives::JsonInteger;

use std::error::Error;
use std::fmt;

/// Context information for UnexpectedEndOfStream errors.
///
/// Provides human-readable information about where in the schema
/// decoding failed, making errors actionable for debugging.
#[derive(Debug, PartialEq, Clone)]
pub struct EOFContext {
    /// The type name where EOF occurred (e.g., "GithubWorkflow")
    pub type_name: &'static str,

    /// The field name being decoded, if applicable (e.g., Some("name"))
    pub field_name: Option<&'static str>,

    /// Human-readable description of what was expected
    /// Examples: "presence flag", "string length", "enum variant index"
    pub expected: &'static str,
}

impl EOFContext {
    /// Creates a new EOF context
    pub fn new(
        type_name: &'static str,
        field_name: Option<&'static str>,
        expected: &'static str,
    ) -> Self {
        Self {
            type_name,
            field_name,
            expected,
        }
    }
}

/// Represents an error that can occur during the decoding process.
#[derive(Debug, PartialEq)]
pub enum DecodeError {
    /// The token stream ended unexpectedly.
    ///
    /// Contains context about what was expected, enabling:
    /// - Clear error messages showing expected values
    /// - LLM token masking for constrained generation
    /// - Interactive debugging with valid token suggestions
    UnexpectedEndOfStream {
        /// Context describing what was expected when EOF occurred
        context: EOFContext,
    },
    /// An invalid token was encountered for a given type.
    InvalidToken(u32),
    /// An invalid token was encountered while decoding a schema field.
    InvalidTokenInContext {
        /// Token value that was outside the valid range for the decoder.
        token: u32,
        /// Context describing the schema field being decoded.
        context: EOFContext,
    },
    /// An error occurred while decoding a UTF-8 string.
    Utf8Error,
    /// Invalid data encountered during validation.
    InvalidData(String),
}

impl fmt::Display for DecodeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DecodeError::UnexpectedEndOfStream { context } => {
                write!(
                    f,
                    "Unexpected end of token stream while decoding {}",
                    context.type_name
                )?;

                if let Some(field) = context.field_name {
                    write!(f, ".{}", field)?;
                }

                write!(f, ": expected {}", context.expected)?;

                Ok(())
            }
            DecodeError::InvalidToken(token) => {
                write!(f, "Invalid token '{}' for this type", token)
            }
            DecodeError::InvalidTokenInContext { token, context } => {
                write!(
                    f,
                    "Invalid token '{}' while decoding {}",
                    token, context.type_name
                )?;

                if let Some(field) = context.field_name {
                    write!(f, ".{}", field)?;
                }

                write!(f, ": expected {}", context.expected)
            }
            DecodeError::Utf8Error => write!(f, "Failed to decode UTF-8 string from tokens"),
            DecodeError::InvalidData(msg) => write!(f, "Invalid data: {}", msg),
        }
    }
}

impl Error for DecodeError {}

/// A trait for types that can be converted to and from a sequence of integer tokens.
///
/// This trait will be implemented by the types generated from a JSON schema.
pub trait JsonTokenEncoder: Sized {
    /// Encodes the type into a sequence of tokens.
    /// - `tokens` - A mutable vector to which the encoded tokens will be appended.
    fn encode(&self, tokens: &mut Vec<u32>);

    /// Decodes a sequence of tokens into an instance of the type.
    /// - `tokens` - A mutable iterator over the token stream.
    fn decode(tokens: &mut impl Iterator<Item = u32>) -> Result<Self, DecodeError>;

    /// Decodes one value while rejecting strict token-value violations.
    ///
    /// Implementations may override this when their recovery `decode()` path
    /// normalizes finite-choice tokens. Types without recovery normalization
    /// can keep the default implementation.
    fn decode_strict(tokens: &mut impl Iterator<Item = u32>) -> Result<Self, DecodeError> {
        Self::decode(tokens)
    }

    /// Decodes one complete value and rejects any unconsumed trailing tokens.
    ///
    /// This uses the same recovery semantics as `decode()` for the value itself,
    /// then enforces whole-stream consumption.
    fn decode_full<I>(tokens: I) -> Result<Self, DecodeError>
    where
        I: IntoIterator<Item = u32>,
    {
        let mut iter = BufferedTokenIterator::new(tokens.into_iter());
        let value = Self::decode(&mut iter)?;
        let trailing = iter.remaining();
        if trailing == 0 {
            Ok(value)
        } else {
            Err(DecodeError::InvalidData(format!(
                "decode_full left {} trailing token{} after decoding {}",
                trailing,
                if trailing == 1 { "" } else { "s" },
                std::any::type_name::<Self>()
            )))
        }
    }

    /// Decodes one complete value with strict token-value semantics and rejects
    /// any unconsumed trailing tokens.
    fn decode_strict_full<I>(tokens: I) -> Result<Self, DecodeError>
    where
        I: IntoIterator<Item = u32>,
    {
        let mut iter = BufferedTokenIterator::new(tokens.into_iter());
        let value = Self::decode_strict(&mut iter)?;
        let trailing = iter.remaining();
        if trailing == 0 {
            Ok(value)
        } else {
            Err(DecodeError::InvalidData(format!(
                "decode_strict_full left {} trailing token{} after decoding {}",
                trailing,
                if trailing == 1 { "" } else { "s" },
                std::any::type_name::<Self>()
            )))
        }
    }

    /// Returns the number of tokens this value will consume when encoded.
    /// This method should be efficient and avoid actual encoding when possible.
    fn token_count(&self) -> usize;
}

// ============================================================================
// Partial Decode Support for Streaming
// ============================================================================

/// Result of partial string decoding from a potentially truncated token stream.
///
/// Used for streaming LLM generation where the string may be incomplete.
#[derive(Debug, Clone, PartialEq)]
pub struct PartialStringResult {
    /// Decoded string content (may be incomplete).
    /// Uses lossy UTF-8 conversion for partial sequences.
    pub value: String,
    /// Whether the END_STRING_MARKER was found (complete string).
    pub complete: bool,
    /// Number of bytes successfully decoded.
    pub bytes_decoded: usize,
}

/// Result of partial vector decoding from a potentially truncated token stream.
///
/// Used for streaming LLM generation where not all elements may be present.
#[derive(Debug, Clone, PartialEq)]
pub struct PartialVecResult<T> {
    /// Elements successfully decoded.
    pub items: Vec<T>,
    /// Expected total length (None if length token was missing).
    pub expected_length: Option<usize>,
    /// Whether all expected elements were decoded.
    pub complete: bool,
}

/// Marker token for end of string (first token after byte range 0-255).
pub const END_STRING_MARKER: u32 = 256;

/// Default maximum decoded collection length for fallback runtime decoders.
pub const DEFAULT_MAX_COLLECTION_LENGTH: usize = 2048;

/// Decode a String with fallback to partial decode for streaming mode.
///
/// This function attempts full String decode first. If that fails (truncated stream),
/// it returns what could be decoded as a partial string. This is used by generated
/// `decode_prefix` methods for String fields.
///
/// # Returns
/// - `(value, complete, bytes_decoded)` tuple
///
/// # Note
/// This function is designed for streaming decode where partial results are acceptable.
/// For strict decoding, use `String::decode()` directly.
#[inline]
pub fn decode_string_streaming(tokens: &mut impl Iterator<Item = u32>) -> (String, bool, usize) {
    let result = primitives::decode_string_prefix(tokens);
    (result.value, result.complete, result.bytes_decoded)
}

/// Decode a `Vec<T>` with fallback to partial decode for streaming mode.
///
/// This function returns elements that could be decoded, even if the expected
/// count wasn't reached. This is used by generated `decode_prefix` methods
/// for Vec fields.
///
/// # Returns
/// - `(items, complete, expected_length)` tuple
///
/// # Note
/// This function is designed for streaming decode where partial results are acceptable.
/// For strict decoding, use `Vec::<T>::decode()` directly.
#[inline]
pub fn decode_vec_streaming<T: JsonTokenEncoder>(
    tokens: &mut impl Iterator<Item = u32>,
) -> (Vec<T>, bool, Option<usize>) {
    let result = primitives::decode_vec_prefix::<T>(tokens);
    (result.items, result.complete, result.expected_length)
}

/// A buffered token iterator that supports backtracking for untagged enum decoding.
///
/// This solves the fundamental issue where untagged enums need to try multiple
/// variant decoders sequentially, requiring the ability to "undo" token consumption
/// when a variant decode fails.
pub struct BufferedTokenIterator {
    tokens: Vec<u32>,
    position: usize,
}

impl BufferedTokenIterator {
    /// Creates a new BufferedTokenIterator from a token stream
    pub fn new(tokens: impl Iterator<Item = u32>) -> Self {
        Self {
            tokens: tokens.collect(),
            position: 0,
        }
    }

    /// Creates a BufferedTokenIterator from an owned token buffer without copying it.
    pub fn from_vec(tokens: Vec<u32>) -> Self {
        Self {
            tokens,
            position: 0,
        }
    }

    /// Creates a BufferedTokenIterator with exactly the specified number of tokens
    ///
    /// This is used for length-prefixed enum decoding to avoid over-consuming
    /// tokens from the parent iterator. Takes exactly `count` tokens and returns
    /// an error if there aren't enough tokens available.
    pub fn with_exact_count(
        tokens: &mut impl Iterator<Item = u32>,
        count: usize,
    ) -> Result<Self, DecodeError> {
        crate::time_operation!("buffered_iterator_with_exact_count");
        let collected: Vec<u32> = tokens.take(count).collect();
        if collected.len() != count {
            return Err(DecodeError::UnexpectedEndOfStream {
                context: EOFContext::new("BufferedTokenIterator", None, "buffered tokens"),
            });
        }
        Ok(Self {
            tokens: collected,
            position: 0,
        })
    }

    /// Creates a checkpoint of the current position for potential rollback
    pub fn checkpoint(&self) -> usize {
        self.position
    }

    /// Rolls back to a previous checkpoint position
    pub fn rollback(&mut self, checkpoint: usize) {
        self.position = checkpoint;
    }

    /// Returns the number of remaining tokens
    pub fn remaining(&self) -> usize {
        self.tokens.len().saturating_sub(self.position)
    }

    /// Returns true if there are no more tokens to consume
    pub fn is_empty(&self) -> bool {
        self.position >= self.tokens.len()
    }

    /// Peeks at the next token without consuming it
    ///
    /// Returns None if there are no more tokens.
    /// This is useful for lookahead-based decoding strategies.
    pub fn peek(&self) -> Option<u32> {
        if self.position < self.tokens.len() {
            Some(self.tokens[self.position])
        } else {
            None
        }
    }

    /// Peeks at a token at the specified offset from current position
    ///
    /// Returns None if the offset is beyond the available tokens.
    /// Offset 0 is equivalent to peek(), offset 1 is the token after next, etc.
    pub fn peek_at(&self, offset: usize) -> Option<u32> {
        let index = self.position + offset;
        if index < self.tokens.len() {
            Some(self.tokens[index])
        } else {
            None
        }
    }
}

impl Iterator for BufferedTokenIterator {
    type Item = u32;

    fn next(&mut self) -> Option<Self::Item> {
        if self.position < self.tokens.len() {
            let token = self.tokens[self.position];
            self.position += 1;
            Some(token)
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_buffered_iterator_peek() {
        let tokens = vec![1, 2, 3, 4, 5];
        let mut iter = BufferedTokenIterator::new(tokens.into_iter());

        assert_eq!(iter.peek(), Some(1));
        assert_eq!(iter.peek(), Some(1)); // Peek doesn't consume
        assert_eq!(iter.next(), Some(1)); // Now consume
        assert_eq!(iter.peek(), Some(2));
        assert_eq!(iter.next(), Some(2));
        assert_eq!(iter.next(), Some(3));
        assert_eq!(iter.peek(), Some(4));
        assert_eq!(iter.next(), Some(4));
        assert_eq!(iter.next(), Some(5));
        assert_eq!(iter.peek(), None); // No more tokens
        assert_eq!(iter.next(), None);
    }

    #[test]
    fn test_buffered_iterator_from_vec_reuses_owned_tokens() {
        let mut iter = BufferedTokenIterator::from_vec(vec![10, 20, 30]);

        assert_eq!(iter.remaining(), 3);
        assert_eq!(iter.peek(), Some(10));
        assert_eq!(iter.next(), Some(10));
        assert_eq!(iter.remaining(), 2);
        assert_eq!(iter.next(), Some(20));
        assert_eq!(iter.next(), Some(30));
        assert!(iter.is_empty());
    }

    #[test]
    fn test_buffered_iterator_peek_at() {
        let tokens = vec![10, 20, 30, 40, 50];
        let iter = BufferedTokenIterator::new(tokens.into_iter());

        // Test peek_at with various offsets
        assert_eq!(iter.peek_at(0), Some(10)); // Same as peek()
        assert_eq!(iter.peek_at(1), Some(20));
        assert_eq!(iter.peek_at(2), Some(30));
        assert_eq!(iter.peek_at(3), Some(40));
        assert_eq!(iter.peek_at(4), Some(50));
        assert_eq!(iter.peek_at(5), None); // Beyond end

        // Verify original peek() still works
        assert_eq!(iter.peek(), Some(10));
    }

    #[test]
    fn test_buffered_iterator_checkpoint_and_rollback() {
        let tokens = vec![10, 20, 30];
        let mut iter = BufferedTokenIterator::new(tokens.into_iter());

        let checkpoint = iter.checkpoint();
        assert_eq!(iter.next(), Some(10));
        assert_eq!(iter.next(), Some(20));
        assert_eq!(iter.remaining(), 1);

        iter.rollback(checkpoint);
        assert_eq!(iter.remaining(), 3);
        assert_eq!(iter.next(), Some(10)); // Start over
    }
}
