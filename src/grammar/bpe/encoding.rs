//! Emits generated BPE encode helpers.

use proc_macro2::TokenStream;
use quote::quote;

pub(super) fn render_encoding_helpers() -> TokenStream {
    quote! {
        /// Encode base tokens to BPE tokens
        ///
        /// Uses priority-ordered merge application: merges are applied in order
        /// of their training frequency (merge 0 = highest frequency = highest priority).
        /// This ensures that higher-value merges are not blocked by lower-priority ones.
        ///
        /// # Arguments
        /// * `tokens` - Base token sequence
        /// * `max_vocab_size` - Maximum vocabulary size (controls merges applied)
        ///
        /// # Returns
        /// BPE-compressed token sequence
        pub fn encode(tokens: &[u32], max_vocab_size: u32) -> Vec<u32> {
            encode_owned(tokens.to_vec(), max_vocab_size)
        }

        /// Encode owned base tokens to BPE tokens without cloning the input.
        pub fn encode_owned(mut current: Vec<u32>, max_vocab_size: u32) -> Vec<u32> {
            if current.is_empty() || max_vocab_size <= BPE_START {
                return current;
            }

            let max_merges = ((max_vocab_size - BPE_START) as usize).min(MAX_MERGES);
            if max_merges == 0 || current.len() < 2 {
                return current;
            }
            if current.len() > u32::MAX as usize || max_merges > u32::MAX as usize {
                return encode_owned_with_full_merge_scan(current, max_merges);
            }

            let token_count = current.len();
            let initial_pair_positions = build_initial_pair_positions(&current, max_merges);
            if initial_pair_positions.is_empty() {
                return current;
            }

            let mut next: Vec<u32> = (0..token_count)
                .map(|idx| if idx + 1 < token_count { (idx + 1) as u32 } else { NO_LINK })
                .collect();
            let mut prev: Vec<u32> = (0..token_count)
                .map(|idx| if idx == 0 { NO_LINK } else { (idx - 1) as u32 })
                .collect();
            let mut alive = vec![true; token_count];
            let mut initial_cursor = 0usize;
            let mut future_positions: FxHashMap<u32, PositionBucket> = FxHashMap::default();
            let mut position_pool: Vec<Vec<u32>> = Vec::new();

            {
                let mut index_state = PairIndexState {
                    current: &mut current,
                    next: &mut next,
                    prev: &mut prev,
                    alive: &mut alive,
                    future_positions: &mut future_positions,
                    position_pool: &mut position_pool,
                };

                // Apply merges in priority order (merge 0 first, then 1, etc.)
                // This ensures higher-frequency merges are applied before lower-frequency ones,
                // matching the behavior of the BPE training simulation.
                for merge_idx in 0..max_merges {
                    if initial_cursor >= initial_pair_positions.len()
                        && index_state.future_positions.is_empty()
                    {
                        break;
                    }

                    let initial_start = initial_cursor;
                    while initial_cursor < initial_pair_positions.len()
                        && candidate_merge_idx(initial_pair_positions[initial_cursor]) == merge_idx
                    {
                        initial_cursor += 1;
                    }
                    let initial_positions = &initial_pair_positions[initial_start..initial_cursor];
                    let pair = MERGE_PAIRS[merge_idx];
                    let new_token = MERGE_TOKENS[merge_idx];

                    match index_state.future_positions.remove(&(merge_idx as u32)) {
                        Some(PositionBucket::One(position)) => {
                            if initial_positions.is_empty() {
                                apply_merge_positions(
                                    std::iter::once(position),
                                    pair,
                                    new_token,
                                    merge_idx,
                                    max_merges,
                                    &mut index_state,
                                );
                            } else {
                                let mut positions = index_state.position_pool.pop().unwrap_or_default();
                                positions.push(position);
                                positions.extend(
                                    initial_positions
                                        .iter()
                                        .map(|&candidate| candidate_left_position(candidate)),
                                );
                                positions.sort_unstable();
                                positions.dedup();
                                apply_merge_positions(
                                    positions.iter().copied(),
                                    pair,
                                    new_token,
                                    merge_idx,
                                    max_merges,
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
                                    .map(|&candidate| candidate_left_position(candidate)),
                            );
                            positions.sort_unstable();
                            positions.dedup();
                            apply_merge_positions(
                                positions.iter().copied(),
                                pair,
                                new_token,
                                merge_idx,
                                max_merges,
                                &mut index_state,
                            );
                            positions.clear();
                            index_state.position_pool.push(positions);
                        }
                        None => {
                            apply_merge_positions(
                                initial_positions
                                    .iter()
                                    .map(|&candidate| candidate_left_position(candidate)),
                                pair,
                                new_token,
                                merge_idx,
                                max_merges,
                                &mut index_state,
                            );
                        }
                    }
                }
            }

            compact_linked_tokens(current, &next)
        }

        /// Build sorted initial merge-position candidates without per-pair buckets.
        #[inline]
        fn build_initial_pair_positions(
            tokens: &[u32],
            max_merges: usize,
        ) -> Vec<u64> {
            let mut positions = Vec::new();

            for left in 0..tokens.len().saturating_sub(1) {
                let pair = (tokens[left], tokens[left + 1]);
                if let Some(merge_idx) = merge_index_for_pair(pair, max_merges) {
                    positions.push(pack_initial_pair_position(merge_idx, left));
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

        fn encode_owned_with_full_merge_scan(
            mut current: Vec<u32>,
            max_merges: usize,
        ) -> Vec<u32> {
            for merge_idx in 0..max_merges {
                apply_full_scan_merge(
                    &mut current,
                    MERGE_PAIRS[merge_idx],
                    MERGE_TOKENS[merge_idx],
                );
            }
            current
        }

        fn apply_full_scan_merge(tokens: &mut Vec<u32>, pair: (u32, u32), new_token: u32) {
            let mut read = 0usize;
            let mut write = 0usize;

            while read < tokens.len() {
                if read + 1 < tokens.len()
                    && tokens[read] == pair.0
                    && tokens[read + 1] == pair.1
                {
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
            max_merges: usize,
            state: &mut PairIndexState<'_>,
        )
        where
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

                push_current_pair(state.prev[left], merge_idx, max_merges, state);
                push_current_pair(left_position, merge_idx, max_merges, state);
            }
        }

        /// Queue the adjacent pair starting at `left` if a future merge can consume it.
        #[inline]
        fn push_current_pair(
            left_position: u32,
            current_merge_idx: usize,
            max_merges: usize,
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
            let Some(merge_idx) = merge_index_for_pair(pair, max_merges) else {
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

        /// Return the merge priority index for a pair within this vocabulary size.
        #[inline]
        fn merge_index_for_pair(
            pair: (u32, u32),
            max_merges: usize,
        ) -> Option<usize> {
            let pair_key = merge_pair_lookup_key(pair);
            let table_idx = MERGE_PAIR_INDEX
                .binary_search_by_key(&pair_key, |(key, _)| *key)
                .ok()?;
            let merge_idx = MERGE_PAIR_INDEX[table_idx].1;
            (merge_idx < max_merges).then_some(merge_idx)
        }

        #[inline]
        fn merge_pair_lookup_key(pair: (u32, u32)) -> u64 {
            ((pair.0 as u64) << u32::BITS) | pair.1 as u64
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

        /// Look up a merge pair and return the merged token if it exists
        ///
        /// Uses the static sorted pair-index table, avoiding a runtime hash-map build.
        ///
        /// # Arguments
        /// * `left` - Left token of the pair
        /// * `right` - Right token of the pair
        /// * `max_vocab_size` - Maximum vocabulary size (controls which merges are available)
        ///
        /// # Returns
        /// Some(merged_token) if the pair can be merged, None otherwise
        pub fn lookup_merge(left: u32, right: u32, max_vocab_size: u32) -> Option<u32> {
            if max_vocab_size <= BPE_START {
                return None;
            }
            let max_merges = ((max_vocab_size - BPE_START) as usize).min(MAX_MERGES);
            let merge_idx = merge_index_for_pair((left, right), max_merges)?;
            let merged = MERGE_TOKENS[merge_idx];
            (merged < max_vocab_size).then_some(merged)
        }
    }
}
