//! Rendering for the standalone tokenizer support module embedded in generated
//! Rust output.
//!
//! The helpers in this module assemble the shared decode errors, token
//! iterator, primitive encoders, streaming prefix helpers, JSON value support,
//! and variable-base utilities that generated schema-specific types call.

use proc_macro2::TokenStream;
use quote::quote;

use super::Grammar;

mod collections;
mod json;
mod optional_field;
mod primitives;
mod streaming;
mod string;
mod variable_base;

impl Grammar {
    /// Generate standalone tokenizer module
    ///
    /// This makes ALL generated code work standalone - users can copy it anywhere.
    /// The generated variable-base helpers use the grammar's configured
    /// encoding base, which is derived from `max_base_vocab` when present.
    fn max_token_width_for_base(base: u32) -> u32 {
        let mut width = 1u32;
        let mut max_value = base as u64;
        while max_value <= u32::MAX as u64 {
            width += 1;
            max_value = max_value.saturating_mul(base as u64);
        }
        width
    }

    fn base_power_u32(base: u32, exponent: u32) -> u32 {
        let mut value = 1u64;
        for _ in 0..exponent {
            value = value.saturating_mul(base as u64);
        }
        u32::try_from(value).expect("base power should fit in generated width branch")
    }

    fn generate_fixed_base_encode_chain(base: u32, max_width: u32) -> TokenStream {
        let value_expr = quote! { value };
        let mut branch = Self::generate_fixed_base_encode_for_width(base, max_width, &value_expr);

        for width in (1..max_width).rev() {
            let threshold = Self::base_power_u32(base, width);
            let encode_for_width =
                Self::generate_fixed_base_encode_for_width(base, width, &value_expr);
            branch = quote! {
                if value < #threshold {
                    #encode_for_width
                } else {
                    #branch
                }
            };
        }

        branch
    }

    fn generate_fixed_base_encode_for_width(
        base: u32,
        width: u32,
        value_expr: &TokenStream,
    ) -> TokenStream {
        let width_indicator = width - 1;
        let digit_pushes = (0..width).rev().map(|power| {
            let divisor = Self::base_power_u32(base, power);
            if power == 0 {
                quote! {
                    tokens.push(#value_expr % GENERATED_ENCODING_BASE);
                }
            } else {
                quote! {
                    tokens.push((#value_expr / #divisor) % GENERATED_ENCODING_BASE);
                }
            }
        });

        quote! {
            tokens.push(#width_indicator);
            #(#digit_pushes)*
        }
    }

    fn generate_fixed_base_token_count_chain(base: u32, max_width: u32) -> TokenStream {
        let mut branch = {
            let count = max_width as usize + 1;
            quote! { #count }
        };

        for width in (1..max_width).rev() {
            let threshold = Self::base_power_u32(base, width);
            let count = width as usize + 1;
            branch = quote! {
                if value < #threshold {
                    #count
                } else {
                    #branch
                }
            };
        }

        branch
    }

    pub(super) fn generate_tokenizer_module(&self) -> TokenStream {
        // Capture encoding_base and max_collection_length for use in quote! blocks
        let encoding_base = self.encoding_base;
        let max_collection_length = self.max_collection_length;
        let max_token_width = Self::max_token_width_for_base(encoding_base);
        let fixed_base_encode_chain =
            Self::generate_fixed_base_encode_chain(encoding_base, max_token_width);
        let fixed_base_token_count_chain =
            Self::generate_fixed_base_token_count_chain(encoding_base, max_token_width);

        // Check if we have vocab constraints from the definitions
        let has_vocab_constraint = self
            .definitions
            .constants
            .iter()
            .any(|c| c.to_string().contains("MAX_VOCAB"));

        let error_enum = quote! {
            // Standalone error types (embedded for wheel compatibility)
            /// Context information for UnexpectedEndOfStream errors
            #[derive(Debug, PartialEq, Clone)]
            pub struct EOFContext {
                pub type_name: &'static str,
                pub field_name: Option<&'static str>,
                pub expected: &'static str,
            }

            impl EOFContext {
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

            /// Decoding errors
            #[derive(Debug, PartialEq)]
            pub enum DecodeError {
                UnexpectedEndOfStream { context: EOFContext },
                InvalidToken(u32),
                InvalidTokenInContext { token: u32, context: EOFContext },
                Utf8Error,
                InvalidData(String),
            }

            impl std::fmt::Display for DecodeError {
                fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
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
                                token,
                                context.type_name
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

            impl std::error::Error for DecodeError {}
        };

        let field_context_helper = quote! {
            #[inline]
            pub fn with_schema_field_decode_context<T>(
                result: Result<T, DecodeError>,
                type_name: &'static str,
                field_name: &'static str,
            ) -> Result<T, DecodeError> {
                match result {
                    Ok(value) => Ok(value),
                    Err(DecodeError::UnexpectedEndOfStream { context }) => {
                        Err(DecodeError::UnexpectedEndOfStream {
                            context: EOFContext::new(type_name, Some(field_name), context.expected),
                        })
                    }
                    Err(DecodeError::InvalidToken(token)) => {
                        Err(DecodeError::InvalidTokenInContext {
                            token,
                            context: EOFContext::new(type_name, Some(field_name), "valid token for field"),
                        })
                    }
                    Err(DecodeError::InvalidTokenInContext { token, context }) => {
                        Err(DecodeError::InvalidTokenInContext {
                            token,
                            context: EOFContext::new(
                                type_name,
                                Some(field_name),
                                context.expected,
                            ),
                        })
                    }
                    Err(DecodeError::InvalidData(message)) => {
                        Err(DecodeError::InvalidData(format!(
                            "{}.{}: {}",
                            type_name, field_name, message
                        )))
                    }
                    Err(error) => Err(error),
                }
            }
        };

        let trait_def = quote! {
            pub trait JsonTokenEncoder: Sized {
                fn encode(&self, tokens: &mut Vec<u32>);
                fn decode(tokens: &mut impl Iterator<Item = u32>) -> Result<Self, DecodeError>;
                fn decode_strict(tokens: &mut impl Iterator<Item = u32>) -> Result<Self, DecodeError> {
                    Self::decode(tokens)
                }
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
                fn token_count(&self) -> usize;
            }
        };

        // Buffered iterator for token consumption tracking (negligible overhead)
        let buffered_iterator = quote! {
            /// Iterator with token consumption tracking
            ///
            /// Tracks position within token stream to enable surplus token detection.
            /// Overhead: ~1 CPU cycle per token (negligible).
            pub struct BufferedTokenIterator {
                tokens: Vec<u32>,
                position: usize,
            }

            impl BufferedTokenIterator {
                /// Create new buffered iterator from any iterator
                #[inline]
                pub fn new(tokens: impl Iterator<Item = u32>) -> Self {
                    Self { tokens: tokens.collect(), position: 0 }
                }

                /// Create a buffered iterator from an owned token buffer.
                #[inline]
                pub fn from_vec(tokens: Vec<u32>) -> Self {
                    Self { tokens, position: 0 }
                }

                /// Get number of remaining tokens
                #[inline]
                pub fn remaining(&self) -> usize {
                    self.tokens.len().saturating_sub(self.position)
                }
            }

            impl Iterator for BufferedTokenIterator {
                type Item = u32;

                #[inline]
                fn next(&mut self) -> Option<u32> {
                    if self.position < self.tokens.len() {
                        let token = self.tokens[self.position];
                        self.position += 1;
                        Some(token)
                    } else {
                        None
                    }
                }
            }
        };

        // Variable-base encoding helper functions (embedded for wheel compatibility)
        let variable_base_helpers = variable_base::render_variable_base_helpers(
            encoding_base,
            &fixed_base_encode_chain,
            &fixed_base_token_count_chain,
        );

        let primitives = primitives::render_primitive_impls();

        // Extract string encoding mode from compilation constraints (defaults to UTF-8)
        let string_encoding = self
            .compilation_constraints
            .as_ref()
            .map(|c| c.string_encoding)
            .unwrap_or_default();

        // Generate string encoding constants based on mode
        let end_string_marker = string_encoding.end_marker();
        let max_byte_value = string_encoding.max_byte_value();
        let string_impl = string::render_string_impl(string_encoding, has_vocab_constraint);

        let generic_primitives = quote! {

            impl<T: JsonTokenEncoder> JsonTokenEncoder for Option<T> {
                fn encode(&self, tokens: &mut Vec<u32>) {
                    match self {
                        Some(value) => { tokens.push(1); value.encode(tokens); }
                        None => { tokens.push(0); }
                    }
                }
                fn decode(tokens: &mut impl Iterator<Item = u32>) -> Result<Self, DecodeError> {
                    match tokens.next() {
                        Some(presence_flag) if presence_flag % 2 == 1 => Ok(Some(T::decode(tokens)?)),
                        Some(_) => Ok(None),
                        None => Err(DecodeError::UnexpectedEndOfStream {
                            context: EOFContext::new("Option<T>", None, "presence flag")
                        }),
                    }
                }
                fn decode_strict(tokens: &mut impl Iterator<Item = u32>) -> Result<Self, DecodeError> {
                    match tokens.next() {
                        Some(1) => Ok(Some(T::decode_strict(tokens)?)),
                        Some(0) => Ok(None),
                        Some(token) => Err(DecodeError::InvalidTokenInContext {
                            token,
                            context: EOFContext::new("Option<T>", None, "presence flag 0 or 1"),
                        }),
                        None => Err(DecodeError::UnexpectedEndOfStream {
                            context: EOFContext::new("Option<T>", None, "presence flag")
                        }),
                    }
                }
                fn token_count(&self) -> usize {
                    match self {
                        Some(value) => 1 + value.token_count(),
                        None => 1,
                    }
                }
            }

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

            impl JsonTokenEncoder for () {
                fn encode(&self, tokens: &mut Vec<u32>) {
                    tokens.push(0);
                }
                fn decode(tokens: &mut impl Iterator<Item = u32>) -> Result<Self, DecodeError> {
                    match tokens.next() {
                        Some(0) => Ok(()),
                        Some(token) => Err(DecodeError::InvalidData(format!("Expected null token (0), got {}", token))),
                        None => Err(DecodeError::UnexpectedEndOfStream {
                            context: EOFContext::new("()", None, "null token")
                        }),
                    }
                }
                fn token_count(&self) -> usize { 1 }
            }
        };

        let collections_and_numbers = collections::render_collections_and_numbers(
            has_vocab_constraint,
            max_collection_length,
            self.decimal_number_encoding,
            end_string_marker,
        );

        let optional_field_type = optional_field::render_optional_field_type();

        let json_types = json::render_json_types();

        // Add MAX_BASE_VOCAB constant for string encoding
        // This is used by encode_string/decode_string to determine END_STRING_MARKER position
        let base_vocab = self.base_vocab;
        let max_base_vocab_constant = quote! {
            /// Maximum base vocabulary size (actual or constrained value)
            /// Used for String encoding with END_STRING_MARKER at (MAX_BASE_VOCAB - 1)
            /// Encoding base = MAX_BASE_VOCAB - 1 (reserves highest token for marker)
            pub const MAX_BASE_VOCAB: u32 = #base_vocab;
        };

        // Streaming helper functions for decode_prefix implementations
        // These are standalone versions embedded in the generated module
        let streaming_helpers =
            streaming::render_streaming_helpers(end_string_marker, max_byte_value, encoding_base);

        // Per-token role export (opt-in at call time, zero effect on
        // streams): value bytes are consumed at exactly two generated
        // sites (String::decode and the decimal number codec), which mark
        // a thread-local sink while `token_roles` replays a decode through
        // a counting iterator. Everything unmarked is structure.
        let role_export = quote! {
            pub mod role_export {
                use std::cell::Cell;

                pub const ROLE_STRUCTURE: u8 = 0;
                pub const ROLE_VALUE: u8 = 1;
                pub const ROLE_VALUE_END: u8 = 2;
                pub const ROLE_DISCRIMINATOR: u8 = 3;

                thread_local! {
                    static ACTIVE: Cell<bool> = const { Cell::new(false) };
                    static POSITION: Cell<usize> = const { Cell::new(0) };
                }
                thread_local! {
                    static MARKS: std::cell::RefCell<Vec<(usize, u8)>> =
                        const { std::cell::RefCell::new(Vec::new()) };
                }

                pub(super) struct CountingIter<I> {
                    inner: I,
                }

                impl<I: Iterator<Item = u32>> Iterator for CountingIter<I> {
                    type Item = u32;
                    fn next(&mut self) -> Option<u32> {
                        let item = self.inner.next();
                        if item.is_some() {
                            POSITION.with(|p| p.set(p.get() + 1));
                        }
                        item
                    }
                }

                #[inline]
                pub(super) fn mark(role: u8) {
                    if ACTIVE.with(|a| a.get()) {
                        let pos = POSITION.with(|p| p.get());
                        if pos > 0 {
                            MARKS.with(|m| m.borrow_mut().push((pos - 1, role)));
                        }
                    }
                }

                #[inline]
                pub(super) fn mark_value() {
                    mark(ROLE_VALUE);
                }

                #[inline]
                pub(super) fn mark_value_end() {
                    mark(ROLE_VALUE_END);
                }

                /// Tag the just-consumed oneOf/enum variant-selector (the
                /// discriminator) token. Called from generated enum decode.
                #[inline]
                pub fn mark_discriminator() {
                    mark(ROLE_DISCRIMINATOR);
                }

                /// Roles for every token of a raw (pre-BPE) stream, derived
                /// by replaying decode. Returns one role per input token.
                pub fn token_roles<T: super::JsonTokenEncoder>(
                    tokens: &[u32],
                ) -> Result<Vec<u8>, super::DecodeError> {
                    ACTIVE.with(|a| a.set(true));
                    POSITION.with(|p| p.set(0));
                    MARKS.with(|m| m.borrow_mut().clear());
                    let mut iter = CountingIter { inner: tokens.iter().copied() };
                    let outcome = T::decode(&mut iter);
                    ACTIVE.with(|a| a.set(false));
                    outcome?;
                    let mut roles = vec![ROLE_STRUCTURE; tokens.len()];
                    MARKS.with(|m| {
                        for (pos, role) in m.borrow().iter() {
                            if *pos < roles.len() {
                                roles[*pos] = *role;
                            }
                        }
                    });
                    Ok(roles)
                }

                /// Strictly replay a complete or valid-incomplete raw prefix.
                ///
                /// Returns one compiler role per consumed input token and a
                /// completion flag. Recovery normalization is disabled. A
                /// strict EOF after consuming the entire input is the only
                /// accepted incomplete outcome; every other decode error and
                /// every surplus token is rejected.
                pub fn strict_prefix_roles<T: super::JsonTokenEncoder>(
                    tokens: &[u32],
                ) -> Result<(Vec<u8>, bool), super::DecodeError> {
                    ACTIVE.with(|a| a.set(true));
                    POSITION.with(|p| p.set(0));
                    MARKS.with(|m| m.borrow_mut().clear());
                    let mut iter = CountingIter { inner: tokens.iter().copied() };
                    let outcome = T::decode_strict(&mut iter);
                    let consumed = POSITION.with(|p| p.get());
                    ACTIVE.with(|a| a.set(false));

                    let complete = match outcome {
                        Ok(_) if consumed == tokens.len() => true,
                        Ok(_) => {
                            return Err(super::DecodeError::InvalidData(format!(
                                "strict prefix replay left {} surplus token{}",
                                tokens.len().saturating_sub(consumed),
                                if tokens.len().saturating_sub(consumed) == 1 { "" } else { "s" },
                            )));
                        }
                        Err(super::DecodeError::UnexpectedEndOfStream { .. })
                            if consumed == tokens.len() => false,
                        Err(super::DecodeError::UnexpectedEndOfStream { .. }) => {
                            return Err(super::DecodeError::InvalidData(format!(
                                "strict prefix replay reported EOF with {} unconsumed token{}",
                                tokens.len().saturating_sub(consumed),
                                if tokens.len().saturating_sub(consumed) == 1 { "" } else { "s" },
                            )));
                        }
                        Err(error) => return Err(error),
                    };

                    let mut roles = vec![ROLE_STRUCTURE; tokens.len()];
                    MARKS.with(|m| {
                        for (pos, role) in m.borrow().iter() {
                            if *pos < roles.len() {
                                roles[*pos] = *role;
                            }
                        }
                    });
                    Ok((roles, complete))
                }
            }
        };

        // Cursor-derived terminal object-property key/path for host-value
        // positions. Active only during `current_terminal_key_path` replay;
        // zero effect on ordinary encode/decode/role paths.
        let terminal_key_export = quote! {
            /// Decoder-cursor terminal object-property introspection.
            ///
            /// Reports the active schema object-property key (and dotted path)
            /// when a raw prefix is positioned at a host-value leaf (string or
            /// decimal number codec): either immediately before the first value
            /// byte or mid-value before VALUE_END. Returns `None` when the
            /// prefix is complete, dead, or waiting on structure (presence,
            /// discriminator, length, etc.). Never inspects gold JSON.
            pub mod terminal_key_export {
                use std::cell::{Cell, RefCell};

                #[derive(Clone, Debug)]
                enum PathSeg {
                    Prop(&'static str),
                    Index(usize),
                }

                thread_local! {
                    static ACTIVE: Cell<bool> = const { Cell::new(false) };
                    static HOST_OPEN: Cell<bool> = const { Cell::new(false) };
                    static PATH: RefCell<Vec<PathSeg>> = const { RefCell::new(Vec::new()) };
                    static CAPTURED: RefCell<Option<(String, String)>> =
                        const { RefCell::new(None) };
                }

                /// Clears the replay session TLS even if decode panics.
                struct SessionGuard;

                impl Drop for SessionGuard {
                    fn drop(&mut self) {
                        ACTIVE.with(|a| a.set(false));
                        HOST_OPEN.with(|h| h.set(false));
                        PATH.with(|path| path.borrow_mut().clear());
                        CAPTURED.with(|c| *c.borrow_mut() = None);
                    }
                }

                /// Pops one path segment when the active session still owns it.
                struct PathPopGuard;

                impl Drop for PathPopGuard {
                    fn drop(&mut self) {
                        if is_active() {
                            PATH.with(|path| {
                                let _ = path.borrow_mut().pop();
                            });
                        }
                    }
                }

                #[inline]
                fn is_active() -> bool {
                    ACTIVE.with(|a| a.get())
                }

                fn path_snapshot() -> Option<(String, String)> {
                    PATH.with(|path| {
                        let segs = path.borrow();
                        if segs.is_empty() {
                            return None;
                        }
                        let key = segs.iter().rev().find_map(|seg| match seg {
                            PathSeg::Prop(name) => Some((*name).to_string()),
                            PathSeg::Index(_) => None,
                        })?;
                        let path_str = segs
                            .iter()
                            .map(|seg| match seg {
                                PathSeg::Prop(name) => (*name).to_string(),
                                PathSeg::Index(index) => index.to_string(),
                            })
                            .collect::<Vec<_>>()
                            .join(".");
                        Some((key, path_str))
                    })
                }

                #[inline]
                pub(super) fn begin_host_value() {
                    if is_active() {
                        HOST_OPEN.with(|h| h.set(true));
                    }
                }

                #[inline]
                pub(super) fn end_host_value() {
                    if is_active() {
                        HOST_OPEN.with(|h| h.set(false));
                    }
                }

                /// Snapshot path when EOF occurs inside an open host-value leaf.
                #[inline]
                pub(super) fn capture_host_value_eof() {
                    if is_active() && HOST_OPEN.with(|h| h.get()) {
                        if let Some(snapshot) = path_snapshot() {
                            CAPTURED.with(|c| *c.borrow_mut() = Some(snapshot));
                        }
                    }
                }

                /// Push a fixed schema object-property key for the duration of `f`.
                #[inline]
                pub fn with_object_property<T, F: FnOnce() -> T>(
                    property: &'static str,
                    f: F,
                ) -> T {
                    let _guard = if is_active() {
                        PATH.with(|path| path.borrow_mut().push(PathSeg::Prop(property)));
                        Some(PathPopGuard)
                    } else {
                        None
                    };
                    f()
                }

                /// Push an array index for the duration of `f`.
                #[inline]
                pub fn with_array_index<T, F: FnOnce() -> T>(index: usize, f: F) -> T {
                    let _guard = if is_active() {
                        PATH.with(|path| path.borrow_mut().push(PathSeg::Index(index)));
                        Some(PathPopGuard)
                    } else {
                        None
                    };
                    f()
                }

                /// Replay a raw prefix and return `(terminal_key, dotted_path)`
                /// when positioned at a host-value object-property leaf.
                ///
                /// Fail-closed: dead prefixes, complete streams, and structural
                /// wait positions return `None` without guessing. Session TLS is
                /// always cleared on exit (including panic unwind).
                pub fn current_terminal_key_path<T: super::JsonTokenEncoder>(
                    tokens: &[u32],
                ) -> Option<(String, String)> {
                    ACTIVE.with(|a| a.set(true));
                    HOST_OPEN.with(|h| h.set(false));
                    PATH.with(|path| path.borrow_mut().clear());
                    CAPTURED.with(|c| *c.borrow_mut() = None);
                    let _session = SessionGuard;

                    let mut iter = tokens.iter().copied();
                    let outcome = T::decode_strict(&mut iter);
                    let captured = CAPTURED.with(|c| c.borrow_mut().take());

                    match outcome {
                        Err(super::DecodeError::UnexpectedEndOfStream { .. }) => captured,
                        _ => None,
                    }
                }
            }
        };

        quote! {
            pub mod tokenizer {
                #error_enum
                #field_context_helper
                #trait_def
                #buffered_iterator
                #variable_base_helpers
                #max_base_vocab_constant
                #primitives
                #string_impl
                #generic_primitives
                #collections_and_numbers
                #optional_field_type
                #json_types
                #streaming_helpers
                #role_export
                #terminal_key_export
            }
        }
    }
}
