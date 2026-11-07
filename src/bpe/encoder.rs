//! BPE encoding and decoding algorithms
//!
//! Implements runtime token compression by applying learned merge operations
//! to base token sequences.

use super::BpeMergeTable;
use crate::tokenizer::{DecodeError, EOFContext};
use rustc_hash::FxHashMap;

/// Legacy best-first BPE encoder retained for historical test evidence.
///
/// This encoder repeatedly applies the single best merge (lowest merged token
/// ID) found anywhere in the sequence. That selection strategy diverges from
/// the trainer's sequential merge-order simulation, so its output can differ
/// from the merge tables' training-time segmentation. Production paths
/// (wheels, data preparation, measurement binaries) use
/// [`SequentialBpeEncoder`], which matches the trainer exactly; prefer it for
/// any new code.
pub struct BpeEncoder {
    /// Map from token pair to merged token
    pair_to_token: FxHashMap<(u32, u32), u32>,
}

impl BpeEncoder {
    /// Create new BPE encoder from merge table
    ///
    /// # Arguments
    /// * `table` - BPE merge table
    /// * `max_vocab_size` - Maximum vocabulary size (controls how many merges to apply)
    ///
    /// # Example
    /// ```no_run
    /// use tct::bpe::{BpeMergeTable, encoder::BpeEncoder};
    ///
    /// let table = BpeMergeTable::from_file("merges.json")?;
    /// let encoder = BpeEncoder::new(&table, 336); // 80 base + 256 BPE
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn new(table: &BpeMergeTable, max_vocab_size: u32) -> Self {
        let (pair_to_token, _) = table.build_lookup_maps(max_vocab_size);
        Self { pair_to_token }
    }

    /// Encode base tokens to BPE tokens (legacy best-first selection)
    ///
    /// Applies BPE merges iteratively using best-first selection: always
    /// select the merge with the lowest merged token ID anywhere in the
    /// sequence, apply that one occurrence, then repeat until no more merges
    /// are possible.
    ///
    /// This is a legacy, divergent algorithm: it was never validated against
    /// nanochat or tiktoken, and it does not reproduce the trainer's
    /// sequential merge-order simulation. Use [`SequentialBpeEncoder::encode`]
    /// for trainer-equivalent production encoding.
    ///
    /// # Arguments
    /// * `tokens` - Base token sequence
    ///
    /// # Returns
    /// BPE-compressed token sequence
    ///
    /// # Example
    /// ```no_run
    /// # use tct::bpe::{BpeMergeTable, encoder::BpeEncoder};
    /// # let table = BpeMergeTable::from_file("merges.json")?;
    /// # let encoder = BpeEncoder::new(&table, 336);
    /// let base_tokens = vec![5, 12, 7, 5, 12];
    /// let bpe_tokens = encoder.encode(&base_tokens);
    /// // If merges trained: [5,12]→80, [80,7]→81
    /// // Result: [81, 80] (compressed from 5 to 2 tokens)
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn encode(&self, tokens: &[u32]) -> Vec<u32> {
        if tokens.is_empty() {
            return Vec::new();
        }

        let mut current = tokens.to_vec();

        // Apply merges iteratively using best-first selection
        loop {
            let (merged, applied) = self.apply_best_merge(&current);
            if !applied {
                // No merges possible, we're done
                break;
            }
            current = merged;
        }

        current
    }

    /// Apply the highest-priority merge in the sequence
    ///
    /// Scans all adjacent token pairs, finds the one with the lowest merged token ID
    /// (which corresponds to highest training frequency), and applies only that merge.
    ///
    /// This legacy best-first selection is retained for historical evidence
    /// only; the production path is [`SequentialBpeEncoder`].
    ///
    /// # Returns
    /// Tuple of (new_tokens, merge_was_applied)
    fn apply_best_merge(&self, tokens: &[u32]) -> (Vec<u32>, bool) {
        if tokens.len() < 2 {
            return (tokens.to_vec(), false);
        }

        // Scan all pairs to find the best merge (lowest token ID = highest priority)
        let mut best_merge: Option<(usize, u32)> = None; // (position, new_token)

        for i in 0..(tokens.len() - 1) {
            let pair = (tokens[i], tokens[i + 1]);
            if let Some(&merged_token) = self.pair_to_token.get(&pair) {
                // Update if this is the first merge found, or if it has higher priority
                // (lower token ID = trained earlier = higher frequency)
                let should_replace = match best_merge {
                    Some((_, best_token)) => merged_token < best_token,
                    None => true,
                };
                if should_replace {
                    best_merge = Some((i, merged_token));
                }
            }
        }

        // Apply the best merge if found
        if let Some((pos, new_token)) = best_merge {
            let mut result = Vec::with_capacity(tokens.len() - 1);
            result.extend_from_slice(&tokens[..pos]);
            result.push(new_token);
            result.extend_from_slice(&tokens[pos + 2..]);
            (result, true)
        } else {
            // No merges possible
            (tokens.to_vec(), false)
        }
    }
}

/// Indexed BPE encoder that matches the training simulation exactly.
///
/// This encoder applies merges in priority order while keeping an index of
/// current adjacent-pair positions. Stale positions are verified before use, so
/// merge-order semantics stay identical to the trainer's sequential simulation.
const NO_LINK: u32 = u32::MAX;
const COMPACT_CANDIDATE_LEFT_MASK: u64 = u32::MAX as u64;

struct PairIndexState<'a> {
    current: &'a mut [u32],
    next: &'a mut [u32],
    prev: &'a mut [u32],
    alive: &'a mut [bool],
    pair_to_index: &'a FxHashMap<(u32, u32), usize>,
    future_positions: &'a mut FxHashMap<u32, PositionBucket>,
    position_pool: &'a mut Vec<Vec<u32>>,
}

enum PositionBucket {
    One(u32),
    Many(Vec<u32>),
}

pub struct SequentialBpeEncoder {
    /// Ordered list of (pair, new_token) - merge 0 has highest priority
    merges: Vec<((u32, u32), u32)>,
    /// Map from merge pair to merge priority index for position indexing
    pair_to_index: FxHashMap<(u32, u32), usize>,
}

impl SequentialBpeEncoder {
    /// Create new sequential BPE encoder from merge table
    pub fn new(table: &BpeMergeTable, max_vocab_size: u32) -> Self {
        let bpe_start = table.bpe_start();
        let max_merges = if max_vocab_size > bpe_start {
            ((max_vocab_size - bpe_start) as usize).min(table.merges.len())
        } else {
            0
        };

        let mut merges = Vec::with_capacity(max_merges);
        let mut pair_to_index = FxHashMap::with_capacity_and_hasher(max_merges, Default::default());

        for (index, merge) in table.merges[..max_merges].iter().enumerate() {
            let pair = (merge.pair[0], merge.pair[1]);
            merges.push((pair, merge.new_token));
            pair_to_index.insert(pair, index);
        }

        Self {
            merges,
            pair_to_index,
        }
    }

    /// Encode base tokens to BPE tokens using indexed sequential merge application
    ///
    /// Applies merges in order (merge 0 first, then merge 1, etc.)
    pub fn encode(&self, tokens: &[u32]) -> Vec<u32> {
        if tokens.is_empty() {
            return Vec::new();
        }
        if self.merges.is_empty() {
            return tokens.to_vec();
        }
        if tokens.len() < 2 {
            return tokens.to_vec();
        }
        if tokens.len() > u32::MAX as usize || self.merges.len() > u32::MAX as usize {
            return self.encode_with_full_merge_scan(tokens);
        }

        let mut current = tokens.to_vec();
        let initial_pair_positions =
            Self::build_initial_pair_positions(&current, &self.pair_to_index);
        if initial_pair_positions.is_empty() {
            return current;
        }

        let len = current.len();
        let mut next: Vec<u32> = (0..len)
            .map(|idx| {
                if idx + 1 < len {
                    (idx + 1) as u32
                } else {
                    NO_LINK
                }
            })
            .collect();
        let mut prev: Vec<u32> = (0..len)
            .map(|idx| if idx == 0 { NO_LINK } else { (idx - 1) as u32 })
            .collect();
        let mut alive = vec![true; len];
        let mut initial_cursor = 0usize;
        let mut future_positions: FxHashMap<u32, PositionBucket> = FxHashMap::default();
        let mut position_pool: Vec<Vec<u32>> = Vec::new();

        {
            let mut index_state = PairIndexState {
                current: &mut current,
                next: &mut next,
                prev: &mut prev,
                alive: &mut alive,
                pair_to_index: &self.pair_to_index,
                future_positions: &mut future_positions,
                position_pool: &mut position_pool,
            };

            // Apply merges in priority order (merge 0 first, then 1, etc.)
            for (merge_idx, &(pair, new_token)) in self.merges.iter().enumerate() {
                if initial_cursor >= initial_pair_positions.len()
                    && index_state.future_positions.is_empty()
                {
                    break;
                }

                let initial_start = initial_cursor;
                while initial_cursor < initial_pair_positions.len()
                    && Self::candidate_merge_idx(initial_pair_positions[initial_cursor])
                        == merge_idx
                {
                    initial_cursor += 1;
                }
                let initial_positions = &initial_pair_positions[initial_start..initial_cursor];

                match index_state.future_positions.remove(&(merge_idx as u32)) {
                    Some(PositionBucket::One(position)) => {
                        if initial_positions.is_empty() {
                            Self::apply_merge_positions(
                                std::iter::once(position),
                                pair,
                                new_token,
                                merge_idx,
                                &mut index_state,
                            );
                        } else {
                            let mut positions = index_state.position_pool.pop().unwrap_or_default();
                            positions.push(position);
                            positions.extend(
                                initial_positions
                                    .iter()
                                    .map(|&candidate| Self::candidate_left_position(candidate)),
                            );
                            positions.sort_unstable();
                            positions.dedup();
                            Self::apply_merge_positions(
                                positions.iter().copied(),
                                pair,
                                new_token,
                                merge_idx,
                                &mut index_state,
                            );
                            positions.clear();
                            index_state.position_pool.push(positions);
                        }
                    }
                    Some(PositionBucket::Many(mut positions)) => {
                        positions.extend(
                            initial_positions
                                .iter()
                                .map(|&candidate| Self::candidate_left_position(candidate)),
                        );
                        positions.sort_unstable();
                        positions.dedup();
                        Self::apply_merge_positions(
                            positions.iter().copied(),
                            pair,
                            new_token,
                            merge_idx,
                            &mut index_state,
                        );
                        positions.clear();
                        index_state.position_pool.push(positions);
                    }
                    None => {
                        Self::apply_merge_positions(
                            initial_positions
                                .iter()
                                .map(|&candidate| Self::candidate_left_position(candidate)),
                            pair,
                            new_token,
                            merge_idx,
                            &mut index_state,
                        );
                    }
                }
            }
        }

        Self::compact_linked_tokens(current, &next)
    }

    /// Build sorted initial merge-position candidates without per-pair buckets.
    #[inline]
    fn build_initial_pair_positions(
        tokens: &[u32],
        pair_to_index: &FxHashMap<(u32, u32), usize>,
    ) -> Vec<u64> {
        let mut positions = Vec::new();

        for left in 0..tokens.len().saturating_sub(1) {
            let pair = (tokens[left], tokens[left + 1]);
            if let Some(&merge_idx) = pair_to_index.get(&pair) {
                positions.push(Self::pack_initial_pair_position(merge_idx, left));
            }
        }

        positions.sort_unstable();
        positions
    }

    #[inline]
    fn pack_initial_pair_position(merge_idx: usize, left: usize) -> u64 {
        ((merge_idx as u64) << u32::BITS) | left as u64
    }

    #[inline]
    fn candidate_merge_idx(candidate: u64) -> usize {
        (candidate >> u32::BITS) as usize
    }

    #[inline]
    fn candidate_left_position(candidate: u64) -> u32 {
        (candidate & COMPACT_CANDIDATE_LEFT_MASK) as u32
    }

    /// Rare fallback for single sequences too large for compact candidate indices.
    fn encode_with_full_merge_scan(&self, tokens: &[u32]) -> Vec<u32> {
        let mut current = tokens.to_vec();
        for &(pair, new_token) in &self.merges {
            Self::apply_full_scan_merge(&mut current, pair, new_token);
        }
        current
    }

    fn apply_full_scan_merge(tokens: &mut Vec<u32>, pair: (u32, u32), new_token: u32) {
        let mut read = 0usize;
        let mut write = 0usize;

        while read < tokens.len() {
            if read + 1 < tokens.len() && tokens[read] == pair.0 && tokens[read + 1] == pair.1 {
                tokens[write] = new_token;
                read += 2;
            } else {
                tokens[write] = tokens[read];
                read += 1;
            }
            write += 1;
        }

        tokens.truncate(write);
    }

    /// Apply one merge to all candidate left positions after stale-position checks.
    #[inline]
    fn apply_merge_positions<I>(
        positions: I,
        pair: (u32, u32),
        new_token: u32,
        merge_idx: usize,
        state: &mut PairIndexState<'_>,
    ) where
        I: IntoIterator<Item = u32>,
    {
        for left_position in positions {
            let left = left_position as usize;
            if !state.alive[left] || state.current[left] != pair.0 {
                continue;
            }
            let right_position = state.next[left];
            if right_position == NO_LINK {
                continue;
            }
            let right = right_position as usize;
            if !state.alive[right] || state.current[right] != pair.1 {
                continue;
            }

            state.current[left] = new_token;
            state.alive[right] = false;

            let after_position = state.next[right];
            state.next[left] = after_position;
            if after_position != NO_LINK {
                state.prev[after_position as usize] = left_position;
            }

            Self::push_current_pair(state.prev[left], merge_idx, state);
            Self::push_current_pair(left_position, merge_idx, state);
        }
    }

    /// Queue the adjacent pair starting at `left` if a future merge can consume it.
    #[inline]
    fn push_current_pair(
        left_position: u32,
        current_merge_idx: usize,
        state: &mut PairIndexState<'_>,
    ) {
        if left_position == NO_LINK {
            return;
        }
        let left = left_position as usize;
        if !state.alive[left] {
            return;
        }
        let right_position = state.next[left];
        if right_position == NO_LINK {
            return;
        }
        let right = right_position as usize;
        if !state.alive[right] {
            return;
        }

        let pair = (state.current[left], state.current[right]);
        let Some(&merge_idx) = state.pair_to_index.get(&pair) else {
            return;
        };
        if merge_idx <= current_merge_idx {
            return;
        }

        let merge_position = merge_idx as u32;
        match state.future_positions.get_mut(&merge_position) {
            Some(bucket) => match bucket {
                PositionBucket::One(existing) => {
                    let existing_position = *existing;
                    if existing_position != left_position {
                        let mut positions = state.position_pool.pop().unwrap_or_default();
                        positions.push(existing_position);
                        positions.push(left_position);
                        *bucket = PositionBucket::Many(positions);
                    }
                }
                PositionBucket::Many(positions) => {
                    positions.push(left_position);
                }
            },
            None => {
                state
                    .future_positions
                    .insert(merge_position, PositionBucket::One(left_position));
            }
        }
    }

    /// Compact live tokens from the linked representation into the owned buffer.
    #[inline]
    fn compact_linked_tokens(mut tokens: Vec<u32>, next: &[u32]) -> Vec<u32> {
        let mut current_position = 0u32;
        let mut write = 0usize;

        while current_position != NO_LINK {
            let current = current_position as usize;
            tokens[write] = tokens[current];
            write += 1;
            current_position = next[current];
        }

        tokens.truncate(write);
        tokens
    }
}

/// BPE decoder that reverses merge operations
pub struct BpeDecoder {
    /// Map from merged token to original pair
    token_to_pair: FxHashMap<u32, (u32, u32)>,
    /// Exact decode-step budget for each known BPE token.
    token_to_decode_steps: FxHashMap<u32, usize>,
    /// First BPE token. Tokens below this are base tokens.
    bpe_start: u32,
    /// Total vocabulary size (base + BPE tokens)
    max_vocab_size: u32,
}

impl BpeDecoder {
    /// Create new BPE decoder from merge table
    ///
    /// # Arguments
    /// * `table` - BPE merge table
    /// * `max_vocab_size` - Maximum vocabulary size (must match encoder)
    ///
    /// # Example
    /// ```no_run
    /// use tct::bpe::{BpeMergeTable, encoder::BpeDecoder};
    ///
    /// let table = BpeMergeTable::from_file("merges.json")?;
    /// let decoder = BpeDecoder::new(&table, 336);
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn new(table: &BpeMergeTable, max_vocab_size: u32) -> Self {
        let (_, token_to_pair) = table.build_lookup_maps(max_vocab_size);
        let token_to_decode_steps =
            Self::build_decode_step_budgets(&token_to_pair, table.bpe_start(), max_vocab_size);
        Self {
            token_to_pair,
            token_to_decode_steps,
            bpe_start: table.bpe_start(),
            max_vocab_size,
        }
    }

    /// Decode BPE tokens to base tokens
    ///
    /// Expands merged tokens back to original token pairs without recursive
    /// calls, so deeply chained merge tables cannot overflow the Rust stack.
    ///
    /// # Arguments
    /// * `tokens` - BPE token sequence
    ///
    /// # Returns
    /// Base token sequence
    ///
    /// # Example
    /// ```no_run
    /// # use tct::bpe::{BpeMergeTable, encoder::BpeDecoder};
    /// # let table = BpeMergeTable::from_file("merges.json")?;
    /// # let decoder = BpeDecoder::new(&table, 336);
    /// let bpe_tokens = vec![81, 80]; // Compressed tokens
    /// let base_tokens = decoder.decode(&bpe_tokens);
    /// // Result: [5, 12, 7, 5, 12] (expanded back to base)
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn decode(&self, tokens: &[u32]) -> Vec<u32> {
        self.decode_with_expansion_budget(tokens, usize::MAX)
    }

    /// Decode BPE tokens while rejecting invalid token values.
    ///
    /// Unlike [`decode`](Self::decode), this strict path does not modulo-normalize
    /// out-of-range tokens or unknown BPE-range tokens. It is intended for
    /// callers that need malformed token streams rejected instead of recovered.
    pub fn decode_strict(&self, tokens: &[u32]) -> Result<Vec<u32>, DecodeError> {
        if tokens.is_empty() {
            return Ok(Vec::new());
        }

        if self.max_vocab_size == 0 || self.bpe_start == 0 {
            return Err(Self::invalid_bpe_token(
                tokens[0],
                "token below BPE vocabulary size",
            ));
        }

        let mut result = Vec::new();
        let mut stack = Vec::new();
        let mut remaining_budget = self.decode_expansion_budget(tokens);

        for &token in tokens {
            stack.push(token);
            while let Some(token) = stack.pop() {
                if remaining_budget == 0 {
                    return Err(DecodeError::InvalidData(
                        "BPE decode expansion budget exhausted".to_string(),
                    ));
                }
                remaining_budget = remaining_budget.saturating_sub(1);
                self.expand_token_strict(token, &mut result, &mut stack)?;
            }
        }

        Ok(result)
    }

    fn decode_with_expansion_budget(&self, tokens: &[u32], expansion_budget: usize) -> Vec<u32> {
        if self.max_vocab_size == 0 || self.bpe_start == 0 {
            return tokens.to_vec();
        }

        let mut result = Vec::new();
        let mut stack = Vec::new();
        let mut remaining_budget = expansion_budget.min(self.decode_expansion_budget(tokens));

        for &token in tokens {
            stack.push(token);
            while let Some(token) = stack.pop() {
                if remaining_budget == 0 {
                    result.push(self.recover_token_to_base(token));
                    continue;
                }
                remaining_budget = remaining_budget.saturating_sub(1);
                self.expand_token_iterative(token, &mut result, &mut stack);
            }
        }
        result
    }

    /// Expand one token step with graceful modulo handling.
    ///
    /// Handles three token ranges:
    /// 1. Base tokens [0..bpe_start)
    /// 2. Valid BPE tokens [bpe_start..max_vocab_size): Trained merges
    /// 3. Out-of-range tokens: Normalized via modulo, then re-classified
    ///
    /// For tokens outside the vocabulary (>= max_vocab_size), applies modulo to bring
    /// them into valid range. Unknown BPE tokens (in BPE range but not in merge table)
    /// are mapped to base vocabulary for graceful recovery.
    fn expand_token_iterative(&self, token: u32, result: &mut Vec<u32>, stack: &mut Vec<u32>) {
        // Normalize token to valid vocabulary range
        let normalized = token % self.max_vocab_size;

        if normalized < self.bpe_start {
            // Base token - pass through.
            result.push(normalized);
        } else if let Some(&(left, right)) = self.token_to_pair.get(&normalized) {
            // Valid BPE token - expand left before right using the LIFO stack.
            stack.push(right);
            stack.push(left);
        } else {
            // Unknown BPE token (in BPE range but not in merge table)
            // Map to base vocabulary for graceful recovery
            result.push(normalized % self.bpe_start);
        }
    }

    fn expand_token_strict(
        &self,
        token: u32,
        result: &mut Vec<u32>,
        stack: &mut Vec<u32>,
    ) -> Result<(), DecodeError> {
        if token >= self.max_vocab_size {
            return Err(Self::invalid_bpe_token(
                token,
                "token below BPE vocabulary size",
            ));
        }

        if token < self.bpe_start {
            result.push(token);
            Ok(())
        } else if let Some(&(left, right)) = self.token_to_pair.get(&token) {
            stack.push(right);
            stack.push(left);
            Ok(())
        } else {
            Err(Self::invalid_bpe_token(
                token,
                "base token or known BPE merge token",
            ))
        }
    }

    fn recover_token_to_base(&self, token: u32) -> u32 {
        let normalized = token % self.max_vocab_size;
        if normalized < self.bpe_start {
            normalized
        } else {
            normalized % self.bpe_start
        }
    }

    fn invalid_bpe_token(token: u32, expected: &'static str) -> DecodeError {
        DecodeError::InvalidTokenInContext {
            token,
            context: EOFContext::new("BpeDecoder", Some("token"), expected),
        }
    }

    fn decode_expansion_budget(&self, tokens: &[u32]) -> usize {
        tokens.iter().fold(0usize, |budget, &token| {
            budget.saturating_add(self.decode_steps_for_token(token))
        })
    }

    fn decode_steps_for_token(&self, token: u32) -> usize {
        let normalized = token % self.max_vocab_size;
        if normalized >= self.bpe_start {
            self.token_to_decode_steps
                .get(&normalized)
                .copied()
                .unwrap_or(1)
        } else {
            1
        }
    }

    fn build_decode_step_budgets(
        token_to_pair: &FxHashMap<u32, (u32, u32)>,
        bpe_start: u32,
        max_vocab_size: u32,
    ) -> FxHashMap<u32, usize> {
        if max_vocab_size == 0 || bpe_start == 0 {
            return FxHashMap::default();
        }

        let mut tokens: Vec<u32> = token_to_pair.keys().copied().collect();
        tokens.sort_unstable();

        let mut budgets =
            FxHashMap::with_capacity_and_hasher(token_to_pair.len(), Default::default());
        for token in tokens {
            if let Some(&(left, right)) = token_to_pair.get(&token) {
                let left_steps =
                    Self::decode_steps_for_component(left, bpe_start, max_vocab_size, &budgets);
                let right_steps =
                    Self::decode_steps_for_component(right, bpe_start, max_vocab_size, &budgets);
                budgets.insert(
                    token,
                    1usize
                        .saturating_add(left_steps)
                        .saturating_add(right_steps),
                );
            }
        }

        budgets
    }

    fn decode_steps_for_component(
        token: u32,
        bpe_start: u32,
        max_vocab_size: u32,
        budgets: &FxHashMap<u32, usize>,
    ) -> usize {
        let normalized = token % max_vocab_size;
        if normalized >= bpe_start {
            budgets.get(&normalized).copied().unwrap_or(1)
        } else {
            1
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bpe::{BpeMerge, BpeMergeTable, BpeMetadata};

    fn create_test_table() -> BpeMergeTable {
        BpeMergeTable {
            schema: "test".to_string(),
            base_vocab_size: 80,
            merges: vec![
                BpeMerge {
                    pair: [5, 12],
                    new_token: 80,
                    frequency: 1000,
                },
                BpeMerge {
                    pair: [80, 7],
                    new_token: 81,
                    frequency: 500,
                },
                BpeMerge {
                    pair: [3, 3],
                    new_token: 82,
                    frequency: 250,
                },
            ],
            metadata: BpeMetadata {
                trained_on_instances: 100,
                training_date: "2025-10-05".to_string(),
                max_merges: 3,
                dict_size: None,
            },
        }
    }

    #[test]
    fn test_encode_single_merge() {
        let table = create_test_table();
        let encoder = BpeEncoder::new(&table, 100);

        // [5, 12] should merge to 80
        let tokens = vec![5, 12];
        let encoded = encoder.encode(&tokens);
        assert_eq!(encoded, vec![80]);
    }

    #[test]
    fn test_encode_chained_merge() {
        let table = create_test_table();
        let encoder = BpeEncoder::new(&table, 100);

        // [5, 12, 7] should merge to [80, 7] then to [81]
        let tokens = vec![5, 12, 7];
        let encoded = encoder.encode(&tokens);
        assert_eq!(encoded, vec![81]);
    }

    #[test]
    fn test_encode_multiple_merges() {
        let table = create_test_table();
        let encoder = BpeEncoder::new(&table, 100);

        // [5, 12, 7, 5, 12] → [80, 7, 80] → [81, 80]
        let tokens = vec![5, 12, 7, 5, 12];
        let encoded = encoder.encode(&tokens);
        assert_eq!(encoded, vec![81, 80]);
    }

    #[test]
    fn test_encode_no_merges() {
        let table = create_test_table();
        let encoder = BpeEncoder::new(&table, 100);

        // No matching pairs
        let tokens = vec![1, 2, 4, 6];
        let encoded = encoder.encode(&tokens);
        assert_eq!(encoded, vec![1, 2, 4, 6]);
    }

    #[test]
    fn test_encode_empty() {
        let table = create_test_table();
        let encoder = BpeEncoder::new(&table, 100);

        let tokens: Vec<u32> = vec![];
        let encoded = encoder.encode(&tokens);
        assert_eq!(encoded, Vec::<u32>::new());
    }

    fn create_sequential_table(schema: &str, merges: &[([u32; 2], u32)]) -> BpeMergeTable {
        BpeMergeTable {
            schema: schema.to_string(),
            base_vocab_size: 80,
            merges: merges
                .iter()
                .enumerate()
                .map(|(index, &(pair, new_token))| BpeMerge {
                    pair,
                    new_token,
                    frequency: (merges.len() - index) as u64,
                })
                .collect(),
            metadata: BpeMetadata {
                trained_on_instances: 1,
                training_date: "2026-04-26".to_string(),
                max_merges: merges.len(),
                dict_size: None,
            },
        }
    }

    fn create_decode_chain_table(depth: usize) -> (BpeMergeTable, Vec<u32>, u32) {
        assert!(depth > 0);
        let mut merges = Vec::with_capacity(depth);
        let mut expected = vec![1, 2];

        merges.push(BpeMerge {
            pair: [1, 2],
            new_token: 80,
            frequency: depth as u64,
        });

        for index in 1..depth {
            let next_base = 3 + (index as u32 % 70);
            expected.push(next_base);
            merges.push(BpeMerge {
                pair: [79 + index as u32, next_base],
                new_token: 80 + index as u32,
                frequency: (depth - index) as u64,
            });
        }

        let last_token = 79 + depth as u32;
        (
            BpeMergeTable {
                schema: "decode-chain".to_string(),
                base_vocab_size: 80,
                metadata: BpeMetadata {
                    trained_on_instances: 1,
                    training_date: "2026-05-01".to_string(),
                    max_merges: merges.len(),
                    dict_size: None,
                },
                merges,
            },
            expected,
            last_token,
        )
    }

    #[test]
    fn test_sequential_encoder_handles_overlapping_pairs_left_to_right() {
        let table = create_sequential_table("sequential-overlap", &[([1, 1], 80)]);
        let encoder = SequentialBpeEncoder::new(&table, 81);

        assert_eq!(encoder.encode(&[1, 1, 1]), vec![80, 1]);
        assert_eq!(encoder.encode(&[1, 1, 1, 1]), vec![80, 80]);
    }

    #[test]
    fn test_sequential_encoder_skips_destroyed_stale_pairs() {
        let table = create_sequential_table("sequential-stale", &[([1, 2], 80), ([2, 3], 81)]);
        let encoder = SequentialBpeEncoder::new(&table, 82);

        assert_eq!(encoder.encode(&[1, 2, 3]), vec![80, 3]);
    }

    #[test]
    fn test_sequential_encoder_enqueues_new_internal_boundary_pair() {
        let table = create_sequential_table(
            "sequential-internal-boundary",
            &[([1, 2], 80), ([80, 80], 81)],
        );
        let encoder = SequentialBpeEncoder::new(&table, 82);

        assert_eq!(encoder.encode(&[1, 2, 1, 2]), vec![81]);
    }

    #[test]
    fn test_sequential_encoder_enqueues_left_boundary_pair() {
        let table =
            create_sequential_table("sequential-left-boundary", &[([1, 2], 80), ([9, 80], 81)]);
        let encoder = SequentialBpeEncoder::new(&table, 82);

        assert_eq!(encoder.encode(&[9, 1, 2]), vec![81]);
    }

    #[test]
    fn test_sequential_encoder_enqueues_right_boundary_pair() {
        let table =
            create_sequential_table("sequential-right-boundary", &[([1, 2], 80), ([80, 9], 81)]);
        let encoder = SequentialBpeEncoder::new(&table, 82);

        assert_eq!(encoder.encode(&[1, 2, 9]), vec![81]);
    }

    #[test]
    fn test_sequential_encoder_does_not_revisit_earlier_merge() {
        let table =
            create_sequential_table("sequential-no-revisit", &[([100, 5], 101), ([1, 2], 100)]);
        let encoder = SequentialBpeEncoder::new(&table, 102);

        assert_eq!(encoder.encode(&[1, 2, 5]), vec![100, 5]);
    }

    #[test]
    fn test_sequential_encoder_respects_vocab_caps() {
        let table =
            create_sequential_table("sequential-vocab-caps", &[([1, 2], 80), ([80, 3], 81)]);

        assert_eq!(
            SequentialBpeEncoder::new(&table, 80).encode(&[1, 2, 3]),
            vec![1, 2, 3]
        );
        assert_eq!(
            SequentialBpeEncoder::new(&table, 81).encode(&[1, 2, 3]),
            vec![80, 3]
        );
        assert_eq!(
            SequentialBpeEncoder::new(&table, 82).encode(&[1, 2, 3]),
            vec![81]
        );
    }

    #[test]
    fn test_sequential_encoder_compact_candidate_positions_round_trip() {
        let candidate = SequentialBpeEncoder::pack_initial_pair_position(12_345, 67_890);

        assert_eq!(SequentialBpeEncoder::candidate_merge_idx(candidate), 12_345);
        assert_eq!(
            SequentialBpeEncoder::candidate_left_position(candidate),
            67_890
        );
    }

    #[test]
    fn test_sequential_encoder_returns_owned_tokens_without_candidates() {
        let table = create_sequential_table("sequential-no-candidates", &[([10, 11], 80)]);
        let encoder = SequentialBpeEncoder::new(&table, 81);

        assert_eq!(encoder.encode(&[1, 2, 3]), vec![1, 2, 3]);
    }

    #[test]
    fn test_compact_linked_tokens_preserves_linked_order_in_owned_buffer() {
        let tokens = vec![10, 20, 30, 40];
        let next = vec![2, NO_LINK, 3, NO_LINK];

        assert_eq!(
            SequentialBpeEncoder::compact_linked_tokens(tokens, &next),
            vec![10, 30, 40]
        );
    }

    #[test]
    fn test_full_scan_fallback_preserves_sequential_semantics() {
        let table = create_sequential_table(
            "sequential-full-scan-fallback",
            &[([1, 2], 80), ([80, 3], 81), ([3, 3], 82)],
        );
        let encoder = SequentialBpeEncoder::new(&table, 83);

        assert_eq!(
            encoder.encode_with_full_merge_scan(&[1, 2, 3, 3, 3]),
            encoder.encode(&[1, 2, 3, 3, 3])
        );
    }

    #[test]
    fn test_decode_single_token() {
        let table = create_test_table();
        let decoder = BpeDecoder::new(&table, 100);

        // 80 → [5, 12]
        let tokens = vec![80];
        let decoded = decoder.decode(&tokens);
        assert_eq!(decoded, vec![5, 12]);
    }

    #[test]
    fn test_decode_chained_merge() {
        let table = create_test_table();
        let decoder = BpeDecoder::new(&table, 100);

        // 81 → [80, 7] → [5, 12, 7]
        let tokens = vec![81];
        let decoded = decoder.decode(&tokens);
        assert_eq!(decoded, vec![5, 12, 7]);
    }

    #[test]
    fn test_decoder_expands_deep_chain_without_recursive_calls() {
        let (table, expected, last_token) = create_decode_chain_table(4096);
        let decoder = BpeDecoder::new(&table, last_token + 1);

        assert_eq!(decoder.decode(&[last_token]), expected);
    }

    #[test]
    fn test_decoder_expansion_budget_recovers_remaining_tokens() {
        let (table, _expected, last_token) = create_decode_chain_table(3);
        let decoder = BpeDecoder::new(&table, last_token + 1);

        assert_eq!(
            decoder.decode_with_expansion_budget(&[last_token], 2),
            vec![0, 4, 5]
        );
    }

    #[test]
    fn test_decode_multiple_tokens() {
        let table = create_test_table();
        let decoder = BpeDecoder::new(&table, 100);

        // [81, 80] → [5, 12, 7, 5, 12]
        let tokens = vec![81, 80];
        let decoded = decoder.decode(&tokens);
        assert_eq!(decoded, vec![5, 12, 7, 5, 12]);
    }

    #[test]
    fn test_decode_base_tokens() {
        let table = create_test_table();
        let decoder = BpeDecoder::new(&table, 100);

        // Base tokens pass through unchanged
        let tokens = vec![1, 2, 3, 4];
        let decoded = decoder.decode(&tokens);
        assert_eq!(decoded, vec![1, 2, 3, 4]);
    }

    #[test]
    fn test_round_trip() {
        let table = create_test_table();
        let encoder = BpeEncoder::new(&table, 100);
        let decoder = BpeDecoder::new(&table, 100);

        let original = vec![5, 12, 7, 5, 12, 3, 3];
        let encoded = encoder.encode(&original);
        let decoded = decoder.decode(&encoded);

        assert_eq!(decoded, original);
    }

    #[test]
    fn test_limited_vocab_size() {
        let table = create_test_table();
        // Only use first merge (vocab = 80 + 1 = 81)
        let encoder = BpeEncoder::new(&table, 81);

        // Should only apply first merge [5,12]→80
        let tokens = vec![5, 12, 7];
        let encoded = encoder.encode(&tokens);
        assert_eq!(encoded, vec![80, 7]); // No second merge
    }

    // ============================================================================
    // Tests for Best-First Merge Selection (Nanochat-style correctness)
    // ============================================================================

    /// Create test table specifically for testing merge priority
    ///
    /// Merges are ordered by frequency (descending), with token IDs
    /// reflecting this priority (lower ID = higher frequency):
    /// - (12, 12) → 100  (highest frequency: 5000)
    /// - (5, 12) → 101   (lower frequency: 1000)
    fn create_priority_test_table() -> BpeMergeTable {
        BpeMergeTable {
            schema: "priority-test".to_string(),
            base_vocab_size: 80,
            merges: vec![
                BpeMerge {
                    pair: [12, 12],
                    new_token: 100, // Lower ID = trained first = higher priority
                    frequency: 5000,
                },
                BpeMerge {
                    pair: [5, 12],
                    new_token: 101, // Higher ID = trained later = lower priority
                    frequency: 1000,
                },
            ],
            metadata: BpeMetadata {
                trained_on_instances: 100,
                training_date: "2025-10-17".to_string(),
                max_merges: 2,
                dict_size: None,
            },
        }
    }

    #[test]
    fn test_best_first_merge_selection() {
        // This test captures the key difference between greedy left-to-right
        // and the legacy encoder's best-first merge selection.
        //
        // Given: [5, 12, 12, 7]
        // Merges: (12,12)→100 (high priority), (5,12)→101 (low priority)
        //
        // Greedy left-to-right (OLD, INCORRECT):
        //   - Scans left, finds (5,12) first
        //   - Applies it: [101, 12, 7]
        //   - Cannot apply (12,12) anymore - pair broken!
        //   - Result: [101, 12, 7] (suboptimal)
        //
        // Best-first (NEW, CORRECT):
        //   - Finds all pairs: {(5,12)→101, (12,12)→100, (12,7)→none}
        //   - Selects best: (12,12)→100 (lowest token ID = highest priority)
        //   - Applies it: [5, 100, 7]
        //   - Result: [5, 100, 7] (optimal)

        let table = create_priority_test_table();
        let encoder = BpeEncoder::new(&table, 150);

        let tokens = vec![5, 12, 12, 7];
        let encoded = encoder.encode(&tokens);

        // Should apply high-priority merge first, NOT greedy left-to-right
        assert_eq!(
            encoded,
            vec![5, 100, 7],
            "Expected best-first selection to apply (12,12)→100 (high priority), \
             but got greedy left-to-right which applied (5,12)→101 (low priority)"
        );
    }

    #[test]
    fn test_best_first_with_multiple_candidates() {
        // Test case with multiple merge candidates at different positions
        //
        // Given: [5, 12, 99, 12, 12]
        // Merges: (12,12)→100 (priority 1), (5,12)→101 (priority 2)
        //
        // Best-first should apply (12,12) at position 3-4 first:
        //   [5, 12, 99, 12, 12] → [5, 12, 99, 100]
        // Then apply (5,12):
        //   [5, 12, 99, 100] → [101, 99, 100]

        let table = create_priority_test_table();
        let encoder = BpeEncoder::new(&table, 150);

        let tokens = vec![5, 12, 99, 12, 12];
        let encoded = encoder.encode(&tokens);

        // Optimal compression applies high-priority merges first
        assert_eq!(
            encoded,
            vec![101, 99, 100],
            "Expected best-first to prioritize (12,12)→100 over (5,12)→101"
        );
    }

    #[test]
    fn test_best_first_recursive_application() {
        // Test that after applying a merge, we reconsider all pairs
        //
        // Given: [12, 12, 12, 12]
        // Merges: (12,12)→100 (priority 1), (100,100)→102 (priority 3)
        //
        // Best-first should:
        //   1. Apply (12,12) at position 0: [100, 12, 12]
        //   2. Apply (12,12) at position 1: [100, 100]
        //   3. Apply (100,100): [102]

        let mut table = create_priority_test_table();
        table.merges.push(BpeMerge {
            pair: [100, 100],
            new_token: 102,
            frequency: 3000,
        });
        table.metadata.max_merges = 3;

        let encoder = BpeEncoder::new(&table, 150);

        let tokens = vec![12, 12, 12, 12];
        let encoded = encoder.encode(&tokens);

        // Should recursively merge: [12,12,12,12] → [100,100] → [102]
        assert_eq!(
            encoded,
            vec![102],
            "Expected recursive merge application to produce [102]"
        );
    }

    // ============================================================================
    // Comprehensive Round-Trip Tests with Best-First Encoding
    // ============================================================================

    #[test]
    fn test_round_trip_with_priority_merges() {
        // Test that decode correctly reverses best-first encoding
        // even when merge order differs from greedy left-to-right
        let table = create_priority_test_table();
        let encoder = BpeEncoder::new(&table, 150);
        let decoder = BpeDecoder::new(&table, 150);

        // This sequence triggers best-first vs greedy difference
        let original = vec![5, 12, 12, 7];
        let encoded = encoder.encode(&original);

        // Encoding produces [5, 100, 7] via best-first
        assert_eq!(encoded, vec![5, 100, 7]);

        // Decoding should restore original
        let decoded = decoder.decode(&encoded);
        assert_eq!(decoded, original);
    }

    #[test]
    fn test_round_trip_deep_recursion() {
        // Test decoder handles deeply nested merges
        let mut table = create_priority_test_table();
        table.merges.push(BpeMerge {
            pair: [100, 100],
            new_token: 102,
            frequency: 3000,
        });
        table.metadata.max_merges = 3;

        let encoder = BpeEncoder::new(&table, 150);
        let decoder = BpeDecoder::new(&table, 150);

        let original = vec![12, 12, 12, 12];
        let encoded = encoder.encode(&original);

        // Encoder produces [102] via nested merges
        assert_eq!(encoded, vec![102]);

        // Decoder should recursively expand: 102 → [100,100] → [12,12,12,12]
        let decoded = decoder.decode(&encoded);
        assert_eq!(decoded, original);
    }

    #[test]
    fn test_round_trip_mixed_tokens() {
        // Test round trip with mix of base and BPE tokens
        let table = create_test_table();
        let encoder = BpeEncoder::new(&table, 100);
        let decoder = BpeDecoder::new(&table, 100);

        // Mix of mergeable and non-mergeable patterns (all valid base tokens < 80)
        let original = vec![1, 2, 5, 12, 7, 79, 3, 3, 5, 12];
        let encoded = encoder.encode(&original);
        let decoded = decoder.decode(&encoded);

        assert_eq!(decoded, original);
    }

    #[test]
    fn test_round_trip_empty() {
        let table = create_test_table();
        let encoder = BpeEncoder::new(&table, 100);
        let decoder = BpeDecoder::new(&table, 100);

        let original: Vec<u32> = vec![];
        let encoded = encoder.encode(&original);
        let decoded = decoder.decode(&encoded);

        assert_eq!(decoded, original);
    }

    #[test]
    fn test_round_trip_single_base_token() {
        let table = create_test_table();
        let encoder = BpeEncoder::new(&table, 100);
        let decoder = BpeDecoder::new(&table, 100);

        let original = vec![42];
        let encoded = encoder.encode(&original);
        let decoded = decoder.decode(&encoded);

        assert_eq!(decoded, original);
    }

    #[test]
    fn test_decoder_correctness_independent_of_encoding_order() {
        // This test verifies that decoder works correctly regardless of
        // whether encoding used greedy or best-first approach.
        //
        // The key insight: decoding only depends on the merge table structure,
        // not on the order merges were applied during encoding.

        let table = create_priority_test_table();
        let decoder = BpeDecoder::new(&table, 150);

        // Manually construct encoded sequence that could come from either approach
        // [5, 100, 7] could be produced by:
        // - Best-first: [5,12,12,7] → [5,100,7] (applied (12,12) first)
        // - Hypothetical: some other merge order

        let encoded = vec![5, 100, 7];
        let decoded = decoder.decode(&encoded);

        // Decoder should always expand to same base tokens
        assert_eq!(decoded, vec![5, 12, 12, 7]);
    }

    #[test]
    fn test_decoder_handles_invalid_tokens_gracefully() {
        // Test that decoder handles tokens outside vocabulary with modulo
        let table = create_test_table();
        let decoder = BpeDecoder::new(&table, 100);

        // 999 is way outside vocabulary (max_vocab_size = 100)
        // 999 % 100 = 99, which is in BPE range [80..100) but not in merge table
        // So it maps to base range: 99 % 80 = 19
        let tokens = vec![5, 999, 12];
        let decoded = decoder.decode(&tokens);

        // Should apply modulo to bring into valid range
        assert_eq!(decoded, vec![5, 19, 12]);
    }

    #[test]
    fn test_decoder_strict_rejects_unknown_bpe_tokens() {
        let table = create_test_table();
        let decoder = BpeDecoder::new(&table, 100);

        assert_eq!(decoder.decode_strict(&[81]).unwrap(), vec![5, 12, 7]);

        assert_eq!(
            decoder.decode(&[85]),
            vec![5],
            "recovery decode should normalize unknown BPE-range tokens to base"
        );
        assert!(matches!(
            decoder.decode_strict(&[85]),
            Err(crate::tokenizer::DecodeError::InvalidTokenInContext { token: 85, context })
                if context.type_name == "BpeDecoder"
                    && context.field_name == Some("token")
                    && context.expected == "base token or known BPE merge token"
        ));

        assert_eq!(
            decoder.decode(&[999]),
            vec![19],
            "recovery decode should modulo-normalize out-of-range tokens"
        );
        assert!(matches!(
            decoder.decode_strict(&[999]),
            Err(crate::tokenizer::DecodeError::InvalidTokenInContext { token: 999, context })
                if context.type_name == "BpeDecoder"
                    && context.field_name == Some("token")
                    && context.expected == "token below BPE vocabulary size"
        ));
    }
}
