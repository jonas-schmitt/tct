//! Independent reference-oracle checks for the optimized BPE trainer.

use std::collections::BTreeMap;
use tct::bpe::{BpeMerge, bpe_start_token, trainer::train_bpe_incremental};

type Pair = (u32, u32);

fn reference_apply_merge(sequence: &[u32], pair: Pair, new_token: u32) -> Vec<u32> {
    let mut merged = Vec::with_capacity(sequence.len());
    let mut index = 0usize;
    while index < sequence.len() {
        if index + 1 < sequence.len() && sequence[index] == pair.0 && sequence[index + 1] == pair.1
        {
            merged.push(new_token);
            index += 2;
        } else {
            merged.push(sequence[index]);
            index += 1;
        }
    }
    merged
}

fn reference_train_bpe_full_recount(
    mut sequences: Vec<Vec<u32>>,
    max_merges: usize,
    base_vocab_size: u32,
) -> Vec<BpeMerge> {
    let mut merges = Vec::new();
    let bpe_start = bpe_start_token(base_vocab_size);

    for merge_index in 0..max_merges {
        let mut pair_counts: BTreeMap<Pair, u64> = BTreeMap::new();
        for sequence in &sequences {
            for window in sequence.windows(2) {
                *pair_counts.entry((window[0], window[1])).or_default() += 1;
            }
        }

        let Some((pair, frequency)) =
            pair_counts
                .into_iter()
                .max_by(|(left_pair, left_count), (right_pair, right_count)| {
                    left_count
                        .cmp(right_count)
                        .then_with(|| right_pair.cmp(left_pair))
                })
        else {
            break;
        };

        let new_token = bpe_start + merge_index as u32;
        merges.push(BpeMerge {
            pair: [pair.0, pair.1],
            new_token,
            frequency,
        });

        for sequence in &mut sequences {
            *sequence = reference_apply_merge(sequence, pair, new_token);
        }
    }

    merges
}

fn train_with_thread_count(
    sequences: Vec<Vec<u32>>,
    max_merges: usize,
    base_vocab_size: u32,
    threads: usize,
) -> Vec<BpeMerge> {
    rayon::ThreadPoolBuilder::new()
        .num_threads(threads)
        .build()
        .unwrap()
        .install(|| train_bpe_incremental(sequences, max_merges, base_vocab_size))
}

fn tie_heavy_determinism_corpus() -> Vec<Vec<u32>> {
    vec![
        vec![1, 2, 3, 1, 2, 3],
        vec![3, 1, 2, 3, 1, 2],
        vec![4, 5, 4, 5],
        vec![5, 4, 5, 4],
        vec![7, 8, 7, 9],
        vec![7, 9, 7, 8],
        vec![10, 11, 10, 11],
        vec![11, 10, 11, 10],
        vec![300, 301, 300],
        vec![301, 300, 301],
        vec![1, 2, 3, 1, 2, 3],
        vec![7, 8, 7, 9],
    ]
}

#[test]
fn optimized_bpe_trainer_matches_full_recount_reference_oracle() {
    let cases = [
        (
            "generated-token tie break",
            vec![vec![1, 2, 1, 2, 3], vec![1, 2, 1, 2, 3]],
            3,
            256,
        ),
        (
            "tie-heavy overlapping corpus",
            vec![
                vec![1, 2, 1, 2, 1],
                vec![1, 2, 2, 1, 2],
                vec![2, 1, 2, 1],
                vec![1, 2, 1, 2, 1],
                vec![4, 5, 4, 5, 4, 5],
                vec![4, 5, 4, 5, 4, 5],
            ],
            8,
            256,
        ),
        (
            "wide base vocab with duplicate weighted rows",
            vec![
                vec![7, 8, 7, 8, 9],
                vec![7, 8, 7, 8, 9],
                vec![8, 9, 8, 9],
                vec![9, 7, 9, 7],
                vec![10, 11, 10],
            ],
            7,
            512,
        ),
        (
            "max merges exceeds available pairs",
            vec![vec![1, 1, 1, 1], vec![2], vec![3, 4]],
            10,
            300,
        ),
    ];

    for (name, sequences, max_merges, base_vocab_size) in cases {
        let optimized = train_bpe_incremental(sequences.clone(), max_merges, base_vocab_size);
        let reference = reference_train_bpe_full_recount(sequences, max_merges, base_vocab_size);
        assert_eq!(optimized, reference, "{name}");
    }
}

#[test]
fn optimized_bpe_trainer_is_stable_across_input_order_and_thread_counts() {
    let corpus = tie_heavy_determinism_corpus();
    let max_merges = 10;
    let base_vocab_size = 512;
    let expected = reference_train_bpe_full_recount(corpus.clone(), max_merges, base_vocab_size);

    let mut reversed = corpus.clone();
    reversed.reverse();
    let mut rotated = corpus.clone();
    rotated.rotate_left(5);
    let mut sorted = corpus.clone();
    sorted.sort();

    for variant in [corpus.clone(), reversed, rotated, sorted] {
        for threads in [1, 2, 4] {
            let trained =
                train_with_thread_count(variant.clone(), max_merges, base_vocab_size, threads);
            assert_eq!(trained, expected, "thread count {threads}");
        }
    }
}

#[test]
fn optimized_bpe_trainer_is_stable_across_repeated_tie_heavy_runs() {
    let corpus = tie_heavy_determinism_corpus();
    let max_merges = 10;
    let base_vocab_size = 512;
    let expected = reference_train_bpe_full_recount(corpus.clone(), max_merges, base_vocab_size);

    for round in 0..32 {
        let mut variant = corpus.clone();
        let rotation = round % variant.len();
        variant.rotate_left(rotation);
        if round % 2 == 1 {
            variant.reverse();
        }

        let trained = train_bpe_incremental(variant, max_merges, base_vocab_size);
        assert_eq!(trained, expected, "round {round}");
    }
}

#[test]
fn optimized_bpe_trainer_keeps_expected_incremental_tie_break() {
    let merges = train_bpe_incremental(vec![vec![1, 2, 1, 2, 3]; 2], 3, 256);

    assert_eq!(
        merges,
        vec![
            BpeMerge {
                pair: [1, 2],
                new_token: 256,
                frequency: 4,
            },
            BpeMerge {
                pair: [256, 3],
                new_token: 257,
                frequency: 2,
            },
            BpeMerge {
                pair: [256, 257],
                new_token: 258,
                frequency: 2,
            },
        ]
    );
}
