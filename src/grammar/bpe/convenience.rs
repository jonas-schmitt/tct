//! Emits generated codec convenience wrappers for BPE-enabled tokenizers.

use proc_macro2::TokenStream;
use quote::quote;

use super::super::Grammar;

impl Grammar {
    /// Generate convenience wrapper functions for encoding/decoding with BPE.
    pub(in crate::grammar) fn generate_bpe_convenience_wrappers(&self) -> TokenStream {
        if self.bpe_merge_table.is_none() {
            return quote! {};
        }

        quote! {
            /// Codec convenience functions for encoding and decoding with BPE compression
            ///
            /// Put in a module to avoid namespace collision with PyO3 bindings in Python wheels.
            pub mod codec {
                /// Encode with BPE compression
                ///
                /// This is the default encoding method that applies all compiled optimizations:
                /// - BPE compression for token sequences
                ///
                /// # Arguments
                /// * `value` - The value to encode
                ///
                /// # Returns
                /// BPE-compressed token sequence
                ///
                /// # Example
                /// ```ignore
                /// use tct::tokenizer::JsonTokenEncoder;
                ///
                /// let workflow: GithubWorkflow = ...;
                /// let tokens = codec::encode(&workflow);  // Uses BPE automatically
                /// ```
                pub fn encode<T: crate::tokenizer::JsonTokenEncoder>(value: &T) -> Vec<u32> {
                    // Step 1: Base TCT encoding
                    let mut base_tokens = Vec::new();
                    value.encode(&mut base_tokens);

                    // Step 2: Apply BPE compression with correct vocab size
                    super::bpe::encode_owned(base_tokens, super::VOCAB_SIZE)
                }

                /// Decode with BPE decompression
                ///
                /// Returns (value, tokens_consumed, total_base_tokens) for surplus detection.
                /// Overhead: ~1 CPU cycle per token for position tracking (negligible).
                ///
                /// # Returns
                /// - value: Decoded instance
                /// - tokens_consumed: Base tokens actually decoded
                /// - total_base_tokens: Total base tokens after BPE expansion
                ///
                /// # Example
                /// ```ignore
                /// let tokens = codec::encode(&workflow);
                /// let (workflow, consumed, total) = codec::decode(tokens)?;
                /// if consumed < total {
                ///     eprintln!("Warning: {} surplus tokens not consumed", total - consumed);
                /// }
                /// ```
                pub fn decode<T: crate::tokenizer::JsonTokenEncoder>(
                    tokens: Vec<u32>
                ) -> Result<(T, usize, usize), crate::tokenizer::DecodeError> {
                    // Step 1: Expand BPE to base tokens with correct vocab size
                    let base_tokens = super::bpe::decode(&tokens, super::VOCAB_SIZE);
                    let base_total = base_tokens.len();

                    // Step 2: Decode with consumption tracking
                    let mut iter = crate::tokenizer::BufferedTokenIterator::from_vec(base_tokens);
                    let value = T::decode(&mut iter)?;
                    let consumed = base_total - iter.remaining();

                    Ok((value, consumed, base_total))
                }

                /// Decode with strict BPE decompression and strict base-token decoding.
                ///
                /// This rejects malformed BPE token values before decoding the
                /// expanded base token stream.
                pub fn decode_strict<T: crate::tokenizer::JsonTokenEncoder>(
                    tokens: Vec<u32>
                ) -> Result<(T, usize, usize), crate::tokenizer::DecodeError> {
                    let base_tokens = super::bpe::decode_strict(&tokens, super::VOCAB_SIZE)?;
                    let base_total = base_tokens.len();

                    let mut iter = crate::tokenizer::BufferedTokenIterator::from_vec(base_tokens);
                    let value = T::decode_strict(&mut iter)?;
                    let consumed = base_total - iter.remaining();

                    Ok((value, consumed, base_total))
                }

                /// Encode without BPE compression
                ///
                /// Useful for comparing base TCT encoding vs TCT+BPE performance.
                ///
                /// # Arguments
                /// * `value` - The value to encode
                ///
                /// # Returns
                /// Base token sequence (no BPE)
                pub fn encode_no_bpe<T: crate::tokenizer::JsonTokenEncoder>(value: &T) -> Vec<u32> {
                    let mut tokens = Vec::new();
                    value.encode(&mut tokens);
                    tokens  // Skip BPE
                }

                /// Decode without BPE - debugging helper
                ///
                /// Returns (value, tokens_consumed, total_tokens) for consistency with decode().
                ///
                /// # Arguments
                /// * `tokens` - Base token sequence (no BPE)
                ///
                /// # Returns
                /// Tuple of (value, tokens_consumed, total_tokens)
                pub fn decode_no_bpe<T: crate::tokenizer::JsonTokenEncoder>(
                    tokens: Vec<u32>
                ) -> Result<(T, usize, usize), crate::tokenizer::DecodeError> {
                    let total = tokens.len();
                    let mut iter = crate::tokenizer::BufferedTokenIterator::from_vec(tokens);
                    let value = T::decode(&mut iter)?;
                    let consumed = total - iter.remaining();
                    Ok((value, consumed, total))
                }
            }
        }
    }
}
