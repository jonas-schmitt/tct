//! BPE Training Binary
//!
//! Trains Byte Pair Encoding (BPE) merge tables from JSON instance data.
//!
//! # Workflow
//! 1. Compile schema to generate tokenizer
//! 2. Encode all instances to base tokens
//! 3. Count pair frequencies (per instance, then aggregate)
//! 4. Select top N most frequent pairs as merges
//! 5. Output merge table to JSON file
//!
//! # Usage (Traditional - encode instances during training)
//! ```bash
//! tct-train-bpe \
//!   --schema schemas/popular/github-workflow.json \
//!   --instances data/workflows/*.json \
//!   --target-vocab 2048 \
//!   --output schemas/bpe_merges/github-workflow-bpe2048.json
//! ```
//!
//! # Usage (Pre-encoded - faster, uses output from tct-prepare-data)
//! ```bash
//! tct-train-bpe \
//!   --schema schemas/popular/kubernetes.json \
//!   --encoded data/k8s-encoded/all.jsonl \
//!   --target-vocab 2048 \
//!   --output schemas/bpe/kubernetes-bpe2048.json
//! ```

use clap::{Arg, ArgMatches, Command, value_parser};
use rand::SeedableRng;
use rand::seq::SliceRandom;
use rayon::prelude::*;
use serde::Serialize;
use serde_json::Value;
use std::env;
use std::error::Error;
use std::fs::{self, File};
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Instant;
use tct::TctCompiler;
use tct::bpe::trainer::{
    BpeTrainingOptions, BpeTrainingProgress, train_bpe_incremental_with_options,
};
use tct::bpe::{BpeMergeTable, BpeMetadata, bpe_start_token};
use tct::instance_encoder::{InstanceEncoder, expand_glob_pattern};
use tct::provenance::{FileProvenance, file_provenance, sha256_hex};

const PAD_TOKEN_COUNT: usize = 1;
/// Universal model special tokens reserved on top of PAD for every training run:
/// begin-of-sequence and end-of-sequence. Schema-id tokens are reserved
/// separately and only when multi-schema `--schema-ids` are provided.
const BOS_TOKEN_COUNT: usize = 1;
const EOS_TOKEN_COUNT: usize = 1;
const TRAINING_DATE_ENV: &str = "TCT_BPE_TRAINING_DATE";
const SAMPLE_SEED: u64 = 42;

#[derive(Debug, Serialize)]
struct BpeTrainingProvenance {
    command: Vec<String>,
    schema_name: String,
    instances_pattern: Option<String>,
    encoded_input: Option<String>,
    cache_path: Option<String>,
    schema_file: Option<FileProvenance>,
    encoded_input_file: Option<FileProvenance>,
    prepare_data_metadata_file: Option<FileProvenance>,
    cache_file: Option<FileProvenance>,
    input_files: Vec<FileProvenance>,
    input_corpus_fingerprint: String,
    token_corpus: TokenCorpusProvenance,
    output_files: Vec<FileProvenance>,
    base_vocab_size: u32,
    base_vocab_source: String,
    optimization_level: Option<String>,
    target_vocab: Option<usize>,
    target_avg_length: Option<f64>,
    schema_ids: Vec<String>,
    reserved_special_tokens: usize,
    max_merges_deprecated: Option<usize>,
    checkpoints: Option<Vec<usize>>,
    sample_size: Option<usize>,
    trained_on_instances: usize,
    generated_merges: usize,
    timings: BpeTrainingTimings,
    provenance_path: String,
    created_at: String,
    tct_version: String,
    repo_commit: Option<String>,
    repo_dirty: Option<bool>,
}

#[derive(Debug, Default, Serialize)]
struct BpeTrainingTimings {
    training_wall_time_ns: u64,
    provenance_hashing_ns: u64,
}

#[derive(Debug, Serialize)]
struct TokenCorpusProvenance {
    source: &'static str,
    sequence_count: usize,
    sequence_fingerprint: String,
    sample_size: Option<usize>,
    sample_seed: Option<u64>,
    cache_path: Option<String>,
    cache_hit: bool,
    cache_written: bool,
    parallel_early_stop: bool,
    truncated_to_sample_size: bool,
    selected_instance_files: Vec<FileProvenance>,
    accepted_instance_files: Vec<FileProvenance>,
    rejected_instance_files: Vec<FileProvenance>,
}

struct LoadedTokenCorpus {
    sequences: Vec<Vec<u32>>,
    provenance: TokenCorpusProvenance,
}

struct InstanceLoadConfig<'a> {
    instances_pattern: &'a str,
    code: &'a str,
    root_type_name: &'a str,
    base_vocab_size: u32,
    sample_size: Option<usize>,
    cache_path: Option<&'a String>,
    output_path: &'a str,
}

enum EncodeAttempt {
    Success { path: PathBuf, tokens: Vec<u32> },
    Failure { path: PathBuf },
}

fn measure_provenance_hashing<T>(
    timings: &mut BpeTrainingTimings,
    operation: impl FnOnce() -> T,
) -> T {
    let start = Instant::now();
    let result = operation();
    timings.provenance_hashing_ns = timings
        .provenance_hashing_ns
        .saturating_add(elapsed_ns_u64(start));
    result
}

fn elapsed_ns_u64(start: Instant) -> u64 {
    start.elapsed().as_nanos().try_into().unwrap_or(u64::MAX)
}

fn merge_count_for_generated_vocab(
    vocab_size: usize,
    bpe_start: usize,
    reserved_special_tokens: usize,
) -> usize {
    vocab_size.saturating_sub(bpe_start + PAD_TOKEN_COUNT + reserved_special_tokens)
}

fn generated_vocab_size(
    bpe_start: usize,
    merge_count: usize,
    reserved_special_tokens: usize,
) -> usize {
    bpe_start + merge_count + PAD_TOKEN_COUNT + reserved_special_tokens
}

fn provenance_path_for_output(output_path: &str) -> String {
    let path = Path::new(output_path);
    let mut provenance_path = PathBuf::from(path);
    if path.extension().and_then(|ext| ext.to_str()) == Some("json") {
        provenance_path.set_extension("provenance.json");
    } else {
        provenance_path = PathBuf::from(format!("{}.provenance.json", output_path));
    }
    provenance_path.display().to_string()
}

fn training_date() -> Result<String, Box<dyn Error>> {
    if let Ok(value) = env::var(TRAINING_DATE_ENV) {
        chrono::NaiveDate::parse_from_str(&value, "%Y-%m-%d").map_err(|error| {
            input_error(format!(
                "{TRAINING_DATE_ENV} must use YYYY-MM-DD format: {error}"
            ))
        })?;
        return Ok(value);
    }

    Ok(chrono::Utc::now().format("%Y-%m-%d").to_string())
}

fn git_command_output(args: &[&str]) -> Option<String> {
    let output = std::process::Command::new("git").args(args).output().ok()?;
    if !output.status.success() {
        return None;
    }
    let stdout = String::from_utf8(output.stdout).ok()?;
    Some(stdout.trim().to_string())
}

fn repo_commit() -> Option<String> {
    git_command_output(&["rev-parse", "HEAD"]).filter(|commit| !commit.is_empty())
}

fn repo_dirty() -> Option<bool> {
    git_command_output(&["status", "--porcelain"]).map(|status| !status.is_empty())
}

fn adjacent_prepare_data_metadata(encoded_path: Option<&String>) -> Option<PathBuf> {
    encoded_path
        .and_then(|path| Path::new(path).parent())
        .map(|parent| parent.join("metadata.json"))
        .filter(|path| path.exists())
}

/// Load parallel role bitmaps from JSONL (one `Vec<u8>` array per line).
fn load_roles_jsonl(path: &str) -> Result<Vec<Vec<u8>>, Box<dyn Error>> {
    let file = File::open(path)?;
    let reader = BufReader::new(file);
    let mut roles = Vec::new();
    for (i, line) in reader.lines().enumerate() {
        let line = line?;
        if line.trim().is_empty() {
            continue;
        }
        let role_seq: Vec<u8> = serde_json::from_str(&line)
            .map_err(|e| format!("Failed to parse roles line {}: {}", i + 1, e))?;
        roles.push(role_seq);
    }
    Ok(roles)
}

fn optional_file_provenance(
    path: Option<&str>,
    timings: &mut BpeTrainingTimings,
) -> Result<Option<FileProvenance>, Box<dyn Error>> {
    match path {
        Some(path) if Path::new(path).exists() => {
            Ok(Some(measure_provenance_hashing(timings, || {
                file_provenance(path)
            })?))
        }
        _ => Ok(None),
    }
}

fn input_file_provenances(
    schema_path: Option<&String>,
    instances_pattern: Option<&String>,
    encoded_path: Option<&String>,
    timings: &mut BpeTrainingTimings,
) -> Result<Vec<FileProvenance>, Box<dyn Error>> {
    let mut records = Vec::new();

    if let Some(path) = schema_path {
        records.push(measure_provenance_hashing(timings, || {
            file_provenance(path)
        })?);
    }

    if let Some(pattern) = instances_pattern {
        let mut paths = expand_glob_pattern(pattern)?;
        paths.retain(|path| path.is_file());
        paths.sort();
        for path in paths {
            records.push(measure_provenance_hashing(timings, || {
                file_provenance(path)
            })?);
        }
    }

    if let Some(path) = encoded_path {
        records.push(measure_provenance_hashing(timings, || {
            file_provenance(path)
        })?);
    }

    records.sort_by(|left, right| left.path.cmp(&right.path));
    Ok(records)
}

fn input_corpus_fingerprint(
    records: &[FileProvenance],
    timings: &mut BpeTrainingTimings,
) -> String {
    measure_provenance_hashing(timings, || input_corpus_fingerprint_inner(records))
}

fn input_corpus_fingerprint_inner(records: &[FileProvenance]) -> String {
    let mut bytes = Vec::new();
    bytes.extend_from_slice(b"tct-bpe-artifact-inputs-v1\n");
    for record in records {
        bytes.extend_from_slice(record.path.as_bytes());
        bytes.push(0);
        bytes.extend_from_slice(record.bytes.to_string().as_bytes());
        bytes.push(0);
        bytes.extend_from_slice(record.sha256.as_bytes());
        bytes.push(b'\n');
    }
    sha256_hex(&bytes)
}

fn token_sequence_fingerprint(sequences: &[Vec<u32>], timings: &mut BpeTrainingTimings) -> String {
    measure_provenance_hashing(timings, || token_sequence_fingerprint_inner(sequences))
}

fn token_sequence_fingerprint_inner(sequences: &[Vec<u32>]) -> String {
    let mut bytes = Vec::new();
    bytes.extend_from_slice(b"tct-bpe-token-corpus-v1\n");
    for sequence in sequences {
        bytes.extend_from_slice(sequence.len().to_string().as_bytes());
        bytes.push(b':');
        for token in sequence {
            bytes.extend_from_slice(token.to_string().as_bytes());
            bytes.push(b',');
        }
        bytes.push(b'\n');
    }
    sha256_hex(&bytes)
}

fn file_provenances_for_paths(
    paths: &[PathBuf],
    timings: &mut BpeTrainingTimings,
) -> Result<Vec<FileProvenance>, Box<dyn Error>> {
    paths
        .iter()
        .map(|path| measure_provenance_hashing(timings, || file_provenance(path)))
        .collect::<Result<Vec<_>, _>>()
        .map_err(Into::into)
}

fn token_corpus_provenance_for_encoded(
    sequences: &[Vec<u32>],
    timings: &mut BpeTrainingTimings,
) -> TokenCorpusProvenance {
    TokenCorpusProvenance {
        source: "encoded_input",
        sequence_count: sequences.len(),
        sequence_fingerprint: token_sequence_fingerprint(sequences, timings),
        sample_size: None,
        sample_seed: None,
        cache_path: None,
        cache_hit: false,
        cache_written: false,
        parallel_early_stop: false,
        truncated_to_sample_size: false,
        selected_instance_files: Vec::new(),
        accepted_instance_files: Vec::new(),
        rejected_instance_files: Vec::new(),
    }
}

fn validate_token_sequence_below_base_vocab(
    tokens: &[u32],
    base_vocab_size: u32,
    context: &str,
    sequence_index: usize,
) -> Result<(), Box<dyn Error>> {
    for (token_position, token) in tokens.iter().copied().enumerate() {
        if token >= base_vocab_size {
            return Err(input_error(format!(
                "{context} sequence {} token position {} has token id {}, but BPE training base vocabulary size {} requires raw base token ids below {}",
                sequence_index, token_position, token, base_vocab_size, base_vocab_size
            )));
        }
    }
    Ok(())
}

fn validate_token_sequences_below_base_vocab(
    sequences: &[Vec<u32>],
    base_vocab_size: u32,
    context: &str,
) -> Result<(), Box<dyn Error>> {
    for (sequence_index, tokens) in sequences.iter().enumerate() {
        validate_token_sequence_below_base_vocab(
            tokens,
            base_vocab_size,
            context,
            sequence_index + 1,
        )?;
    }
    Ok(())
}

fn report_training_progress(event: BpeTrainingProgress) {
    match event {
        BpeTrainingProgress::Deduplicating { sequence_count } => {
            eprintln!("  Deduplicating {} sequences...", sequence_count);
        }
        BpeTrainingProgress::Deduplicated { unique_sequences } => {
            eprintln!(
                "  {} unique sequences (deduplication complete)",
                unique_sequences
            );
        }
        BpeTrainingProgress::InitialAverageLength { average } => {
            eprintln!("  Initial avg sequence length: {:.2}", average);
        }
        BpeTrainingProgress::InitialPairCounts { unique_pairs } => {
            eprintln!("  {} unique pairs found", unique_pairs);
            eprintln!("  Building priority queue...");
        }
        BpeTrainingProgress::StartingMergeLoop { max_merges } => {
            eprintln!("  Starting merge loop ({} max merges)...", max_merges);
        }
        BpeTrainingProgress::NoMorePairs { merges_done } => {
            eprintln!("  ⚠️  No more pairs to merge after {} merges", merges_done);
        }
        BpeTrainingProgress::TargetAverageReached {
            current_average,
            target,
            merges_done,
        } => {
            eprintln!(
                "  ✅ Target avg length reached: {:.2} <= {:.1} after {} merges",
                current_average, target, merges_done
            );
        }
        BpeTrainingProgress::Merge {
            merges_done,
            current_average,
            vocab_size,
            pair,
            new_token,
            frequency,
        } => {
            eprintln!(
                "  merge {:>5}: avg={:.2} vocab={} pair=({},{})→{} freq={}",
                merges_done, current_average, vocab_size, pair.0, pair.1, new_token, frequency
            );
        }
        BpeTrainingProgress::Completed { merges } => {
            eprintln!("  ✅ Completed {} merges", merges);
        }
    }
}

/// Load token sequences from cache or encode from instances
///
/// Features:
/// - Caching: Save/load encoded sequences to avoid re-encoding
/// - Random sampling: Ensures exactly N valid sequences when sample_size is set
/// - Representative sample: Uses proper random distribution (shuffling)
/// - Failure handling: Accounts for non-encodable manifests
fn load_or_encode_instances(
    config: InstanceLoadConfig<'_>,
    timings: &mut BpeTrainingTimings,
) -> Result<LoadedTokenCorpus, Box<dyn Error>> {
    // Try loading from cache first
    if let Some(cache_file) = config.cache_path {
        if Path::new(cache_file).exists() {
            eprintln!("📦 Loading token sequences from cache: {}", cache_file);
            let cache_content = fs::read_to_string(cache_file)?;
            let token_sequences: Vec<Vec<u32>> = serde_json::from_str(&cache_content)?;
            validate_token_sequences_below_base_vocab(
                &token_sequences,
                config.base_vocab_size,
                &format!("cache {cache_file}"),
            )?;
            eprintln!("  ✅ Loaded {} sequences from cache", token_sequences.len());
            eprintln!();
            let provenance = TokenCorpusProvenance {
                source: "cache",
                sequence_count: token_sequences.len(),
                sequence_fingerprint: token_sequence_fingerprint(&token_sequences, timings),
                sample_size: config.sample_size,
                sample_seed: config.sample_size.map(|_| SAMPLE_SEED),
                cache_path: Some(cache_file.clone()),
                cache_hit: true,
                cache_written: false,
                parallel_early_stop: false,
                truncated_to_sample_size: false,
                selected_instance_files: Vec::new(),
                accepted_instance_files: Vec::new(),
                rejected_instance_files: Vec::new(),
            };
            return Ok(LoadedTokenCorpus {
                sequences: token_sequences,
                provenance,
            });
        } else {
            eprintln!("📦 Cache file not found: {}", cache_file);
            eprintln!("  Will encode instances and save to cache");
            eprintln!();
        }
    }

    // Load instance paths
    eprintln!(
        "📂 Loading instances from pattern: {}",
        config.instances_pattern
    );
    let mut instance_paths = expand_glob_pattern(config.instances_pattern)?;
    eprintln!("  Found {} instance files", instance_paths.len());

    if instance_paths.is_empty() {
        return Err("No instance files found matching pattern".into());
    }

    // Random sampling: shuffle and take more than needed to account for failures
    if let Some(target_size) = config.sample_size {
        eprintln!(
            "🎲 Random sampling: targeting {} valid sequences",
            target_size
        );

        // Use deterministic seed for reproducibility
        let mut rng = rand::rngs::StdRng::seed_from_u64(SAMPLE_SEED);
        instance_paths.shuffle(&mut rng);

        eprintln!(
            "  ✅ Shuffled {} files with seed={} (reproducible)",
            instance_paths.len(),
            SAMPLE_SEED
        );
        eprintln!();
    }

    let selected_instance_files = if config.sample_size.is_some() {
        file_provenances_for_paths(&instance_paths, timings)?
    } else {
        Vec::new()
    };

    if config.sample_size.is_some() {
        eprintln!(
            "  🧾 Recorded {} shuffled candidate files for provenance",
            instance_paths.len()
        );
        eprintln!();
    }

    // Encode instances (PARALLELIZED for performance)
    eprintln!("🔢 Encoding instances to tokens (parallel)...");
    eprintln!("  Creating instance encoder...");
    let encoder = Arc::new(InstanceEncoder::new(config.code, config.root_type_name)?);

    let target_sequences = config.sample_size.unwrap_or(instance_paths.len());
    eprintln!("  Target: {} valid token sequences", target_sequences);
    eprintln!();

    // Atomic counters for thread-safe progress tracking
    let processed = Arc::new(AtomicUsize::new(0));
    let successful_count = Arc::new(AtomicUsize::new(0));
    let failed_count = Arc::new(AtomicUsize::new(0));

    // Parallel encoding with Rayon - with dynamic early stopping
    let results: Vec<_> = instance_paths
        .par_iter()
        .filter_map(|path| {
            // Clone Arc references for this thread
            let encoder = Arc::clone(&encoder);
            let processed = Arc::clone(&processed);
            let successful_count = Arc::clone(&successful_count);
            let failed_count = Arc::clone(&failed_count);

            // Early stopping: check if we've reached target
            if let Some(target) = config.sample_size {
                let current_successful = successful_count.load(Ordering::Relaxed);
                // Stop if we already have enough successful encodings
                if current_successful >= target {
                    return None;
                }
            }

            // Encode the instance
            let result = encoder.encode_file(path);

            // Update counters
            let current_processed = processed.fetch_add(1, Ordering::Relaxed) + 1;

            // Progress reporting every 1000 instances
            if current_processed.is_multiple_of(1000) {
                eprintln!(
                    "  Progress: {}/{} processed ({} successful, {} failed)",
                    current_processed,
                    instance_paths.len(),
                    successful_count.load(Ordering::Relaxed),
                    failed_count.load(Ordering::Relaxed)
                );
            }

            // Check again after encoding in case we just crossed the threshold
            if let Some(target) = config.sample_size {
                let current_successful = successful_count.load(Ordering::Relaxed);
                if current_successful >= target && current_processed.is_multiple_of(5000) {
                    eprintln!(
                        "  ⏹️  Stopping: reached {} successful encodings (target: {})",
                        current_successful, target
                    );
                }
            }

            match result {
                Ok(tokens) if !tokens.is_empty() => {
                    let success_idx = successful_count.fetch_add(1, Ordering::Relaxed);
                    if success_idx < 10 {
                        eprintln!("  ✅ Encoded: {}", path.display());
                    }
                    Some(EncodeAttempt::Success {
                        path: path.clone(),
                        tokens,
                    })
                }
                Ok(_) => {
                    // Empty token sequence
                    let fail_idx = failed_count.fetch_add(1, Ordering::Relaxed);
                    if fail_idx < 10 {
                        eprintln!("  ⚠️  Empty token sequence: {}", path.display());
                    } else if fail_idx == 10 {
                        eprintln!("  ⚠️  ... suppressing further error messages");
                    }
                    Some(EncodeAttempt::Failure { path: path.clone() })
                }
                Err(e) => {
                    let fail_idx = failed_count.fetch_add(1, Ordering::Relaxed);
                    if fail_idx < 10 {
                        eprintln!("  ⚠️  Failed to encode {}: {}", path.display(), e);
                    } else if fail_idx == 10 {
                        eprintln!("  ⚠️  ... suppressing further error messages");
                    }
                    Some(EncodeAttempt::Failure { path: path.clone() })
                }
            }
        })
        .collect();

    // Separate successful encodings from failed paths
    let mut token_sequences = Vec::new();
    let mut failed_paths = Vec::new();
    let mut accepted_paths = Vec::new();

    for result in results {
        match result {
            EncodeAttempt::Success { path, tokens } => {
                validate_token_sequence_below_base_vocab(
                    &tokens,
                    config.base_vocab_size,
                    &format!("instance {}", path.display()),
                    accepted_paths.len() + 1,
                )?;
                accepted_paths.push(path);
                token_sequences.push(tokens);
            }
            EncodeAttempt::Failure { path } => failed_paths.push(path),
        }
    }

    // Truncate to target if we got more than needed (when sampling)
    let mut truncated_to_sample_size = false;
    if token_sequences.len() > target_sequences {
        token_sequences.truncate(target_sequences);
        accepted_paths.truncate(target_sequences);
        truncated_to_sample_size = true;
        eprintln!(
            "  ✅ Truncated to target of {} valid sequences",
            target_sequences
        );
    }

    let successful = token_sequences.len();
    let failed = failed_paths.len();

    eprintln!();
    eprintln!("📊 Encoding statistics:");
    eprintln!("  Files processed: {}", successful + failed);
    eprintln!(
        "  Successfully encoded: {} ({:.1}%)",
        successful,
        (successful as f64 / (successful + failed) as f64) * 100.0
    );
    eprintln!(
        "  Failed encoding: {} ({:.1}%)",
        failed,
        (failed as f64 / (successful + failed) as f64) * 100.0
    );
    eprintln!();

    if token_sequences.is_empty() {
        return Err("No instances were successfully encoded - cannot train BPE".into());
    }

    // Verify we reached target if sampling
    if let Some(target) = config.sample_size {
        if successful < target {
            eprintln!(
                "⚠️  WARNING: Only {} valid sequences found (target: {})",
                successful, target
            );
            eprintln!("   Consider using larger dataset or reducing --sample-size");
            eprintln!();
        } else {
            eprintln!(
                "✅ Random sampling complete: {} valid sequences",
                successful
            );
            eprintln!();
        }
    }

    // Write failed paths to a file
    if failed > 0 {
        let failed_log_path = format!("{}.failed_instances.txt", config.output_path);
        let mut failed_log = String::new();
        for path in &failed_paths {
            failed_log.push_str(&format!("{}\n", path.display()));
        }
        fs::write(&failed_log_path, failed_log)?;

        eprintln!("⚠️  {} instances failed encoding and were skipped", failed);
        eprintln!("   Failed instance paths written to: {}", failed_log_path);
        eprintln!();
    }

    // Save to cache if specified
    if let Some(cache_file) = config.cache_path {
        eprintln!("💾 Saving token sequences to cache: {}", cache_file);
        let cache_json = serde_json::to_string(&token_sequences)?;
        fs::write(cache_file, cache_json)?;
        eprintln!("  ✅ Saved {} sequences to cache", token_sequences.len());
        eprintln!();
    }

    let provenance = TokenCorpusProvenance {
        source: "instance_encoding",
        sequence_count: token_sequences.len(),
        sequence_fingerprint: token_sequence_fingerprint(&token_sequences, timings),
        sample_size: config.sample_size,
        sample_seed: config.sample_size.map(|_| SAMPLE_SEED),
        cache_path: config.cache_path.cloned(),
        cache_hit: false,
        cache_written: config.cache_path.is_some(),
        parallel_early_stop: config.sample_size.is_some(),
        truncated_to_sample_size,
        selected_instance_files,
        accepted_instance_files: file_provenances_for_paths(&accepted_paths, timings)?,
        rejected_instance_files: file_provenances_for_paths(&failed_paths, timings)?,
    };

    Ok(LoadedTokenCorpus {
        sequences: token_sequences,
        provenance,
    })
}

fn main() -> Result<(), Box<dyn Error>> {
    let command: Vec<String> = std::env::args().collect();
    let mut timings = BpeTrainingTimings::default();
    let matches = Command::new("tct-train-bpe")
        .about("Train BPE merge tables from JSON instance data")
        .version("0.1.0")
        .arg(
            Arg::new("schema")
                .short('s')
                .long("schema")
                .help("JSON schema file (required for TCT mode, optional for --encoded with --base-vocab)")
                .required(false)
                .value_parser(value_parser!(String)),
        )
        .arg(
            Arg::new("instances")
                .short('i')
                .long("instances")
                .help("Instance JSON files (glob pattern) - not needed if --encoded is provided")
                .required(false)
                .value_parser(value_parser!(String)),
        )
        .arg(
            Arg::new("encoded")
                .short('e')
                .long("encoded")
                .help("Pre-encoded JSONL file (from tct-prepare-data) - faster than encoding during training")
                .value_parser(value_parser!(String)),
        )
        .arg(
            Arg::new("target-vocab")
                .short('t')
                .long("target-vocab")
                .help("Vocabulary budget: base + merges + PAD + BOS/EOS + one slot per --schema-ids entry")
                .value_parser(value_parser!(usize)),
        )
        .arg(
            Arg::new("schema-ids")
                .long("schema-ids")
                .help(
                    "Comma-separated schema identifier tokens for multi-schema training (e.g. \
                     tsconfig,eslintrc,kubernetes). One vocabulary slot is reserved per id, in \
                     addition to the always-reserved BOS, EOS, and PAD. Omit for single-schema \
                     training (the generated single-schema sequence wrapper then adds one schema \
                     token beyond the budget). The special-token count is derived automatically; \
                     --target-vocab / --checkpoints name the vocabulary including the special \
                     tokens reserved here.",
                )
                .value_parser(value_parser!(String)),
        )
        .arg(
            Arg::new("max-merges")
                .short('m')
                .long("max-merges")
                .help("[DEPRECATED] Use --target-vocab instead. Maximum number of merges to generate")
                .value_parser(value_parser!(usize))
                .conflicts_with("target-vocab"),
        )
        .arg(
            Arg::new("target-avg-length")
                .long("target-avg-length")
                .help("Stop training when average sequence length reaches this target")
                .value_parser(value_parser!(f64)),
        )
        .arg(
            Arg::new("base-vocab")
                .long("base-vocab")
                .help("Base vocabulary size (required for --encoded mode without --schema)")
                .value_parser(value_parser!(u32)),
        )
        .arg(
            Arg::new("output")
                .short('o')
                .long("output")
                .help("Output merge file path")
                .required(true)
                .value_parser(value_parser!(String)),
        )
        .arg(
            Arg::new("name")
                .short('n')
                .long("name")
                .help("Schema name for merge table")
                .value_parser(value_parser!(String)),
        )
        .arg(
            Arg::new("checkpoints")
                .short('C')
                .long("checkpoints")
                .help("Save one merge table per vocabulary budget (comma-separated, e.g., 512,1024,2048,4096)")
                .value_parser(value_parser!(String)),
        )
        .arg(
            Arg::new("sample-size")
                .long("sample-size")
                .help("Randomly sample N instances from the dataset (ensures exactly N valid token sequences)")
                .value_parser(value_parser!(usize)),
        )
        .arg(
            Arg::new("cache")
                .long("cache")
                .help("Cache file for encoded token sequences (speeds up reruns)")
                .value_parser(value_parser!(String)),
        )
        .arg(
            Arg::new("optimization")
                .short('O')
                .long("optimization")
                .help("Optimization profile: 0/O0, 2/O2, 3/O3, s/Os/size, or vocab/OVocab")
                .value_parser(value_parser!(String))
                .default_value("3"),
        )
        .arg(
            Arg::new("verbose")
                .short('v')
                .long("verbose")
                .help("Log average sequence length after each merge")
                .action(clap::ArgAction::SetTrue),
        )
        .arg(
            Arg::new("decimal-numbers")
                .long("decimal-numbers")
                .help("Digit-aligned decimal text encoding for unbounded i64/u64/f64 (encoding must match any --bpe-merges and downstream wheels)")
                .action(clap::ArgAction::SetTrue),
        )
        .arg(
            Arg::new("structure-only-merges")
                .long("structure-only-merges")
                .help(
                    "Forbid merges that span value payload tokens (string bytes, decimal digit \
                     bytes, END_STRING_MARKER). Requires a parallel role bitmap from \
                     tct-prepare-data --export-roles (all.roles.jsonl beside --encoded, or \
                     --roles <path>).",
                )
                .action(clap::ArgAction::SetTrue),
        )
        .arg(
            Arg::new("roles")
                .long("roles")
                .help(
                    "Parallel role bitmap JSONL (one u8 role array per line, aligned with \
                     --encoded). Defaults to <encoded-dir>/all.roles.jsonl when \
                     --structure-only-merges is set.",
                )
                .value_parser(value_parser!(String)),
        )
        .get_matches();

    let schema_path = matches.get_one::<String>("schema");
    let instances_pattern = matches.get_one::<String>("instances");
    let encoded_path = matches.get_one::<String>("encoded");
    let target_vocab = matches.get_one::<usize>("target-vocab").copied();
    // Schema-id tokens are conditional (multi-schema only); BOS/EOS/PAD are always
    // reserved. The special-token reserve is therefore derived automatically and
    // never passed as a raw count.
    let schema_ids: Vec<String> = matches
        .get_one::<String>("schema-ids")
        .map(|s| {
            s.split(',')
                .map(|part| part.trim().to_string())
                .filter(|part| !part.is_empty())
                .collect()
        })
        .unwrap_or_default();
    // Reserved beyond PAD (PAD is accounted separately via PAD_TOKEN_COUNT):
    // BOS + EOS always, plus one slot per schema id when multi-schema.
    let reserved_special_tokens = BOS_TOKEN_COUNT + EOS_TOKEN_COUNT + schema_ids.len();
    let target_avg_length = matches.get_one::<f64>("target-avg-length").copied();
    let max_merges_deprecated = matches.get_one::<usize>("max-merges").copied();
    let output_path = required_arg::<String>(&matches, "output")?;
    let schema_name = matches.get_one::<String>("name");
    let checkpoints_str = matches.get_one::<String>("checkpoints");
    let sample_size = matches.get_one::<usize>("sample-size").copied();
    let cache_path = matches.get_one::<String>("cache");
    let optimization_str = required_arg::<String>(&matches, "optimization")?;
    let optimization_level = tct::OptimizationLevel::parse_cli(optimization_str)
        .map_err(|error| format!("{}.", error))?;
    let decimal_numbers = matches.get_flag("decimal-numbers");
    let structure_only_merges = matches.get_flag("structure-only-merges");
    let roles_path_arg = matches.get_one::<String>("roles").cloned();
    let explicit_base_vocab = matches.get_one::<u32>("base-vocab").copied();
    let verbose = matches.get_flag("verbose");

    // Validate input mode
    if instances_pattern.is_some() && schema_path.is_none() {
        eprintln!("Error: --instances requires --schema");
        std::process::exit(1);
    }

    let prepare_data_metadata_path = adjacent_prepare_data_metadata(encoded_path);

    // Try to auto-detect base_vocab from metadata.json if --encoded is used
    let auto_base_vocab: Option<u32> =
        if encoded_path.is_some() && schema_path.is_none() && explicit_base_vocab.is_none() {
            if let Some(meta_path) = &prepare_data_metadata_path {
                if let Ok(content) = fs::read_to_string(meta_path) {
                    if let Ok(meta) = serde_json::from_str::<serde_json::Value>(&content) {
                        if let Some(bv) = meta.get("base_vocab_size").and_then(|v| v.as_u64()) {
                            eprintln!(
                                "📋 Auto-detected base_vocab_size: {} (from {})",
                                bv,
                                meta_path.display()
                            );
                            Some(bv as u32)
                        } else {
                            None
                        }
                    } else {
                        None
                    }
                } else {
                    None
                }
            } else {
                None
            }
        } else {
            None
        };

    // Default base vocab: 257 for TCT (256 bytes + end marker)
    const DEFAULT_TCT_BASE_VOCAB: u32 = 257;

    // Use default if no other source
    let final_base_vocab = explicit_base_vocab
        .or(auto_base_vocab)
        .unwrap_or(DEFAULT_TCT_BASE_VOCAB);
    let base_vocab_source = if schema_path.is_some() {
        "schema"
    } else if explicit_base_vocab.is_some() {
        "explicit"
    } else if auto_base_vocab.is_some() {
        "prepare-data-metadata"
    } else {
        "default"
    }
    .to_string();

    // Log if using default
    if encoded_path.is_some()
        && schema_path.is_none()
        && explicit_base_vocab.is_none()
        && auto_base_vocab.is_none()
    {
        eprintln!(
            "📋 Using default base_vocab_size: {} (TCT default)",
            DEFAULT_TCT_BASE_VOCAB
        );
    }

    if instances_pattern.is_none() && encoded_path.is_none() {
        eprintln!("Error: Must specify either --instances or --encoded");
        eprintln!("  --instances: Encode instances during training (requires --schema)");
        eprintln!("  --encoded: Use pre-encoded JSONL from tct-prepare-data");
        std::process::exit(1);
    }

    // Show deprecation warning if old parameter used
    if max_merges_deprecated.is_some() {
        eprintln!("⚠️  WARNING: --max-merges is deprecated, use --target-vocab instead");
        eprintln!();
    }

    // Require at least one stopping criterion
    if target_vocab.is_none()
        && max_merges_deprecated.is_none()
        && checkpoints_str.is_none()
        && target_avg_length.is_none()
    {
        eprintln!("❌ Error: Must specify a stopping criterion:");
        eprintln!("   --target-vocab <size>       Stop at generated VOCAB_SIZE");
        eprintln!("   --target-avg-length <len>   Stop when avg sequence length reached");
        eprintln!(
            "   --checkpoints <sizes>       Save multiple generated VOCAB_SIZE checkpoint files"
        );
        eprintln!("   Example: --target-vocab 2048");
        eprintln!("   Example: --checkpoints 512,1024,2048,4096");
        eprintln!("   Example: --target-avg-length 220.0");
        std::process::exit(1);
    }

    let source_repo_commit = repo_commit();
    let source_repo_dirty = repo_dirty();

    eprintln!("🔧 BPE Training Configuration");
    if let Some(sp) = schema_path {
        eprintln!("  Schema: {}", sp);
    }
    if let Some(bv) = explicit_base_vocab {
        eprintln!("  Base vocab: {} (explicit)", bv);
    }
    if let Some(enc_path) = encoded_path {
        eprintln!("  Pre-encoded: {} (fast mode)", enc_path);
    } else if let Some(inst_pattern) = instances_pattern {
        eprintln!(
            "  Instances: {} (will encode during training)",
            inst_pattern
        );
    }
    if let Some(tv) = target_vocab {
        eprintln!("  Target vocab: {}", tv);
    }
    if let Some(tal) = target_avg_length {
        eprintln!("  Target avg length: {:.1}", tal);
    }
    if let Some(mm) = max_merges_deprecated {
        eprintln!("  Max merges: {} (deprecated)", mm);
    }
    if schema_ids.is_empty() {
        eprintln!(
            "  Reserved special tokens: {} (BOS, EOS) + PAD (single-schema)",
            reserved_special_tokens
        );
    } else {
        eprintln!(
            "  Reserved special tokens: {} (BOS, EOS, {} schema ids: {}) + PAD",
            reserved_special_tokens,
            schema_ids.len(),
            schema_ids.join(", ")
        );
    }
    eprintln!("  Output: {}", output_path);
    eprintln!();

    // Step 1: Compile schema (if provided) or use explicit base vocab
    let (base_vocab_size, name, code_and_grammar) = if let Some(sp) = schema_path {
        eprintln!("📖 Loading schema...");
        let schema_content = fs::read_to_string(sp)?;
        let schema: Value = serde_json::from_str(&schema_content)?;

        let schema_name_str = if let Some(n) = schema_name {
            n.clone()
        } else {
            Path::new(sp)
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("schema")
                .to_string()
        };

        eprintln!("🔨 Compiling schema '{}'...", schema_name_str);
        let mut compiler = TctCompiler::new(schema, "TestType")?;

        compiler = compiler.with_optimization_level(optimization_level);
        if decimal_numbers {
            compiler = compiler.with_decimal_number_encoding();
        }
        eprintln!("  ⚙️  Using optimization level: {}", optimization_level);

        let grammar = compiler.compile()?;
        eprintln!("🔧 Generating TCT tokenizer code...");
        let code = grammar.generate_code()?;
        eprintln!("✅ Schema compiled successfully");
        eprintln!();

        let bv = grammar.base_vocab;

        (bv, schema_name_str, Some((code, grammar)))
    } else {
        // No schema - use explicit, auto-detected, or default base vocab
        let n = schema_name.cloned().unwrap_or_else(|| "bpe".to_string());
        if explicit_base_vocab.is_some() {
            eprintln!("📊 Using explicit base vocabulary: {}", final_base_vocab);
        }
        // auto_base_vocab and default already printed their messages during detection
        eprintln!();
        (final_base_vocab, n, None)
    };

    // Step 2: Load token sequences
    let loaded_corpus = if let Some(enc_path) = encoded_path {
        eprintln!("📦 Loading pre-encoded token sequences from: {}", enc_path);
        let file = File::open(enc_path)?;
        let reader = BufReader::new(file);
        let mut sequences = Vec::new();
        for (i, line) in reader.lines().enumerate() {
            let line = line?;
            if line.trim().is_empty() {
                continue;
            }
            let tokens: Vec<u32> = serde_json::from_str(&line)
                .map_err(|e| format!("Failed to parse line {}: {}", i + 1, e))?;
            validate_token_sequence_below_base_vocab(
                &tokens,
                base_vocab_size,
                &format!("encoded input {enc_path} line {}", i + 1),
                i + 1,
            )?;
            sequences.push(tokens);
            if (i + 1) % 10000 == 0 {
                eprintln!("  Loaded {} sequences...", i + 1);
            }
        }
        eprintln!(
            "  ✅ Loaded {} sequences from pre-encoded file",
            sequences.len()
        );
        eprintln!();
        let provenance = token_corpus_provenance_for_encoded(&sequences, &mut timings);
        LoadedTokenCorpus {
            sequences,
            provenance,
        }
    } else {
        // Traditional path: encode instances during training (requires schema)
        let (code, grammar) = code_and_grammar
            .as_ref()
            .ok_or_else(|| input_error("schema required for --instances mode"))?;
        let inst_pattern = instances_pattern
            .ok_or_else(|| input_error("--instances required if --encoded not provided"))?;
        let code_string = code.to_string();
        load_or_encode_instances(
            InstanceLoadConfig {
                instances_pattern: inst_pattern,
                code: &code_string,
                root_type_name: &grammar.root_type_name,
                base_vocab_size,
                sample_size,
                cache_path,
                output_path,
            },
            &mut timings,
        )?
    };
    let token_corpus_provenance = loaded_corpus.provenance;
    let token_sequences = loaded_corpus.sequences;

    // Step 2b: Load parallel role bitmaps when structure-only merges are requested.
    let structure_only_roles: Option<Vec<Vec<u8>>> = if structure_only_merges {
        let roles_path = roles_path_arg
            .clone()
            .or_else(|| {
                encoded_path.and_then(|path| {
                    let encoded = Path::new(path);
                    let sibling = encoded.with_extension("roles.jsonl");
                    if sibling.exists() {
                        return Some(sibling.display().to_string());
                    }
                    // Prefer all.roles.jsonl when --encoded points at all.jsonl.
                    encoded.parent().map(|parent| {
                        let candidate = parent.join("all.roles.jsonl");
                        if candidate.exists() {
                            candidate.display().to_string()
                        } else {
                            sibling.display().to_string()
                        }
                    })
                })
            })
            .ok_or_else(|| {
                input_error(
                    "--structure-only-merges requires --roles <path> or all.roles.jsonl beside --encoded",
                )
            })?;
        eprintln!(
            "🎭 Loading structure-only role bitmaps from: {}",
            roles_path
        );
        let roles = load_roles_jsonl(&roles_path)?;
        if roles.len() != token_sequences.len() {
            return Err(input_error(format!(
                "roles file has {} sequences but encoded tokens have {}",
                roles.len(),
                token_sequences.len()
            )));
        }
        for (index, (tokens, role_seq)) in token_sequences.iter().zip(roles.iter()).enumerate() {
            if tokens.len() != role_seq.len() {
                return Err(input_error(format!(
                    "roles length mismatch at sequence {}: roles {} != tokens {}",
                    index + 1,
                    role_seq.len(),
                    tokens.len()
                )));
            }
        }
        eprintln!("  ✅ Loaded {} aligned role sequences", roles.len());
        eprintln!();
        Some(roles)
    } else {
        if roles_path_arg.is_some() {
            eprintln!("⚠️  --roles ignored without --structure-only-merges");
        }
        None
    };

    // Step 3: Token sequences are ready for BPE training
    eprintln!("📊 Token sequences prepared for incremental BPE training");
    eprintln!("  {} sequences ready", token_sequences.len());
    if structure_only_roles.is_some() {
        eprintln!("  Structure-only merges: enabled (value payload pairs forbidden)");
    }
    eprintln!();

    eprintln!("📊 Vocabulary allocation:");
    eprintln!(
        "  Base vocabulary (from Grammar): {} tokens",
        base_vocab_size
    );

    // Parse checkpoints if specified
    let checkpoint_sizes: Option<Vec<usize>> = checkpoints_str.map(|s| {
        s.split(',')
            .filter_map(|part| part.trim().parse::<usize>().ok())
            .collect()
    });

    // Calculate max_merges based on stopping criterion. Requested vocabulary
    // sizes refer to generated VOCAB_SIZE, which includes the PAD token and any
    // reserved model special-token slots (BOS / EOS / per-schema id tokens).
    let max_merges = if let Some(ref sizes) = checkpoint_sizes {
        // Use largest checkpoint size for merge generation
        let max_checkpoint = *sizes.iter().max().unwrap_or(&0);
        let bpe_start = bpe_start_token(base_vocab_size) as usize;
        let min_vocab_size = generated_vocab_size(bpe_start, 0, reserved_special_tokens);
        let merges_needed =
            merge_count_for_generated_vocab(max_checkpoint, bpe_start, reserved_special_tokens);

        if merges_needed == 0 {
            eprintln!(
                "⚠️  Warning: max checkpoint {} <= minimum VOCAB_SIZE {}, no merges will be generated",
                max_checkpoint, min_vocab_size
            );
            0
        } else {
            eprintln!(
                "🎯 Checkpoint training: max vocab {} (bpe_start: {} + pad: {} + reserved: {} + merges: {})",
                max_checkpoint, bpe_start, PAD_TOKEN_COUNT, reserved_special_tokens, merges_needed
            );
            eprintln!("  Checkpoint sizes: {:?}", sizes);
            merges_needed
        }
    } else if let Some(target) = target_vocab {
        let bpe_start = bpe_start_token(base_vocab_size) as usize;
        let min_vocab_size = generated_vocab_size(bpe_start, 0, reserved_special_tokens);
        let merges_needed =
            merge_count_for_generated_vocab(target, bpe_start, reserved_special_tokens);

        if merges_needed == 0 {
            eprintln!(
                "⚠️  Warning: target vocab {} <= minimum VOCAB_SIZE {}, no merges will be generated",
                target, min_vocab_size
            );
            0
        } else {
            eprintln!(
                "🎯 Target vocab: {} (bpe_start: {} + pad: {} + reserved: {} + merges: {})",
                target, bpe_start, PAD_TOKEN_COUNT, reserved_special_tokens, merges_needed
            );
            merges_needed
        }
    } else if let Some(target_avg_length) = target_avg_length {
        // When targeting avg length, allow up to 100k merges (will stop early)
        eprintln!(
            "🎯 Target avg length: {:.1} (will stop when reached)",
            target_avg_length
        );
        100_000
    } else {
        let mm = max_merges_deprecated
            .ok_or_else(|| input_error("--max-merges required when no other stop criterion set"))?;
        eprintln!("🎯 Max merges: {} (deprecated parameter)", mm);
        mm
    };

    // Step 6: Generate merges using incremental simulation (nanochat-style)
    // This simulates the actual encoding process: apply each merge to all sequences,
    // then recalculate pair frequencies on the updated sequences
    eprintln!("🔄 Generating merges with incremental simulation...");
    eprintln!("  This ensures optimal merge selection by simulating encoding");
    if let Some(tal) = target_avg_length {
        eprintln!("  Stopping when avg sequence length <= {:.1}", tal);
    }

    // Capture instance count before moving token_sequences
    let num_training_instances = token_sequences.len();

    let training_start = Instant::now();
    let merges = train_bpe_incremental_with_options(
        token_sequences,
        max_merges,
        base_vocab_size,
        BpeTrainingOptions {
            target_avg_length,
            verbose,
            structure_only_roles,
        },
        report_training_progress,
    );
    timings.training_wall_time_ns = elapsed_ns_u64(training_start);
    eprintln!("  Generated {} merges", merges.len());

    let bpe_start = bpe_start_token(base_vocab_size) as usize;
    let total_vocab = generated_vocab_size(bpe_start, merges.len(), reserved_special_tokens);

    eprintln!(
        "  Final vocabulary: {} tokens (base: {} + merges: {} + pad: {} + reserved: {})",
        total_vocab,
        base_vocab_size,
        merges.len(),
        PAD_TOKEN_COUNT,
        reserved_special_tokens
    );
    eprintln!();

    // Step 7 & 8: Write output(s)
    let checkpoint_sizes_for_metadata = checkpoint_sizes.clone();
    let training_date = training_date()?;
    let mut output_file_paths = Vec::new();
    if let Some(sizes) = checkpoint_sizes {
        // Write multiple checkpoint files
        eprintln!("💾 Writing {} checkpoint files...", sizes.len());

        // Determine output base path (remove extension if present)
        let output_base = if output_path.ends_with(".json") {
            &output_path[..output_path.len() - 5]
        } else {
            output_path.as_str()
        };

        for &checkpoint_size in &sizes {
            let bpe_start = bpe_start_token(base_vocab_size) as usize;
            let merges_for_checkpoint = merge_count_for_generated_vocab(
                checkpoint_size,
                bpe_start,
                reserved_special_tokens,
            )
            .min(merges.len());

            let checkpoint_merges = merges
                .iter()
                .take(merges_for_checkpoint)
                .cloned()
                .collect::<Vec<_>>();

            let merge_table = BpeMergeTable {
                schema: name.clone(),
                base_vocab_size,
                merges: checkpoint_merges.clone(),
                metadata: BpeMetadata {
                    trained_on_instances: num_training_instances,
                    training_date: training_date.clone(),
                    max_merges: checkpoint_merges.len(),
                    dict_size: None,
                },
            };

            let checkpoint_path = format!("{}-{}.json", output_base, checkpoint_size);
            validate_merge_table_for_output(&merge_table, &checkpoint_path)?;
            let output_json = serde_json::to_string_pretty(&merge_table)?;
            fs::write(&checkpoint_path, output_json)?;
            output_file_paths.push(checkpoint_path.clone());
            let checkpoint_total_vocab =
                generated_vocab_size(bpe_start, checkpoint_merges.len(), reserved_special_tokens);
            eprintln!(
                "  ✅ {}: {} merges (vocab: {})",
                checkpoint_path,
                checkpoint_merges.len(),
                checkpoint_total_vocab
            );
        }

        eprintln!();
        eprintln!("✅ BPE training complete with {} checkpoints!", sizes.len());
        eprintln!("  Schema: {}", name);
        eprintln!("  Base vocabulary: {} tokens", base_vocab_size);
        eprintln!("  Training instances: {}", num_training_instances);
    } else {
        // Write single output file (original behavior)
        let merge_table = BpeMergeTable {
            schema: name.clone(),
            base_vocab_size,
            merges: merges.clone(),
            metadata: BpeMetadata {
                trained_on_instances: num_training_instances,
                training_date: training_date.clone(),
                max_merges: merges.len(),
                dict_size: None,
            },
        };

        eprintln!("💾 Writing merge table to: {}", output_path);
        validate_merge_table_for_output(&merge_table, output_path)?;
        let output_json = serde_json::to_string_pretty(&merge_table)?;
        fs::write(output_path, output_json)?;
        output_file_paths.push(output_path.to_string());

        eprintln!();
        eprintln!("✅ BPE training complete!");
        eprintln!("  Schema: {}", name);
        eprintln!("  Base vocabulary: {} tokens", base_vocab_size);
        eprintln!("  Merges generated: {}", merge_table.merges.len());
        eprintln!(
            "  Training instances: {}",
            merge_table.metadata.trained_on_instances
        );
    }

    let provenance_path = provenance_path_for_output(output_path);
    let input_files =
        input_file_provenances(schema_path, instances_pattern, encoded_path, &mut timings)?;
    let input_corpus_fingerprint = input_corpus_fingerprint(&input_files, &mut timings);
    let output_files = output_file_paths
        .iter()
        .map(|path| measure_provenance_hashing(&mut timings, || file_provenance(path)))
        .collect::<Result<Vec<_>, _>>()?;
    let provenance = BpeTrainingProvenance {
        command,
        schema_name: name,
        instances_pattern: instances_pattern.cloned(),
        encoded_input: encoded_path.cloned(),
        cache_path: cache_path.cloned(),
        schema_file: optional_file_provenance(schema_path.map(|path| path.as_str()), &mut timings)?,
        encoded_input_file: optional_file_provenance(
            encoded_path.map(|path| path.as_str()),
            &mut timings,
        )?,
        prepare_data_metadata_file: prepare_data_metadata_path
            .as_ref()
            .map(|path| measure_provenance_hashing(&mut timings, || file_provenance(path)))
            .transpose()?,
        cache_file: optional_file_provenance(cache_path.map(|path| path.as_str()), &mut timings)?,
        input_files,
        input_corpus_fingerprint,
        token_corpus: token_corpus_provenance,
        output_files,
        base_vocab_size,
        base_vocab_source,
        optimization_level: schema_path.map(|_| optimization_level.to_string()),
        target_vocab,
        target_avg_length,
        schema_ids: schema_ids.clone(),
        reserved_special_tokens,
        max_merges_deprecated,
        checkpoints: checkpoint_sizes_for_metadata,
        sample_size,
        trained_on_instances: num_training_instances,
        generated_merges: merges.len(),
        timings,
        provenance_path: provenance_path.clone(),
        created_at: chrono::Utc::now().to_rfc3339(),
        tct_version: env!("CARGO_PKG_VERSION").to_string(),
        repo_commit: source_repo_commit,
        repo_dirty: source_repo_dirty,
    };
    fs::write(&provenance_path, serde_json::to_string_pretty(&provenance)?)?;
    eprintln!("  Provenance: {}", provenance_path);

    Ok(())
}

fn validate_merge_table_for_output(
    merge_table: &BpeMergeTable,
    output_path: &str,
) -> Result<(), Box<dyn Error>> {
    merge_table.validate().map_err(|error| {
        input_error(format!(
            "invalid BPE merge table for {}: {}",
            output_path, error
        ))
    })
}

fn required_arg<'a, T>(matches: &'a ArgMatches, name: &str) -> Result<&'a T, Box<dyn Error>>
where
    T: Clone + Send + Sync + 'static,
{
    matches.get_one::<T>(name).ok_or_else(|| {
        input_error(format!(
            "missing required or defaulted CLI argument {name:?}"
        ))
    })
}

fn input_error(message: impl Into<String>) -> Box<dyn Error> {
    std::io::Error::new(std::io::ErrorKind::InvalidInput, message.into()).into()
}

// REMOVED: detect_base_vocab_size() was buggy because counting unique observed
// tokens can undercount the reserved generated vocabulary. Use schema metadata
// or explicit --base-vocab instead.

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generated_vocab_size_helpers_include_pad() {
        assert_eq!(merge_count_for_generated_vocab(260, 257, 0), 2);
        assert_eq!(generated_vocab_size(257, 2, 0), 260);
    }

    #[test]
    fn test_generated_vocab_size_helpers_saturate_below_minimum() {
        assert_eq!(merge_count_for_generated_vocab(257, 257, 0), 0);
        assert_eq!(merge_count_for_generated_vocab(258, 257, 0), 0);
        assert_eq!(generated_vocab_size(257, 0, 0), 258);
    }

    #[test]
    fn test_generated_vocab_size_helpers_with_wide_base_vocab() {
        let bpe_start = bpe_start_token(1024) as usize;

        assert_eq!(bpe_start, 1024);
        assert_eq!(merge_count_for_generated_vocab(8192, bpe_start, 0), 7167);
        assert_eq!(generated_vocab_size(bpe_start, 7167, 0), 8192);
    }

    #[test]
    fn test_generated_vocab_size_helpers_reserve_special_tokens() {
        // Multi-schema budget: base 257 + PAD 1 + reserved 5 (BOS, EOS, 3 schema ids).
        // A final model vocab of 1024 must leave room for all of them.
        let reserved = 5;
        assert_eq!(merge_count_for_generated_vocab(1024, 257, reserved), 761);
        assert_eq!(generated_vocab_size(257, 761, reserved), 1024);
        // Round-trip across the checkpoint set used for the multi-schema model.
        for &final_vocab in &[512usize, 1024, 2048] {
            let merges = merge_count_for_generated_vocab(final_vocab, 257, reserved);
            assert_eq!(generated_vocab_size(257, merges, reserved), final_vocab);
        }
    }

    #[test]
    fn token_sequence_fingerprint_is_order_sensitive() {
        let original = token_sequence_fingerprint_inner(&[vec![1, 2, 3], vec![4, 5]]);
        let same = token_sequence_fingerprint_inner(&[vec![1, 2, 3], vec![4, 5]]);
        let reordered = token_sequence_fingerprint_inner(&[vec![4, 5], vec![1, 2, 3]]);

        assert_eq!(original, same);
        assert_ne!(original, reordered);
        assert_eq!(original.len(), 64);
    }

    #[test]
    fn provenance_hashing_timer_accumulates_elapsed_work() {
        let mut timings = BpeTrainingTimings::default();
        let digest = measure_provenance_hashing(&mut timings, || {
            input_corpus_fingerprint_inner(&[FileProvenance {
                path: "sample.json".to_string(),
                bytes: 5,
                sha256: "0".repeat(64),
            }])
        });

        assert_eq!(digest.len(), 64);
        assert!(timings.provenance_hashing_ns > 0);
    }
}
