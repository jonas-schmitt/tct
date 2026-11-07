//! BPE Training Module
//!
//! Implements incremental BPE training using the nanochat/tiktoken algorithm.
//!
//! # Algorithm
//!
//! The correct BPE training algorithm (as used by tiktoken/nanochat):
//! 1. Count pair frequencies across all sequences
//! 2. Select highest-frequency pair
//! 3. Create merge for that pair
//! 4. **Apply merge to ALL sequences** (simulate encoding)
//! 5. Recalculate pair frequencies on updated sequences
//! 6. Repeat until max_merges reached
//!
//! This ensures that merge selections reflect how earlier merges affect
//! later pair distributions, producing optimal compression.

use ahash::{AHashMap, AHashSet};
use std::collections::BinaryHeap;

use super::{BpeMerge, BpeMergeTable, BpeMetadata, bpe_start_token};

type Pair = (u32, u32);

mod training_state;
use training_state::{MergeJob, Word, compute_total_tokens, count_pairs_parallel};

/// Role constants aligned with generated `tokenizer::role_export`.
pub const ROLE_STRUCTURE: u8 = 0;
pub const ROLE_VALUE: u8 = 1;
pub const ROLE_VALUE_END: u8 = 2;
pub const ROLE_DISCRIMINATOR: u8 = 3;

/// Whether a role may participate in structure-only merges.
///
/// Structure and oneOf/enum discriminators may merge. Value payload bytes and
/// value-end markers (e.g. `END_STRING_MARKER`) may not.
#[inline]
pub fn role_is_structure_mergeable(role: u8) -> bool {
    matches!(role, ROLE_STRUCTURE | ROLE_DISCRIMINATOR)
}

/// Options for the shared optimized BPE trainer.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct BpeTrainingOptions {
    /// Stop once the average encoded sequence length reaches this target.
    ///
    /// The trainer tracks the average incrementally and checks it after every
    /// merge, stopping at whichever merge count lands the average closest to the
    /// target (the merge either side of the crossing point, whichever is nearer).
    pub target_avg_length: Option<f64>,
    /// Emit progress for every merge instead of percentage-style milestones.
    pub verbose: bool,
    /// Parallel per-token roles (same shape as `sequences`) enabling
    /// structure-only merges: pairs are counted and applied only where both
    /// sides are structure-mergeable. Roles must be 1:1 with base tokens.
    pub structure_only_roles: Option<Vec<Vec<u8>>>,
}

/// Progress events emitted by the shared optimized BPE trainer.
#[derive(Debug, Clone, PartialEq)]
pub enum BpeTrainingProgress {
    Deduplicating {
        sequence_count: usize,
    },
    Deduplicated {
        unique_sequences: usize,
    },
    InitialAverageLength {
        average: f64,
    },
    InitialPairCounts {
        unique_pairs: usize,
    },
    StartingMergeLoop {
        max_merges: usize,
    },
    NoMorePairs {
        merges_done: u32,
    },
    TargetAverageReached {
        current_average: f64,
        target: f64,
        merges_done: u32,
    },
    Merge {
        merges_done: u32,
        current_average: f64,
        vocab_size: u32,
        pair: Pair,
        new_token: u32,
        frequency: u64,
    },
    Completed {
        merges: usize,
    },
}

/// Train BPE merges using incremental simulation
///
/// # Arguments
/// * `sequences` - Token sequences to train on (will be consumed and modified)
/// * `max_merges` - Maximum number of merges to generate
/// * `base_vocab_size` - Size of the base vocabulary (e.g., 256 for bytes)
///
/// # Returns
/// Vector of BPE merges in the order they should be applied
pub fn train_bpe_incremental(
    sequences: Vec<Vec<u32>>,
    max_merges: usize,
    base_vocab_size: u32,
) -> Vec<BpeMerge> {
    train_bpe_incremental_with_options(
        sequences,
        max_merges,
        base_vocab_size,
        BpeTrainingOptions::default(),
        |_| {},
    )
}

/// Train BPE merges using the optimized production trainer and progress events.
///
/// This is the implementation used by `tct-train-bpe`. It deduplicates token
/// sequences, tracks pair positions, uses a heap for best-pair selection, and
/// applies delta updates after each merge instead of fully recounting every
/// sequence for every merge.
pub fn train_bpe_incremental_with_options<F>(
    sequences: Vec<Vec<u32>>,
    max_merges: usize,
    base_vocab_size: u32,
    options: BpeTrainingOptions,
    mut progress: F,
) -> Vec<BpeMerge>
where
    F: FnMut(BpeTrainingProgress),
{
    if max_merges == 0 || sequences.is_empty() {
        return Vec::new();
    }

    if let Some(ref roles) = options.structure_only_roles {
        assert_eq!(
            roles.len(),
            sequences.len(),
            "structure-only roles must have one role sequence per token sequence"
        );
        for (index, (sequence, role_seq)) in sequences.iter().zip(roles.iter()).enumerate() {
            assert_eq!(
                role_seq.len(),
                sequence.len(),
                "structure-only roles[{index}] length {} != tokens length {}",
                role_seq.len(),
                sequence.len()
            );
        }
    }

    let bpe_start = bpe_start_token(base_vocab_size);
    let mut merges = Vec::with_capacity(max_merges);

    progress(BpeTrainingProgress::Deduplicating {
        sequence_count: sequences.len(),
    });

    let num_documents: i64 = sequences.len() as i64;
    // Token sequences determine roles (decode-replay is pure), so dedupe by
    // tokens alone and keep the first aligned role vector co-located with the
    // count. Co-location avoids a second map lookup that would need an internal
    // expect for an already-proven 1:1 key relationship.
    let structure_only_roles = options.structure_only_roles;
    let mut sequence_entries: AHashMap<Vec<u32>, (i64, Option<Vec<bool>>)> = AHashMap::new();
    for (index, sequence) in sequences.into_iter().enumerate() {
        let role_safe = structure_only_roles.as_ref().map(|roles| {
            roles[index]
                .iter()
                .copied()
                .map(role_is_structure_mergeable)
                .collect::<Vec<bool>>()
        });
        sequence_entries
            .entry(sequence)
            .and_modify(|(count, _roles)| *count += 1)
            .or_insert((1, role_safe));
    }
    progress(BpeTrainingProgress::Deduplicated {
        unique_sequences: sequence_entries.len(),
    });

    let mut deduplicated_sequences: Vec<_> = sequence_entries.into_iter().collect();
    // AHashMap iteration is intentionally randomized; keep word indices stable
    // so reproducibility does not depend on process hash seeds or host details.
    deduplicated_sequences.sort_by(|(left, _), (right, _)| left.cmp(right));

    let mut words: Vec<Word> = Vec::with_capacity(deduplicated_sequences.len());
    let mut counts: Vec<i64> = Vec::with_capacity(deduplicated_sequences.len());
    for (sequence, (count, role_safe)) in deduplicated_sequences {
        // `role_safe` is `Some` exactly when structure-only roles were supplied:
        // every insert path records the co-located role vector at first sighting.
        if let Some(safe) = role_safe {
            words.push(Word::with_structure_safe(sequence, safe));
        } else {
            words.push(Word::new(sequence));
        }
        counts.push(count);
    }

    // `total_tokens` is maintained exactly and incrementally from here on: each
    // merge below decrements it by the number of tokens that merge removes, so
    // the average is available in O(1) after every merge.
    let mut total_tokens = compute_total_tokens(&words, &counts);
    let initial_avg = total_tokens as f64 / num_documents as f64;
    progress(BpeTrainingProgress::InitialAverageLength {
        average: initial_avg,
    });

    // If the corpus already sits at or below the target, no merge can get the
    // average closer to it (merges only ever shorten sequences), so stop now.
    if let Some(target) = options.target_avg_length
        && initial_avg <= target
    {
        progress(BpeTrainingProgress::TargetAverageReached {
            current_average: initial_avg,
            target,
            merges_done: 0,
        });
        return merges;
    }

    let (mut pair_counts, mut where_to_update) = count_pairs_parallel(&words, &counts);
    progress(BpeTrainingProgress::InitialPairCounts {
        unique_pairs: pair_counts.len(),
    });

    let mut heap = BinaryHeap::with_capacity(pair_counts.len());
    for (pair, positions) in where_to_update.drain() {
        let count = *pair_counts.get(&pair).unwrap_or(&0);
        if count > 0 {
            heap.push(MergeJob {
                pair,
                count: count as u64,
                positions,
            });
        }
    }

    progress(BpeTrainingProgress::StartingMergeLoop { max_merges });

    let mut merges_done = 0u32;
    let mut last_log_percent = 0u32;
    // Average encoded length before the merge currently being applied. Used to
    // decide, when we cross the target, whether the merge just before or just
    // after the crossing lands the average closest to the target.
    let mut previous_average = initial_avg;
    let mut pending_pos: AHashMap<Pair, AHashSet<usize>> = AHashMap::new();

    while (merges_done as usize) < max_merges {
        let Some(mut top) = heap.pop() else {
            progress(BpeTrainingProgress::NoMorePairs { merges_done });
            break;
        };

        if let Some(extra_pos) = pending_pos.remove(&top.pair) {
            top.positions.extend(extra_pos);
        }

        let current_count = *pair_counts.get(&top.pair).unwrap_or(&0);
        if top.count != current_count as u64 {
            pending_pos
                .entry(top.pair)
                .or_default()
                .extend(top.positions);
            heap.push(MergeJob {
                pair: top.pair,
                count: current_count as u64,
                positions: AHashSet::new(),
            });
            continue;
        }

        if top.count == 0 {
            break;
        }

        let new_token = bpe_start + merges_done;
        merges.push(BpeMerge {
            pair: [top.pair.0, top.pair.1],
            new_token,
            frequency: top.count,
        });

        let mut new_positions: AHashMap<Pair, AHashSet<usize>> = AHashMap::new();
        // Each applied merge replaces two tokens with one wherever the pair
        // occurs, so the exact number of tokens removed is the total shrinkage
        // in word length weighted by how many documents each word represents.
        let mut tokens_removed = 0i64;
        for &word_idx in &top.positions {
            let word_count = counts[word_idx];
            let len_before = words[word_idx].ids.len();
            let deltas = words[word_idx].merge_pair(top.pair, new_token);
            tokens_removed += (len_before - words[word_idx].ids.len()) as i64 * word_count;

            for (delta_pair, delta) in deltas {
                let delta_total = delta * word_count;
                if delta_total != 0 {
                    *pair_counts.entry(delta_pair).or_default() += delta_total;
                    if delta > 0 {
                        new_positions
                            .entry(delta_pair)
                            .or_default()
                            .insert(word_idx);
                    }
                }
            }
        }

        for (pair, positions) in new_positions {
            let count = *pair_counts.get(&pair).unwrap_or(&0);
            if count > 0 {
                heap.push(MergeJob {
                    pair,
                    count: count as u64,
                    positions,
                });
            }
        }

        total_tokens -= tokens_removed;
        merges_done += 1;
        let current_average = total_tokens as f64 / num_documents as f64;

        if let Some(target) = options.target_avg_length
            && current_average <= target
        {
            // This merge brought the average from above the target to at or
            // below it. Keep whichever side of the crossing is closer to the
            // target: if the pre-merge average was nearer, drop this last merge.
            let stop_average =
                if (previous_average - target).abs() < (current_average - target).abs() {
                    merges.pop();
                    merges_done -= 1;
                    previous_average
                } else {
                    current_average
                };
            progress(BpeTrainingProgress::TargetAverageReached {
                current_average: stop_average,
                target,
                merges_done,
            });
            break;
        }
        previous_average = current_average;

        let should_log = if options.verbose {
            true
        } else if options.target_avg_length.is_some() {
            merges_done.is_multiple_of(100)
        } else {
            let current_percent = (merges_done as usize * 100) / max_merges;
            if current_percent as u32 > last_log_percent {
                last_log_percent = current_percent as u32;
                true
            } else {
                false
            }
        };

        if should_log {
            progress(BpeTrainingProgress::Merge {
                merges_done,
                current_average,
                vocab_size: bpe_start + merges_done,
                pair: top.pair,
                new_token,
                frequency: top.count,
            });
        }
    }

    if merges.len() == max_merges {
        progress(BpeTrainingProgress::Completed {
            merges: merges.len(),
        });
    }

    merges
}

/// Apply a single merge to a sequence (used during training simulation)
///
/// Scans the sequence and replaces all non-overlapping occurrences of the pair
/// with the merged token. This matches how encoding will work at runtime.
///
/// # Arguments
/// * `sequence` - The token sequence to modify in-place
/// * `pair` - The pair of tokens to merge
/// * `new_token` - The token ID to replace the pair with
pub fn apply_merge_to_sequence(sequence: &mut Vec<u32>, pair: (u32, u32), new_token: u32) {
    let mut i = 0;
    let mut result = Vec::with_capacity(sequence.len());

    while i < sequence.len() {
        // Check if current position matches the pair
        if i + 1 < sequence.len() && sequence[i] == pair.0 && sequence[i + 1] == pair.1 {
            // Apply merge: replace pair with new token
            result.push(new_token);
            i += 2; // Skip both tokens of the pair
        } else {
            // Keep original token
            result.push(sequence[i]);
            i += 1;
        }
    }

    *sequence = result;
}

/// Build a complete BPE merge table from training data
///
/// Convenience function that wraps training with metadata generation.
pub fn build_merge_table(
    sequences: Vec<Vec<u32>>,
    max_merges: usize,
    base_vocab_size: u32,
    schema_name: String,
) -> BpeMergeTable {
    let num_training_instances = sequences.len();
    let merges = train_bpe_incremental(sequences, max_merges, base_vocab_size);

    BpeMergeTable {
        schema: schema_name,
        base_vocab_size,
        merges,
        metadata: BpeMetadata {
            trained_on_instances: num_training_instances,
            training_date: chrono::Utc::now().format("%Y-%m-%d").to_string(),
            max_merges,
            dict_size: None,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_apply_merge_to_sequence_basic() {
        let mut seq = vec![1, 2, 3, 2, 3, 4];
        apply_merge_to_sequence(&mut seq, (2, 3), 100);
        assert_eq!(seq, vec![1, 100, 100, 4]);
    }

    #[test]
    fn test_apply_merge_to_sequence_no_match() {
        let mut seq = vec![1, 2, 3, 4];
        apply_merge_to_sequence(&mut seq, (5, 6), 100);
        assert_eq!(seq, vec![1, 2, 3, 4]); // Unchanged
    }

    #[test]
    fn test_apply_merge_to_sequence_at_start() {
        let mut seq = vec![2, 3, 1];
        apply_merge_to_sequence(&mut seq, (2, 3), 100);
        assert_eq!(seq, vec![100, 1]);
    }

    #[test]
    fn test_apply_merge_to_sequence_at_end() {
        let mut seq = vec![1, 2, 3];
        apply_merge_to_sequence(&mut seq, (2, 3), 100);
        assert_eq!(seq, vec![1, 100]);
    }

    #[test]
    fn test_apply_merge_to_sequence_consecutive_pairs() {
        let mut seq = vec![2, 3, 2, 3];
        apply_merge_to_sequence(&mut seq, (2, 3), 100);
        assert_eq!(seq, vec![100, 100]);
    }

    #[test]
    fn test_apply_merge_to_sequence_overlapping() {
        // Test that we handle overlapping correctly (greedy left-to-right)
        let mut seq = vec![1, 2, 2, 3];
        apply_merge_to_sequence(&mut seq, (2, 2), 100);
        assert_eq!(seq, vec![1, 100, 3]); // Merges (2,2), leaves one 2
    }

    #[test]
    fn test_apply_merge_to_sequence_empty() {
        let mut seq: Vec<u32> = vec![];
        apply_merge_to_sequence(&mut seq, (1, 2), 100);
        assert_eq!(seq, Vec::<u32>::new()); // Unchanged
    }

    #[test]
    fn test_apply_merge_to_sequence_single_token() {
        let mut seq = vec![1];
        apply_merge_to_sequence(&mut seq, (1, 2), 100);
        assert_eq!(seq, vec![1]); // Unchanged
    }

    #[test]
    fn test_train_bpe_incremental_basic() {
        // Create simple sequences with clear pattern
        let sequences = vec![
            vec![1, 2, 3], // (1,2) and (2,3)
            vec![1, 2, 4], // (1,2) and (2,4)
            vec![1, 2, 5], // (1,2) and (2,5)
        ];

        // Train with max 1 merge
        let merges = train_bpe_incremental(sequences, 1, 256);

        // Should merge (1,2) as it appears 3 times
        assert_eq!(merges.len(), 1);
        assert_eq!(merges[0].pair, [1, 2]);
        assert_eq!(merges[0].new_token, 256); // First BPE token
        assert_eq!(merges[0].frequency, 3);
    }

    #[test]
    fn test_train_bpe_incremental_simulation() {
        // Test that incremental simulation affects subsequent merges
        // Use sequences where (1,2) clearly has highest frequency
        let sequences = vec![
            vec![1, 2, 1, 2, 3], // (1,2) appears twice, (2,1), (2,3) appear once
            vec![1, 2, 1, 2, 3],
        ];

        let merges = train_bpe_incremental(sequences, 2, 256);

        // First merge should be (1,2) with frequency 4 (highest)
        assert_eq!(merges.len(), 2);
        assert_eq!(merges[0].pair, [1, 2]);
        assert_eq!(merges[0].new_token, 256);
        assert_eq!(merges[0].frequency, 4);

        // After first merge (1,2)->256, sequences become [256, 256, 3]
        // All new pairs involve BPE token 256: (256,256) and (256,3)
        // Therefore, the second merge MUST involve a BPE token
        let second_merge_has_bpe_token = merges[1].pair[0] >= 256 || merges[1].pair[1] >= 256;

        assert!(
            second_merge_has_bpe_token,
            "Second merge should involve BPE token from first merge, demonstrating incremental simulation"
        );
    }

    #[test]
    fn test_train_bpe_incremental_empty_sequences() {
        let sequences: Vec<Vec<u32>> = vec![];
        let merges = train_bpe_incremental(sequences, 10, 256);
        assert_eq!(merges.len(), 0);
    }

    #[test]
    fn test_train_bpe_incremental_no_pairs() {
        // All sequences have single tokens
        let sequences = vec![vec![1], vec![2], vec![3]];
        let merges = train_bpe_incremental(sequences, 10, 256);
        assert_eq!(merges.len(), 0);
    }

    #[test]
    fn test_train_bpe_incremental_max_merges_zero() {
        let sequences = vec![vec![1, 2, 3]];
        let merges = train_bpe_incremental(sequences, 0, 256);
        assert_eq!(merges.len(), 0);
    }

    #[test]
    fn test_train_bpe_incremental_with_wide_base_vocab() {
        let sequences = vec![vec![1, 2, 3], vec![1, 2, 4]];

        let merges = train_bpe_incremental(sequences, 1, 1000);

        assert_eq!(merges[0].new_token, 1000);
    }

    #[test]
    fn test_train_bpe_incremental_starts_after_base_vocab() {
        let sequences = vec![vec![1, 2, 3], vec![1, 2, 4]];

        let merges = train_bpe_incremental(sequences, 1, 256);

        assert_eq!(merges[0].new_token, 256);
    }

    #[test]
    fn test_train_bpe_incremental_frequency_ordering() {
        // Create sequences where one pair is clearly most frequent
        let sequences = vec![
            vec![1, 2, 1, 2, 1, 2], // (1,2) appears 3 times
            vec![3, 4],             // (3,4) appears 1 time
            vec![5, 6],             // (5,6) appears 1 time
        ];

        let merges = train_bpe_incremental(sequences, 1, 256);

        // Should select (1,2) as it has highest frequency
        assert_eq!(merges[0].pair, [1, 2]);
        assert_eq!(merges[0].frequency, 3);
    }

    #[test]
    fn test_train_bpe_incremental_uses_shared_optimized_default() {
        let sequences = vec![vec![1, 2, 1, 2, 3], vec![1, 2, 1, 2, 3], vec![4, 5, 4]];

        let public = train_bpe_incremental(sequences.clone(), 3, 256);
        let shared = train_bpe_incremental_with_options(
            sequences,
            3,
            256,
            BpeTrainingOptions::default(),
            |_| {},
        );

        assert_eq!(public, shared);
        assert!(
            shared
                .iter()
                .skip(1)
                .any(|merge| merge.pair[0] >= 256 || merge.pair[1] >= 256),
            "optimized incremental training should keep later merges over prior BPE tokens"
        );
    }

    #[test]
    fn test_train_bpe_incremental_with_options_reports_progress() {
        let sequences = vec![vec![1, 2, 1, 2], vec![1, 2, 1, 2]];
        let mut events = Vec::new();

        let merges = train_bpe_incremental_with_options(
            sequences,
            2,
            256,
            BpeTrainingOptions {
                target_avg_length: None,
                verbose: true,
                structure_only_roles: None,
            },
            |event| events.push(event),
        );

        assert_eq!(merges.len(), 2);
        assert!(events.iter().any(|event| matches!(
            event,
            BpeTrainingProgress::Deduplicated {
                unique_sequences: 1
            }
        )));
        assert!(events.iter().any(|event| matches!(
            event,
            BpeTrainingProgress::Merge {
                pair: (1, 2),
                frequency: 4,
                ..
            }
        )));
        assert!(
            events
                .iter()
                .any(|event| matches!(event, BpeTrainingProgress::Completed { merges: 2 }))
        );
    }

    #[test]
    fn test_build_merge_table() {
        let sequences = vec![vec![1, 2, 3], vec![1, 2, 4]];
        let table = build_merge_table(sequences, 1, 256, "test-schema".to_string());

        assert_eq!(table.schema, "test-schema");
        assert_eq!(table.base_vocab_size, 256);
        assert_eq!(table.merges.len(), 1);
        assert_eq!(table.metadata.trained_on_instances, 2);
        assert_eq!(table.metadata.max_merges, 1);
        assert_eq!(table.metadata.dict_size, None);
    }

    #[test]
    fn test_incremental_vs_hypothetical_static() {
        // This test demonstrates WHY incremental simulation is correct.
        //
        // Consider sequences: [[1,2,2,3], [1,2,2,3]]
        //
        // STATIC (WRONG) approach would count all pairs once:
        //   (1,2): 2 occurrences
        //   (2,2): 2 occurrences
        //   (2,3): 2 occurrences
        // Then pick top N pairs. If we pick (1,2) and (2,2), we'd get:
        //   Merge 1: (1,2) -> 100
        //   Merge 2: (2,2) -> 101
        //
        // INCREMENTAL (CORRECT) approach:
        //   Round 1: All pairs have freq 2, pick one (say (2,2))
        //   Apply (2,2)->100 to all sequences: [[1,2,100,3], [1,2,100,3]]
        //   Round 2: Recalculate frequencies:
        //     (1,2): 2 occurrences
        //     (2,100): 2 occurrences (NEW PAIR!)
        //     (100,3): 2 occurrences (NEW PAIR!)
        //   Pick one (say (1,2))
        //
        // The key difference: After merging (2,2), the pairs (2,100) and (100,3)
        // become available, which wouldn't exist in static counting!

        let sequences = vec![vec![1, 2, 2, 3], vec![1, 2, 2, 3]];
        let merges = train_bpe_incremental(sequences.clone(), 3, 256);

        // Verify we got 3 merges
        assert_eq!(merges.len(), 3);

        // Verify that after first merge, we're working with modified sequences
        // (This is tested implicitly by the fact that we can generate 3 merges
        //  from sequences that initially only have 3 unique pairs)

        // The third merge should involve a token ID >= 256 (a BPE-merged token)
        // because after 2 merges, the sequences have changed
        let has_bpe_token_in_pair = merges
            .iter()
            .skip(1)
            .any(|m| m.pair[0] >= 256 || m.pair[1] >= 256);
        assert!(
            has_bpe_token_in_pair,
            "Later merges should involve BPE tokens from earlier merges"
        );
    }

    /// Train a single-document corpus toward a target average length and report
    /// how many merges were kept plus the average announced via
    /// `TargetAverageReached` (if any).
    ///
    /// The corpus `[1,2,3,1,2,3]` (one document) produces a fully deterministic
    /// average-length progression as merges are applied:
    ///   0 merges -> 6.0, 1 -> 4.0, 2 -> 2.0, 3 -> 1.0 (then no pairs remain).
    fn train_to_target(target: f64) -> (usize, Option<f64>) {
        let sequences = vec![vec![1u32, 2, 3, 1, 2, 3]];
        let mut reached = None;
        let merges = train_bpe_incremental_with_options(
            sequences,
            10,
            256,
            BpeTrainingOptions {
                target_avg_length: Some(target),
                verbose: false,
                structure_only_roles: None,
            },
            |event| {
                if let BpeTrainingProgress::TargetAverageReached {
                    current_average, ..
                } = event
                {
                    reached = Some(current_average);
                }
            },
        );
        (merges.len(), reached)
    }

    #[test]
    fn test_structure_only_merges_skip_value_pairs() {
        // Structure pair (10, 11) appears once; value pair (1, 2) appears thrice.
        // Unconstrained training would merge (1, 2). Structure-only must merge
        // only (10, 11) and must never emit a merge whose either side is a
        // value-position token id from the value span.
        let sequences = vec![vec![10, 11, 1, 2], vec![1, 2, 1, 2], vec![1, 2]];
        let roles = vec![
            vec![ROLE_STRUCTURE, ROLE_STRUCTURE, ROLE_VALUE, ROLE_VALUE],
            vec![ROLE_VALUE, ROLE_VALUE, ROLE_VALUE, ROLE_VALUE],
            vec![ROLE_VALUE, ROLE_VALUE],
        ];

        let unconstrained = train_bpe_incremental(sequences.clone(), 1, 256);
        assert_eq!(unconstrained[0].pair, [1, 2]);

        let structure_only = train_bpe_incremental_with_options(
            sequences,
            3,
            256,
            BpeTrainingOptions {
                target_avg_length: None,
                verbose: false,
                structure_only_roles: Some(roles),
            },
            |_| {},
        );

        assert_eq!(structure_only.len(), 1);
        assert_eq!(structure_only[0].pair, [10, 11]);
        assert_eq!(structure_only[0].frequency, 1);
        for merge in &structure_only {
            assert!(
                merge.pair[0] != 1
                    && merge.pair[1] != 1
                    && merge.pair[0] != 2
                    && merge.pair[1] != 2,
                "structure-only merge must not span value byte tokens: {:?}",
                merge.pair
            );
        }
    }

    #[test]
    fn test_structure_only_dedupes_duplicate_sequences_with_first_roles() {
        // Identical token sequences with identical roles collapse to one entry;
        // first-seen roles stay co-located so structure-only merge selection is
        // unchanged by pure frequency duplication.
        let sequences = vec![vec![10, 11, 1], vec![10, 11, 1], vec![10, 11, 1]];
        let roles = vec![
            vec![ROLE_STRUCTURE, ROLE_STRUCTURE, ROLE_VALUE],
            vec![ROLE_STRUCTURE, ROLE_STRUCTURE, ROLE_VALUE],
            vec![ROLE_STRUCTURE, ROLE_STRUCTURE, ROLE_VALUE],
        ];
        let merges = train_bpe_incremental_with_options(
            sequences,
            2,
            256,
            BpeTrainingOptions {
                structure_only_roles: Some(roles),
                ..Default::default()
            },
            |_| {},
        );
        assert_eq!(merges.len(), 1);
        assert_eq!(merges[0].pair, [10, 11]);
        assert_eq!(merges[0].frequency, 3);
    }

    #[test]
    fn test_structure_only_allows_discriminator_merges() {
        // Token ids are independent of role labels. Discriminator token 7 may
        // merge with structure; value token 3 may not. (20,20) is made more
        // frequent so it is selected first under equal-count heap tie-breaks.
        let sequences = vec![vec![7, 20, 20, 3], vec![20, 20, 3], vec![20, 20]];
        let roles = vec![
            vec![
                ROLE_DISCRIMINATOR,
                ROLE_STRUCTURE,
                ROLE_STRUCTURE,
                ROLE_VALUE,
            ],
            vec![ROLE_STRUCTURE, ROLE_STRUCTURE, ROLE_VALUE],
            vec![ROLE_STRUCTURE, ROLE_STRUCTURE],
        ];
        let merges = train_bpe_incremental_with_options(
            sequences,
            2,
            256,
            BpeTrainingOptions {
                structure_only_roles: Some(roles),
                ..Default::default()
            },
            |_| {},
        );
        assert_eq!(merges.len(), 2);
        assert_eq!(merges[0].pair, [20, 20]);
        // After (20,20)->256, doc0 becomes [7, 256, 3]; (7,256) is mergeable.
        assert_eq!(merges[1].pair, [7, 256]);
    }

    #[test]
    fn test_target_avg_stops_at_closest_merge_above() {
        // Target 3.5 sits between avg 4.0 (1 merge) and 2.0 (2 merges); 4.0 is
        // closer, so the trainer must drop back to a single merge.
        let (merges, reached) = train_to_target(3.5);
        assert_eq!(merges, 1);
        assert_eq!(reached, Some(4.0));
    }

    #[test]
    fn test_target_avg_stops_at_closest_merge_below() {
        // Target 2.5 is nearer to 2.0 (2 merges) than 4.0 (1 merge); the trailing
        // merge that crosses the target is kept.
        let (merges, reached) = train_to_target(2.5);
        assert_eq!(merges, 2);
        assert_eq!(reached, Some(2.0));
    }

    #[test]
    fn test_target_avg_rolls_back_when_previous_is_closer() {
        // Target 1.6 is closer to 2.0 (2 merges) than 1.0 (3 merges): the merge
        // crossing below the target is rolled back.
        let (merges, reached) = train_to_target(1.6);
        assert_eq!(merges, 2);
        assert_eq!(reached, Some(2.0));
    }

    #[test]
    fn test_target_avg_keeps_crossing_merge_when_closer() {
        // Target 1.4 is closer to 1.0 (3 merges) than 2.0 (2 merges).
        let (merges, reached) = train_to_target(1.4);
        assert_eq!(merges, 3);
        assert_eq!(reached, Some(1.0));
    }

    #[test]
    fn test_target_avg_already_satisfied_produces_no_merges() {
        // Initial average is exactly 6.0; a target at or above that needs no
        // merges at all.
        let (merges, reached) = train_to_target(6.0);
        assert_eq!(merges, 0);
        assert_eq!(reached, Some(6.0));

        let (merges, reached) = train_to_target(10.0);
        assert_eq!(merges, 0);
        assert_eq!(reached, Some(6.0));
    }

    #[test]
    fn test_target_avg_unreachable_trains_all_merges() {
        // Target 0.5 can never be reached (the minimum average is 1.0), so the
        // trainer exhausts the available merges without announcing the target.
        let (merges, reached) = train_to_target(0.5);
        assert_eq!(merges, 3);
        assert_eq!(reached, None);
    }
}
