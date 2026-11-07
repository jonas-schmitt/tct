//! Internal state for optimized BPE training.
//!
//! This module owns the deduplicated word representation, merge candidate heap
//! ordering, average-length accounting, and parallel pair-count aggregation
//! used by the incremental trainer in `src/bpe/trainer.rs`.

use ahash::{AHashMap, AHashSet};
use rayon::prelude::*;
use std::cmp::Ordering as CmpOrdering;

use super::Pair;

#[derive(Clone, Debug)]
pub(super) struct Word {
    pub(super) ids: Vec<u32>,
    /// When set, length equals `ids` and marks structure/discriminator positions
    /// that may participate in structure-only merges. Value / value-end positions
    /// are false and never enter pair counts or merge applications.
    pub(super) structure_safe: Option<Vec<bool>>,
}

impl Word {
    #[inline]
    pub(super) fn new(ids: Vec<u32>) -> Self {
        Self {
            ids,
            structure_safe: None,
        }
    }

    #[inline]
    pub(super) fn with_structure_safe(ids: Vec<u32>, structure_safe: Vec<bool>) -> Self {
        debug_assert_eq!(
            ids.len(),
            structure_safe.len(),
            "structure-safe flags must align 1:1 with token ids"
        );
        Self {
            ids,
            structure_safe: Some(structure_safe),
        }
    }

    #[inline]
    fn pair_mergeable_at(&self, index: usize) -> bool {
        match &self.structure_safe {
            None => true,
            Some(flags) => {
                flags.get(index).copied().unwrap_or(false)
                    && flags.get(index + 1).copied().unwrap_or(false)
            }
        }
    }

    #[inline]
    fn token_mergeable_at(&self, index: usize) -> bool {
        match &self.structure_safe {
            None => true,
            Some(flags) => flags.get(index).copied().unwrap_or(false),
        }
    }

    #[inline]
    fn pairs(&self) -> impl Iterator<Item = Pair> + '_ {
        (0..self.ids.len().saturating_sub(1))
            .filter(|&index| self.pair_mergeable_at(index))
            .map(|index| (self.ids[index], self.ids[index + 1]))
    }

    pub(super) fn merge_pair(&mut self, pair: Pair, new_id: u32) -> Vec<(Pair, i64)> {
        let (left_token, right_token) = pair;
        let len = self.ids.len();
        if len < 2 {
            return Vec::new();
        }

        let mut out: Vec<u32> = Vec::with_capacity(len);
        let mut out_safe: Option<Vec<bool>> = self
            .structure_safe
            .as_ref()
            .map(|_| Vec::with_capacity(len));
        let mut deltas: Vec<(Pair, i64)> = Vec::with_capacity(6);

        let mut index = 0usize;
        while index < len {
            if index + 1 < len
                && self.ids[index] == left_token
                && self.ids[index + 1] == right_token
                && self.pair_mergeable_at(index)
            {
                let left = out.last().copied();
                let left_mergeable = out_safe
                    .as_ref()
                    .map(|flags| flags.last().copied().unwrap_or(false))
                    .unwrap_or(true);
                let right = if index + 2 < len {
                    Some(self.ids[index + 2])
                } else {
                    None
                };
                let right_mergeable = if index + 2 < len {
                    self.token_mergeable_at(index + 2)
                } else {
                    false
                };

                // Only emit pair-count deltas for structure-mergeable adjacencies.
                if let Some(token) = left
                    && left_mergeable
                {
                    deltas.push(((token, left_token), -1));
                    deltas.push(((token, new_id), 1));
                }
                deltas.push(((left_token, right_token), -1));
                if let Some(token) = right
                    && right_mergeable
                {
                    deltas.push(((right_token, token), -1));
                    deltas.push(((new_id, token), 1));
                }

                out.push(new_id);
                if let Some(flags) = out_safe.as_mut() {
                    // Merged from two structure-safe tokens.
                    flags.push(true);
                }
                index += 2;
            } else {
                out.push(self.ids[index]);
                if let Some(flags) = out_safe.as_mut() {
                    flags.push(self.token_mergeable_at(index));
                }
                index += 1;
            }
        }

        self.ids = out;
        self.structure_safe = out_safe;
        deltas
    }
}

#[derive(Debug, Eq)]
pub(super) struct MergeJob {
    pub(super) pair: Pair,
    pub(super) count: u64,
    pub(super) positions: AHashSet<usize>,
}

impl PartialEq for MergeJob {
    fn eq(&self, other: &Self) -> bool {
        self.count == other.count && self.pair == other.pair
    }
}

impl PartialOrd for MergeJob {
    fn partial_cmp(&self, other: &Self) -> Option<CmpOrdering> {
        Some(self.cmp(other))
    }
}

impl Ord for MergeJob {
    fn cmp(&self, other: &Self) -> CmpOrdering {
        if self.count != other.count {
            self.count.cmp(&other.count)
        } else {
            other.pair.cmp(&self.pair)
        }
    }
}

/// Total number of tokens across all (deduplicated) words, weighted by how many
/// documents each unique word represents.
///
/// This is the numerator of the average encoded sequence length. The trainer
/// seeds it once and then maintains it incrementally: every merge removes an
/// exactly known number of tokens, so the running average can be recomputed in
/// O(1) after each merge instead of rescanning every word.
pub(super) fn compute_total_tokens(words: &[Word], counts: &[i64]) -> i64 {
    words
        .iter()
        .zip(counts.iter())
        .map(|(word, &count)| word.ids.len() as i64 * count)
        .sum()
}

pub(super) fn count_pairs_parallel(
    words: &[Word],
    counts: &[i64],
) -> (AHashMap<Pair, i64>, AHashMap<Pair, AHashSet<usize>>) {
    words
        .par_iter()
        .enumerate()
        .map(|(index, word)| {
            let mut local_counts: AHashMap<Pair, i64> = AHashMap::new();
            let mut local_positions: AHashMap<Pair, AHashSet<usize>> = AHashMap::new();
            if word.ids.len() >= 2 && counts[index] != 0 {
                for pair in word.pairs() {
                    *local_counts.entry(pair).or_default() += counts[index];
                    local_positions.entry(pair).or_default().insert(index);
                }
            }
            (local_counts, local_positions)
        })
        .reduce(
            || (AHashMap::new(), AHashMap::new()),
            |(mut acc_counts, mut acc_positions), (counts, positions)| {
                for (pair, count) in counts {
                    *acc_counts.entry(pair).or_default() += count;
                }
                for (pair, position_set) in positions {
                    acc_positions.entry(pair).or_default().extend(position_set);
                }
                (acc_counts, acc_positions)
            },
        )
}
