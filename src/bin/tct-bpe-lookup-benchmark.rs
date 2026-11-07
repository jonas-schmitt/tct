//! Compare BPE pair-lookup strategies over a shared encoded token corpus.

use clap::{Arg, Command, value_parser};
use rustc_hash::FxHashMap;
use serde::Serialize;
use std::error::Error;
use std::hint::black_box;
use std::path::Path;
use std::time::Instant;
use tct::bpe::{BpeMergeTable, encoder::SequentialBpeEncoder};
use tct::provenance::file_provenance;

const NO_LINK: u32 = u32::MAX;

#[derive(Debug, Serialize)]
struct BenchmarkReport {
    command: Vec<String>,
    bpe_merges_file: tct::provenance::FileProvenance,
    encoded_input_file: tct::provenance::FileProvenance,
    input_sequences: usize,
    bpe_start: u32,
    pad_token: u32,
    max_vocab_size: u32,
    max_vocab_size_interpretation: &'static str,
    lookup_table_entries: usize,
    active_merges: usize,
    initial_pair_queries: usize,
    post_merge_pair_queries: usize,
    applied_merges: usize,
    pair_queries: usize,
    runtime_encoded_sequences: usize,
    runtime_encoded_tokens: usize,
    runtime_encoded_token_checksum: u64,
    warmup_iterations: u32,
    iterations: u32,
    strategies: Vec<StrategyReport>,
    all_strategies_agree: bool,
    all_strategy_outputs_match_runtime: bool,
}

#[derive(Debug, Serialize)]
struct StrategyReport {
    name: &'static str,
    build_time_ns: u64,
    lookup_time_ns: Vec<u64>,
    best_lookup_time_ns: u64,
    hits: usize,
    checksum: u64,
    encoded_sequences: usize,
    encoded_tokens: usize,
    encoded_token_checksum: u64,
    matches_runtime: bool,
}

enum LookupTable {
    Sorted(Vec<(u64, usize)>),
    Hash(FxHashMap<(u32, u32), usize>),
    Bucketed(FxHashMap<u32, Vec<(u32, usize)>>),
}

#[derive(Clone, Copy, Debug)]
struct MergeEntry {
    pair: (u32, u32),
    index: usize,
    new_token: u32,
}

#[derive(Debug, Default)]
struct QueryProfile {
    queries: Vec<(u32, u32)>,
    initial_pair_queries: usize,
    post_merge_pair_queries: usize,
    applied_merges: usize,
}

struct BenchmarkInputs<'a> {
    merges: &'a [MergeEntry],
    max_merges: usize,
    queries: &'a [(u32, u32)],
    token_sequences: &'a [Vec<u32>],
    runtime_encoded_sequences: &'a [Vec<u32>],
}

#[derive(Clone, Debug)]
enum PositionBucket {
    One(u32),
    Many(Vec<u32>),
}

struct ReplayState<'a> {
    current: &'a mut [u32],
    next: &'a mut [u32],
    prev: &'a mut [u32],
    alive: &'a mut [bool],
    future_positions: &'a mut FxHashMap<u32, PositionBucket>,
    position_pool: &'a mut Vec<Vec<u32>>,
    max_merges: usize,
    lookup_table: &'a LookupTable,
    profile: Option<&'a mut QueryProfile>,
}

impl LookupTable {
    fn build(name: &'static str, merges: &[MergeEntry]) -> Result<Self, Box<dyn Error>> {
        match name {
            "sorted_binary_search" => Ok(Self::sorted_binary_search(merges)),
            "hash_map" => {
                let mut table =
                    FxHashMap::with_capacity_and_hasher(merges.len(), Default::default());
                for merge in merges {
                    table.insert(merge.pair, merge.index);
                }
                Ok(Self::Hash(table))
            }
            "bucketed_by_left" => {
                let mut table: FxHashMap<u32, Vec<(u32, usize)>> =
                    FxHashMap::with_capacity_and_hasher(merges.len(), Default::default());
                for merge in merges {
                    table
                        .entry(merge.pair.0)
                        .or_default()
                        .push((merge.pair.1, merge.index));
                }
                for bucket in table.values_mut() {
                    bucket.sort_unstable_by_key(|(right, _)| *right);
                }
                Ok(Self::Bucketed(table))
            }
            _ => Err(format!("unknown lookup strategy: {name}").into()),
        }
    }

    fn sorted_binary_search(merges: &[MergeEntry]) -> Self {
        let mut table: Vec<(u64, usize)> = merges
            .iter()
            .map(|merge| (pair_key(merge.pair), merge.index))
            .collect();
        table.sort_unstable_by_key(|(key, _)| *key);
        Self::Sorted(table)
    }

    #[inline]
    fn lookup(&self, pair: (u32, u32)) -> Option<usize> {
        match self {
            Self::Sorted(table) => {
                let key = pair_key(pair);
                table
                    .binary_search_by_key(&key, |(candidate, _)| *candidate)
                    .ok()
                    .map(|index| table[index].1)
            }
            Self::Hash(table) => table.get(&pair).copied(),
            Self::Bucketed(table) => table.get(&pair.0).and_then(|bucket| {
                bucket
                    .binary_search_by_key(&pair.1, |(right, _)| *right)
                    .ok()
                    .map(|index| bucket[index].1)
            }),
        }
    }

    #[inline]
    fn lookup_capped(&self, pair: (u32, u32), max_merges: usize) -> Option<usize> {
        self.lookup(pair)
            .and_then(|merge_idx| (merge_idx < max_merges).then_some(merge_idx))
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    let command: Vec<String> = std::env::args().collect();
    let matches = Command::new("tct-bpe-lookup-benchmark")
        .about("Compare BPE pair lookup strategies over encoded token sequences")
        .arg(
            Arg::new("bpe-merges")
                .long("bpe-merges")
                .required(true)
                .help("BPE merge table JSON")
                .value_parser(value_parser!(String)),
        )
        .arg(
            Arg::new("encoded")
                .long("encoded")
                .required(true)
                .help("JSONL file containing encoded base token arrays")
                .value_parser(value_parser!(String)),
        )
        .arg(
            Arg::new("max-vocab-size")
                .long("max-vocab-size")
                .help("Vocabulary cap to benchmark; defaults to base + all merges + PAD")
                .value_parser(value_parser!(u32)),
        )
        .arg(
            Arg::new("iterations")
                .long("iterations")
                .help("Measured lookup iterations")
                .value_parser(value_parser!(u32))
                .default_value("5"),
        )
        .arg(
            Arg::new("warmup-iterations")
                .long("warmup-iterations")
                .help("Unmeasured warmup iterations per strategy")
                .value_parser(value_parser!(u32))
                .default_value("1"),
        )
        .get_matches();

    let bpe_merges = required_arg::<String>(&matches, "bpe-merges")?;
    let encoded = required_arg::<String>(&matches, "encoded")?;
    let table = BpeMergeTable::from_file(bpe_merges)?;
    let max_vocab_size = matches
        .get_one::<u32>("max-vocab-size")
        .copied()
        .unwrap_or_else(|| table.bpe_start() + table.merges.len() as u32 + 1);
    let iterations = *required_arg::<u32>(&matches, "iterations")?;
    let warmup_iterations = *required_arg::<u32>(&matches, "warmup-iterations")?;
    if iterations == 0 {
        return Err("--iterations must be greater than zero".into());
    }

    let token_sequences = read_token_sequences(Path::new(encoded))?;
    let all_merges = merge_pairs(&table);
    let active_merges = active_merge_count(&table, max_vocab_size);
    let query_profile =
        collect_generated_encoder_lookup_queries(&token_sequences, &all_merges, active_merges);
    if query_profile.queries.is_empty() {
        return Err("encoded input did not contain any generated BPE lookup queries".into());
    }
    let runtime_encoder = SequentialBpeEncoder::new(&table, max_vocab_size);
    let runtime_encoded_sequences = token_sequences
        .iter()
        .map(|tokens| runtime_encoder.encode(tokens))
        .collect::<Vec<_>>();
    let (runtime_encoded_sequences_count, runtime_encoded_tokens, runtime_encoded_token_checksum) =
        encoded_sequences_summary(&runtime_encoded_sequences);

    let strategy_names = ["sorted_binary_search", "hash_map", "bucketed_by_left"];
    let benchmark_inputs = BenchmarkInputs {
        merges: &all_merges,
        max_merges: active_merges,
        queries: &query_profile.queries,
        token_sequences: &token_sequences,
        runtime_encoded_sequences: &runtime_encoded_sequences,
    };
    let mut reports = Vec::new();
    for name in strategy_names {
        reports.push(benchmark_strategy(
            name,
            &benchmark_inputs,
            warmup_iterations,
            iterations,
        )?);
    }

    let first = &reports[0];
    let all_strategies_agree = reports
        .iter()
        .all(|report| report.hits == first.hits && report.checksum == first.checksum);
    let all_strategy_outputs_match_runtime = reports.iter().all(|report| {
        report.matches_runtime
            && report.encoded_sequences == runtime_encoded_sequences_count
            && report.encoded_tokens == runtime_encoded_tokens
            && report.encoded_token_checksum == runtime_encoded_token_checksum
    });

    let report = BenchmarkReport {
        command,
        bpe_merges_file: file_provenance(bpe_merges)?,
        encoded_input_file: file_provenance(encoded)?,
        input_sequences: token_sequences.len(),
        bpe_start: table.bpe_start(),
        pad_token: max_vocab_size.saturating_sub(1),
        max_vocab_size,
        max_vocab_size_interpretation: "exclusive vocabulary bound passed to generated BPE helpers; normal generated runs pass VOCAB_SIZE, whose last raw slot is PAD_TOKEN; active merges are min(max_vocab_size - bpe_start, merge_count)",
        lookup_table_entries: all_merges.len(),
        active_merges,
        initial_pair_queries: query_profile.initial_pair_queries,
        post_merge_pair_queries: query_profile.post_merge_pair_queries,
        applied_merges: query_profile.applied_merges,
        pair_queries: query_profile.queries.len(),
        runtime_encoded_sequences: runtime_encoded_sequences_count,
        runtime_encoded_tokens,
        runtime_encoded_token_checksum,
        warmup_iterations,
        iterations,
        strategies: reports,
        all_strategies_agree,
        all_strategy_outputs_match_runtime,
    };
    println!("{}", serde_json::to_string_pretty(&report)?);
    Ok(())
}

fn required_arg<'a, T: Clone + Send + Sync + 'static>(
    matches: &'a clap::ArgMatches,
    name: &str,
) -> Result<&'a T, Box<dyn Error>> {
    matches
        .get_one::<T>(name)
        .ok_or_else(|| format!("missing required argument: {name}").into())
}

fn active_merge_count(table: &BpeMergeTable, max_vocab_size: u32) -> usize {
    let bpe_start = table.bpe_start();
    if max_vocab_size > bpe_start {
        ((max_vocab_size - bpe_start) as usize).min(table.merges.len())
    } else {
        0
    }
}

fn merge_pairs(table: &BpeMergeTable) -> Vec<MergeEntry> {
    table
        .merges
        .iter()
        .enumerate()
        .map(|(index, merge)| MergeEntry {
            pair: (merge.pair[0], merge.pair[1]),
            index,
            new_token: merge.new_token,
        })
        .collect()
}

fn read_token_sequences(path: &Path) -> Result<Vec<Vec<u32>>, Box<dyn Error>> {
    let content = std::fs::read_to_string(path)?;
    let mut sequences = Vec::new();
    for (line_index, line) in content.lines().enumerate() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        let tokens: Vec<u32> = serde_json::from_str(trimmed)
            .map_err(|error| format!("{}:{}: {error}", path.display(), line_index + 1))?;
        sequences.push(tokens);
    }
    Ok(sequences)
}

fn collect_generated_encoder_lookup_queries(
    token_sequences: &[Vec<u32>],
    merges: &[MergeEntry],
    max_merges: usize,
) -> QueryProfile {
    let lookup_table = LookupTable::sorted_binary_search(merges);
    let mut profile = QueryProfile::default();
    for tokens in token_sequences {
        let _ = replay_sequence_encode(
            tokens,
            merges,
            max_merges,
            &lookup_table,
            Some(&mut profile),
        );
    }
    profile
}

fn replay_sequence_encode(
    tokens: &[u32],
    merges: &[MergeEntry],
    max_merges: usize,
    lookup_table: &LookupTable,
    profile: Option<&mut QueryProfile>,
) -> Vec<u32> {
    if tokens.len() < 2 || max_merges == 0 {
        return tokens.to_vec();
    }
    if tokens.len() > u32::MAX as usize || max_merges > u32::MAX as usize {
        return encode_with_full_merge_scan(tokens, merges, max_merges);
    }

    let mut current = tokens.to_vec();
    let mut profile = profile;
    let initial_pair_positions =
        build_initial_pair_positions(&current, max_merges, lookup_table, profile.as_deref_mut());
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
    let mut future_positions: FxHashMap<u32, PositionBucket> = FxHashMap::default();
    let mut position_pool: Vec<Vec<u32>> = Vec::new();
    let mut initial_cursor = 0usize;

    {
        let mut replay_state = ReplayState {
            current: &mut current,
            next: &mut next,
            prev: &mut prev,
            alive: &mut alive,
            future_positions: &mut future_positions,
            position_pool: &mut position_pool,
            max_merges,
            lookup_table,
            profile,
        };

        for (merge_idx, merge) in merges.iter().copied().enumerate().take(max_merges) {
            if initial_cursor >= initial_pair_positions.len()
                && replay_state.future_positions.is_empty()
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
            let pair = merge.pair;
            let new_token = merge.new_token;
            match replay_state.future_positions.remove(&(merge_idx as u32)) {
                Some(PositionBucket::One(position)) => {
                    if initial_positions.is_empty() {
                        apply_replay_merge_positions(
                            std::iter::once(position),
                            pair,
                            new_token,
                            merge_idx,
                            &mut replay_state,
                        );
                    } else {
                        let mut positions = replay_state.position_pool.pop().unwrap_or_default();
                        positions.push(position);
                        positions.extend(
                            initial_positions
                                .iter()
                                .map(|&candidate| candidate_left_position(candidate)),
                        );
                        positions.sort_unstable();
                        positions.dedup();
                        apply_replay_merge_positions(
                            positions.iter().copied(),
                            pair,
                            new_token,
                            merge_idx,
                            &mut replay_state,
                        );
                        positions.clear();
                        replay_state.position_pool.push(positions);
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
                    apply_replay_merge_positions(
                        positions.iter().copied(),
                        pair,
                        new_token,
                        merge_idx,
                        &mut replay_state,
                    );
                    positions.clear();
                    replay_state.position_pool.push(positions);
                }
                None => {
                    apply_replay_merge_positions(
                        initial_positions
                            .iter()
                            .map(|&candidate| candidate_left_position(candidate)),
                        pair,
                        new_token,
                        merge_idx,
                        &mut replay_state,
                    );
                }
            }
        }
    }

    compact_replay_tokens(current, &next)
}

fn build_initial_pair_positions(
    tokens: &[u32],
    max_merges: usize,
    lookup_table: &LookupTable,
    mut profile: Option<&mut QueryProfile>,
) -> Vec<u64> {
    let mut positions = Vec::new();
    for left in 0..tokens.len().saturating_sub(1) {
        let pair = (tokens[left], tokens[left + 1]);
        if let Some(profile) = profile.as_deref_mut() {
            profile.initial_pair_queries = profile.initial_pair_queries.saturating_add(1);
            profile.queries.push(pair);
        }
        if let Some(merge_idx) = lookup_table.lookup_capped(pair, max_merges) {
            positions.push(pack_initial_pair_position(merge_idx, left));
        }
    }
    positions.sort_unstable();
    positions
}

fn apply_replay_merge_positions<I>(
    positions: I,
    pair: (u32, u32),
    new_token: u32,
    merge_idx: usize,
    state: &mut ReplayState<'_>,
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
        if let Some(profile) = state.profile.as_deref_mut() {
            profile.applied_merges = profile.applied_merges.saturating_add(1);
        }

        let after_position = state.next[right];
        state.next[left] = after_position;
        if after_position != u32::MAX {
            state.prev[after_position as usize] = left_position;
        }

        push_replay_current_pair(state.prev[left], merge_idx, state);
        push_replay_current_pair(left_position, merge_idx, state);
    }
}

fn push_replay_current_pair(
    left_position: u32,
    current_merge_idx: usize,
    state: &mut ReplayState<'_>,
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
    if let Some(profile) = state.profile.as_deref_mut() {
        profile.post_merge_pair_queries = profile.post_merge_pair_queries.saturating_add(1);
        profile.queries.push(pair);
    }
    let Some(merge_idx) = state.lookup_table.lookup_capped(pair, state.max_merges) else {
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

fn compact_replay_tokens(mut tokens: Vec<u32>, next: &[u32]) -> Vec<u32> {
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

fn encode_with_full_merge_scan(
    tokens: &[u32],
    merges: &[MergeEntry],
    max_merges: usize,
) -> Vec<u32> {
    let mut current = tokens.to_vec();
    for merge in merges.iter().take(max_merges) {
        apply_full_scan_merge(&mut current, merge.pair, merge.new_token);
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

fn pack_initial_pair_position(merge_idx: usize, left: usize) -> u64 {
    ((merge_idx as u64) << u32::BITS) | left as u64
}

fn candidate_merge_idx(candidate: u64) -> usize {
    (candidate >> u32::BITS) as usize
}

fn candidate_left_position(candidate: u64) -> u32 {
    (candidate & u32::MAX as u64) as u32
}

fn benchmark_strategy(
    name: &'static str,
    inputs: &BenchmarkInputs<'_>,
    warmup_iterations: u32,
    iterations: u32,
) -> Result<StrategyReport, Box<dyn Error>> {
    let build_start = Instant::now();
    let lookup_table = LookupTable::build(name, inputs.merges)?;
    let build_time_ns = elapsed_ns_u64(build_start);
    let parity = strategy_encoding_parity(
        name,
        &lookup_table,
        inputs.token_sequences,
        inputs.runtime_encoded_sequences,
        inputs.merges,
        inputs.max_merges,
    )?;

    for _ in 0..warmup_iterations {
        black_box(run_lookup_pass(
            &lookup_table,
            inputs.max_merges,
            inputs.queries,
        ));
    }

    let mut lookup_time_ns = Vec::with_capacity(iterations as usize);
    let mut final_hits = 0usize;
    let mut final_checksum = 0u64;
    for _ in 0..iterations {
        let start = Instant::now();
        let (hits, checksum) = run_lookup_pass(&lookup_table, inputs.max_merges, inputs.queries);
        lookup_time_ns.push(elapsed_ns_u64(start));
        final_hits = hits;
        final_checksum = checksum;
        black_box((final_hits, final_checksum));
    }

    let best_lookup_time_ns = lookup_time_ns.iter().copied().min().unwrap_or(0);
    Ok(StrategyReport {
        name,
        build_time_ns,
        lookup_time_ns,
        best_lookup_time_ns,
        hits: final_hits,
        checksum: final_checksum,
        encoded_sequences: parity.sequence_count,
        encoded_tokens: parity.token_count,
        encoded_token_checksum: parity.checksum,
        matches_runtime: true,
    })
}

#[derive(Debug)]
struct EncodingParity {
    sequence_count: usize,
    token_count: usize,
    checksum: u64,
}

fn strategy_encoding_parity(
    name: &'static str,
    lookup_table: &LookupTable,
    token_sequences: &[Vec<u32>],
    runtime_encoded_sequences: &[Vec<u32>],
    merges: &[MergeEntry],
    max_merges: usize,
) -> Result<EncodingParity, Box<dyn Error>> {
    if token_sequences.len() != runtime_encoded_sequences.len() {
        return Err(format!(
            "{name}: internal parity setup mismatch: {} input sequences but {} runtime outputs",
            token_sequences.len(),
            runtime_encoded_sequences.len()
        )
        .into());
    }

    let mut token_count = 0usize;
    let mut checksum = encoded_checksum_seed();
    for (sequence_index, tokens) in token_sequences.iter().enumerate() {
        let replayed = replay_sequence_encode(tokens, merges, max_merges, lookup_table, None);
        let runtime = &runtime_encoded_sequences[sequence_index];
        if replayed != *runtime {
            return Err(format!(
                "{name}: replay/runtime mismatch for sequence {sequence_index}: input={tokens:?} replayed={replayed:?} runtime={runtime:?}"
            )
            .into());
        }
        update_encoded_checksum(&replayed, &mut token_count, &mut checksum);
    }

    Ok(EncodingParity {
        sequence_count: token_sequences.len(),
        token_count,
        checksum,
    })
}

fn encoded_sequences_summary(sequences: &[Vec<u32>]) -> (usize, usize, u64) {
    let mut token_count = 0usize;
    let mut checksum = encoded_checksum_seed();

    for sequence in sequences {
        update_encoded_checksum(sequence, &mut token_count, &mut checksum);
    }

    (sequences.len(), token_count, checksum)
}

fn update_encoded_checksum(sequence: &[u32], token_count: &mut usize, checksum: &mut u64) {
    *checksum ^= sequence.len() as u64;
    *checksum = (*checksum).wrapping_mul(0x0000_0100_0000_01b3);
    *token_count = token_count.saturating_add(sequence.len());

    for &token in sequence {
        *checksum ^= token as u64;
        *checksum = (*checksum).wrapping_mul(0x0000_0100_0000_01b3);
    }
}

fn encoded_checksum_seed() -> u64 {
    0xcbf2_9ce4_8422_2325u64
}

fn run_lookup_pass(table: &LookupTable, max_merges: usize, queries: &[(u32, u32)]) -> (usize, u64) {
    let mut hits = 0usize;
    let mut checksum = 0xcbf2_9ce4_8422_2325u64;
    for &pair in queries {
        let marker = match table.lookup_capped(pair, max_merges) {
            Some(index) => {
                hits = hits.saturating_add(1);
                index as u64 + 1
            }
            None => 0,
        };
        checksum ^= marker;
        checksum = checksum.wrapping_mul(0x0000_0100_0000_01b3);
    }
    (hits, checksum)
}

#[inline]
fn pair_key(pair: (u32, u32)) -> u64 {
    ((pair.0 as u64) << u32::BITS) | pair.1 as u64
}

fn elapsed_ns_u64(start: Instant) -> u64 {
    start.elapsed().as_nanos().try_into().unwrap_or(u64::MAX)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tct::bpe::{BpeMerge, BpeMetadata};

    fn test_table() -> BpeMergeTable {
        BpeMergeTable {
            schema: "test".to_string(),
            base_vocab_size: 257,
            merges: vec![
                BpeMerge {
                    pair: [1, 2],
                    new_token: 257,
                    frequency: 10,
                },
                BpeMerge {
                    pair: [257, 3],
                    new_token: 258,
                    frequency: 5,
                },
                BpeMerge {
                    pair: [4, 5],
                    new_token: 259,
                    frequency: 3,
                },
            ],
            metadata: BpeMetadata {
                trained_on_instances: 1,
                training_date: "2026-01-01".to_string(),
                max_merges: 3,
                dict_size: None,
            },
        }
    }

    #[test]
    fn lookup_strategies_agree_on_hits_and_checksum() {
        let table = test_table();
        let merges = merge_pairs(&table);
        let active_merges = active_merge_count(&table, table.bpe_start() + 3);
        let queries = vec![(1, 2), (2, 1), (257, 3), (4, 5), (99, 100)];
        let token_sequences = cascading_sequences();
        let runtime_encoder = SequentialBpeEncoder::new(&table, table.bpe_start() + 3);
        let runtime_encoded_sequences = token_sequences
            .iter()
            .map(|tokens| runtime_encoder.encode(tokens))
            .collect::<Vec<_>>();
        let inputs = BenchmarkInputs {
            merges: &merges,
            max_merges: active_merges,
            queries: &queries,
            token_sequences: &token_sequences,
            runtime_encoded_sequences: &runtime_encoded_sequences,
        };
        let reports = ["sorted_binary_search", "hash_map", "bucketed_by_left"]
            .iter()
            .map(|&name| benchmark_strategy(name, &inputs, 0, 1).unwrap())
            .collect::<Vec<_>>();

        assert_eq!(reports[0].hits, 3);
        assert!(
            reports
                .iter()
                .all(|report| report.hits == reports[0].hits
                    && report.checksum == reports[0].checksum)
        );
        assert!(reports.iter().all(|report| report.matches_runtime));
    }

    #[test]
    fn max_vocab_size_limits_active_merges() {
        let table = test_table();
        assert_eq!(active_merge_count(&table, table.bpe_start()), 0);
        assert_eq!(active_merge_count(&table, table.bpe_start() + 1), 1);
        assert_eq!(active_merge_count(&table, table.bpe_start() + 99), 3);
    }

    #[test]
    fn generated_encoder_query_replay_includes_post_merge_pairs() {
        let table = test_table();
        let merges = merge_pairs(&table);
        let profile = collect_generated_encoder_lookup_queries(
            &[vec![1, 2, 3, 4, 5], vec![4, 5, 99]],
            &merges,
            active_merge_count(&table, table.bpe_start() + 3),
        );

        assert_eq!(profile.initial_pair_queries, 6);
        assert_eq!(profile.post_merge_pair_queries, 4);
        assert_eq!(profile.queries.len(), 10);
        assert_eq!(profile.applied_merges, 4);
        assert!(profile.queries.contains(&(257, 3)));
        assert!(profile.queries.contains(&(258, 259)));
    }

    #[test]
    fn lookup_replay_outputs_match_runtime_encoder_for_all_strategies() {
        let table = test_table();
        let merges = merge_pairs(&table);
        let active_merges = active_merge_count(&table, table.bpe_start() + 3);
        let token_sequences = cascading_sequences();
        let runtime_encoder = SequentialBpeEncoder::new(&table, table.bpe_start() + 3);
        let runtime_encoded_sequences = token_sequences
            .iter()
            .map(|tokens| runtime_encoder.encode(tokens))
            .collect::<Vec<_>>();
        assert_eq!(
            runtime_encoded_sequences,
            vec![
                vec![258, 259],
                vec![259, 99],
                vec![258, 258],
                vec![9],
                vec![258, 259, 258]
            ]
        );

        for name in ["sorted_binary_search", "hash_map", "bucketed_by_left"] {
            let lookup_table = LookupTable::build(name, &merges).unwrap();
            let parity = strategy_encoding_parity(
                name,
                &lookup_table,
                &token_sequences,
                &runtime_encoded_sequences,
                &merges,
                active_merges,
            )
            .unwrap();

            assert_eq!(parity.sequence_count, token_sequences.len());
            assert_eq!(parity.token_count, 10);
        }
    }

    fn cascading_sequences() -> Vec<Vec<u32>> {
        vec![
            vec![1, 2, 3, 4, 5],
            vec![4, 5, 99],
            vec![1, 2, 3, 1, 2, 3],
            vec![9],
            vec![1, 2, 3, 4, 5, 1, 2, 3],
        ]
    }
}
