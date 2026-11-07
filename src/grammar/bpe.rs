//! Emits the generated BPE compression module.
//!
//! The generated module embeds a trained merge table and implements the
//! runtime encode/decode helpers used by schema-specific tokenizers. This file
//! is code-generation logic only: it translates a validated merge table into
//! constants, lookup structures, and bounded expansion budgets that preserve the
//! base-token and BPE-token vocabulary ranges.

use proc_macro2::TokenStream;
use quote::quote;

use super::Grammar;

mod convenience;
mod encoding;

fn bpe_decode_steps_for_generated_component(
    token: u32,
    bpe_start: u32,
    budgets: &[usize],
) -> usize {
    if token >= bpe_start {
        let idx = (token - bpe_start) as usize;
        budgets.get(idx).copied().unwrap_or(1)
    } else {
        1
    }
}

impl Grammar {
    /// Generate BPE compression module
    pub(super) fn generate_bpe_module(
        &self,
        merge_table: &crate::bpe::BpeMergeTable,
    ) -> Result<TokenStream, crate::error::TctError> {
        let base_vocab_size = merge_table.base_vocab_size;
        let schema_name = &merge_table.schema;

        // Extract merge data for embedding
        let merge_pairs: Vec<_> = merge_table
            .merges
            .iter()
            .map(|m| {
                let left = m.pair[0];
                let right = m.pair[1];
                quote! { (#left, #right) }
            })
            .collect();

        let merge_tokens: Vec<_> = merge_table
            .merges
            .iter()
            .map(|m| {
                let token = m.new_token;
                quote! { #token }
            })
            .collect();

        let mut pair_index_entries: Vec<_> = merge_table
            .merges
            .iter()
            .enumerate()
            .map(|(merge_idx, merge)| {
                let key = ((merge.pair[0] as u64) << u32::BITS) | merge.pair[1] as u64;
                (key, merge_idx)
            })
            .collect();
        pair_index_entries.sort_by_key(|&(key, _)| key);
        let pair_index_entries: Vec<_> = pair_index_entries
            .into_iter()
            .map(|(key, merge_idx)| quote! { (#key, #merge_idx) })
            .collect();

        // Compute BPE start token after the base range.
        let bpe_start = merge_table.bpe_start();

        let mut decode_step_budget_values = Vec::with_capacity(merge_table.merges.len());
        for merge in &merge_table.merges {
            let left_steps = bpe_decode_steps_for_generated_component(
                merge.pair[0],
                bpe_start,
                &decode_step_budget_values,
            );
            let right_steps = bpe_decode_steps_for_generated_component(
                merge.pair[1],
                bpe_start,
                &decode_step_budget_values,
            );
            decode_step_budget_values.push(
                1usize
                    .saturating_add(left_steps)
                    .saturating_add(right_steps),
            );
        }
        let decode_step_budgets: Vec<_> = decode_step_budget_values
            .iter()
            .map(|budget| quote! { #budget })
            .collect();
        let encoding_helpers = encoding::render_encoding_helpers();

        // Use usize for array size literals (not u32)
        let max_merges = merge_table.merges.len();

        Ok(quote! {
            /// BPE token compression module
            ///
            /// This module provides Byte Pair Encoding (BPE) compression for base tokens.
            /// BPE is trained on instance data and compresses common token sequences.
            #[allow(dead_code)]
            pub mod bpe {
                use rustc_hash::FxHashMap;

                /// BPE merge table embedded at compile time
                const BASE_VOCAB_SIZE: u32 = #base_vocab_size;
                /// Token ID where BPE merges begin (after the base range)
                const BPE_START: u32 = #bpe_start;
                const SCHEMA_NAME: &str = #schema_name;
                const MAX_MERGES: usize = #max_merges;
                /// Sentinel used in linked-position arrays for "no previous" or "no next".
                ///
                /// Generated BPE encoding stores live positions as `u32` indices. Inputs
                /// too large for `u32` indices use the full-scan fallback, so this value
                /// cannot collide with a valid linked-list position.
                const NO_LINK: u32 = u32::MAX;
                /// Mask for the low 32 bits of packed initial-pair candidates.
                ///
                /// Initial candidates are packed as `(merge_idx << 32) | left_position`.
                /// Sorting the packed values groups candidates by merge priority first
                /// while preserving the linked-list left position in the low bits.
                const COMPACT_CANDIDATE_LEFT_MASK: u64 = u32::MAX as u64;

                /// Mutable linked-token index used while applying priority-ordered merges.
                ///
                /// `next` and `prev` use `NO_LINK` as their sentinel. `alive` lets the
                /// encoder reject stale positions after adjacent merges rewrite the linked
                /// list, and `future_positions` holds new boundary candidates keyed by the
                /// later merge that can consume them.
                struct PairIndexState<'a> {
                    current: &'a mut [u32],
                    next: &'a mut [u32],
                    prev: &'a mut [u32],
                    alive: &'a mut [bool],
                    future_positions: &'a mut FxHashMap<u32, PositionBucket>,
                    position_pool: &'a mut Vec<Vec<u32>>,
                }

                /// Pending linked-list positions for a future merge.
                ///
                /// Buckets stay in the allocation-free `One` state for the common case.
                /// They upgrade to `Many` only when multiple boundary candidates target
                /// the same later merge; the vector is sorted, deduplicated, and recycled
                /// through `position_pool` before merge application.
                enum PositionBucket {
                    One(u32),
                    Many(Vec<u32>),
                }

                /// Merge pairs: [(left_token, right_token), ...]
                const MERGE_PAIRS: [(u32, u32); #max_merges] = [
                    #(#merge_pairs),*
                ];

                /// New tokens corresponding to each merge
                const MERGE_TOKENS: [u32; #max_merges] = [
                    #(#merge_tokens),*
                ];

                /// Sorted pair-key to merge-index lookup table.
                ///
                /// Pair keys are `(left << 32) | right`. The table is sorted by
                /// pair key so generated tokenizers can look up merge indices
                /// without building a runtime hash map.
                const MERGE_PAIR_INDEX: [(u64, usize); #max_merges] = [
                    #(#pair_index_entries),*
                ];

                /// Exact iterative decode-step budget for each BPE merge token.
                ///
                /// Index `i` corresponds to token `BPE_START + i`. The values are
                /// generated from the merge table, avoiding a first-decode cache
                /// allocation in generated tokenizers.
                const DECODE_STEP_BUDGETS: [usize; #max_merges] = [
                    #(#decode_step_budgets),*
                ];

                #encoding_helpers

                /// Decode BPE tokens to base tokens
                ///
                /// Uses direct array indexing for O(1) token expansion.
                /// BPE tokens are sequential (BPE_START + i), so token ID directly
                /// maps to MERGE_PAIRS index without needing a HashMap. Expansion
                /// uses an explicit stack rather than recursive calls so deep merge
                /// chains do not consume the Rust call stack.
                ///
                /// # Arguments
                /// * `tokens` - BPE token sequence
                /// * `max_vocab_size` - Maximum vocabulary size (controls which merges to apply)
                ///
                /// # Returns
                /// Base token sequence
                pub fn decode(tokens: &[u32], max_vocab_size: u32) -> Vec<u32> {
                    if tokens.is_empty() || max_vocab_size <= BPE_START {
                        return tokens.to_vec();
                    }

                    if max_vocab_size == 0 || BPE_START == 0 {
                        return tokens.to_vec();
                    }

                    // Calculate the maximum valid BPE token for this vocabulary size
                    let max_merge_token = max_vocab_size.min(BPE_START + MAX_MERGES as u32);
                    // Pre-allocate with estimated 2x expansion ratio (typical for BPE)
                    let mut result = Vec::with_capacity(tokens.len().saturating_mul(2));
                    let mut remaining_budget = decode_expansion_budget(
                        tokens,
                        max_vocab_size,
                        max_merge_token,
                        &DECODE_STEP_BUDGETS,
                    );

                    let mut expansion_stack = Vec::new();

                    for &token in tokens {
                        expansion_stack.push(token);
                        while let Some(token) = expansion_stack.pop() {
                            if remaining_budget == 0 {
                                result.push(recover_token_to_base(token, max_vocab_size));
                                continue;
                            }
                            remaining_budget = remaining_budget.saturating_sub(1);
                            expand_token(
                                token,
                                max_vocab_size,
                                max_merge_token,
                                &mut result,
                                &mut expansion_stack,
                            );
                        }
                    }

                    result
                }

                /// Decode BPE tokens to base tokens while rejecting malformed token values.
                ///
                /// Unlike `decode`, this strict path does not modulo-normalize
                /// out-of-range tokens or unknown BPE-range tokens.
                pub fn decode_strict(
                    tokens: &[u32],
                    max_vocab_size: u32,
                ) -> Result<Vec<u32>, crate::tokenizer::DecodeError> {
                    if tokens.is_empty() {
                        return Ok(Vec::new());
                    }

                    if max_vocab_size == 0 || BPE_START == 0 {
                        return Err(invalid_bpe_token(
                            tokens[0],
                            "token below BPE vocabulary size",
                        ));
                    }

                    let max_merge_token = max_vocab_size.min(BPE_START + MAX_MERGES as u32);
                    let mut result = Vec::with_capacity(tokens.len().saturating_mul(2));
                    let mut remaining_budget = decode_expansion_budget(
                        tokens,
                        max_vocab_size,
                        max_merge_token,
                        &DECODE_STEP_BUDGETS,
                    );

                    let mut expansion_stack = Vec::new();

                    for &token in tokens {
                        expansion_stack.push(token);
                        while let Some(token) = expansion_stack.pop() {
                            if remaining_budget == 0 {
                                return Err(crate::tokenizer::DecodeError::InvalidData(
                                    "BPE decode expansion budget exhausted".to_string(),
                                ));
                            }
                            remaining_budget = remaining_budget.saturating_sub(1);
                            expand_token_strict(
                                token,
                                max_vocab_size,
                                max_merge_token,
                                &mut result,
                                &mut expansion_stack,
                            )?;
                        }
                    }

                    Ok(result)
                }

                /// Expand one token step using direct array indexing
                ///
                /// This is O(1) lookup without a HashMap.
                /// BPE tokens are sequential starting at BPE_START, so direct indexing works.
                #[inline]
                fn expand_token(
                    token: u32,
                    max_vocab_size: u32,
                    max_merge_token: u32,
                    result: &mut Vec<u32>,
                    expansion_stack: &mut Vec<u32>,
                ) {
                    let normalized = token % max_vocab_size;
                    if normalized < BPE_START {
                        // Base token - pass through directly
                        result.push(normalized);
                    } else if normalized < max_merge_token {
                        // Valid BPE token - expand via direct array index
                        // MERGE_TOKENS[i] = BPE_START + i, so inverse is: idx = token - BPE_START
                        let idx = (normalized - BPE_START) as usize;
                        let (left, right) = MERGE_PAIRS[idx];
                        expansion_stack.push(right);
                        expansion_stack.push(left);
                    } else {
                        // Token outside vocabulary - normalize to base range for graceful recovery
                        result.push(recover_normalized_token_to_base(normalized));
                    }
                }

                #[inline]
                fn expand_token_strict(
                    token: u32,
                    max_vocab_size: u32,
                    max_merge_token: u32,
                    result: &mut Vec<u32>,
                    expansion_stack: &mut Vec<u32>,
                ) -> Result<(), crate::tokenizer::DecodeError> {
                    if token >= max_vocab_size {
                        return Err(invalid_bpe_token(
                            token,
                            "token below BPE vocabulary size",
                        ));
                    }

                    if token < BPE_START {
                        result.push(token);
                        Ok(())
                    } else if token < max_merge_token {
                        let idx = (token - BPE_START) as usize;
                        let (left, right) = MERGE_PAIRS[idx];
                        expansion_stack.push(right);
                        expansion_stack.push(left);
                        Ok(())
                    } else {
                        Err(invalid_bpe_token(
                            token,
                            "base token or known BPE merge token",
                        ))
                    }
                }

                #[inline]
                fn recover_token_to_base(token: u32, max_vocab_size: u32) -> u32 {
                    recover_normalized_token_to_base(token % max_vocab_size)
                }

                #[inline]
                fn recover_normalized_token_to_base(normalized: u32) -> u32 {
                    if normalized < BPE_START {
                        normalized
                    } else {
                        normalized % BPE_START
                    }
                }

                fn decode_expansion_budget(
                    tokens: &[u32],
                    max_vocab_size: u32,
                    max_merge_token: u32,
                    budgets: &[usize],
                ) -> usize {
                    tokens.iter().fold(0usize, |budget, &token| {
                        budget.saturating_add(decode_steps_for_component(
                            token,
                            max_vocab_size,
                            max_merge_token,
                            budgets,
                        ))
                    })
                }

                fn decode_steps_for_component(
                    token: u32,
                    max_vocab_size: u32,
                    max_merge_token: u32,
                    budgets: &[usize],
                ) -> usize {
                    let normalized = token % max_vocab_size;
                    if normalized >= BPE_START && normalized < max_merge_token {
                        let idx = (normalized - BPE_START) as usize;
                        budgets.get(idx).copied().unwrap_or(1)
                    } else {
                        1
                    }
                }

                fn invalid_bpe_token(
                    token: u32,
                    expected: &'static str,
                ) -> crate::tokenizer::DecodeError {
                    crate::tokenizer::DecodeError::InvalidTokenInContext {
                        token,
                        context: crate::tokenizer::EOFContext::new(
                            "BpeDecoder",
                            Some("token"),
                            expected,
                        ),
                    }
                }

            }

        })
    }
}
