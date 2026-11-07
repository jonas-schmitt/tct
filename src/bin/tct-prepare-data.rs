//! Unified data preparation for TCT training pipelines
//!
//! This binary prepares encoded token sequences for both BPE training and LLM training.
//! It performs encoding ONCE and creates train/validate splits from the WHOLE corpus.
//!
//! # Features
//! - Multiple source patterns (combine different data directories)
//! - Random train/validate split from entire corpus (not sequential)
//! - Deterministic shuffling with seed for reproducibility
//! - `--preserve-order` flag to skip shuffling for exact source traceability
//! - Source file traceability via `sources.jsonl` output
//! - Parallel encoding for performance
//! - Rich metadata for provenance tracking
//! - JSONL output format (compatible with PyTorch DataLoader)
//!
//! # Usage
//! ```bash
//! tct-prepare-data \
//!   --schema schemas/popular/kubernetes.json \
//!   --sources "~/data/k8s-split-*/*.json" \
//!   --sources "~/data/k8s-new/*.json" \
//!   --output ~/data/k8s-encoded/ \
//!   --train-ratio 0.9 \
//!   --seed 42
//! ```
//!
//! # Output Structure
//! ```text
//! output/
//! ├── metadata.json      # Schema, sources, split info, encoding config
//! ├── train.jsonl        # Training token sequences (for LLM training)
//! ├── validate.jsonl     # Validation token sequences (for LLM validation)
//! ├── all.jsonl          # All sequences combined (for BPE training)
//! ├── sources.jsonl      # Source file mapping (idx -> source path, tokens)
//! └── stats.json         # Token counts, sequence lengths, vocab usage
//! ```

use chrono::Utc;
use clap::{Arg, ArgAction, ArgMatches, Command, value_parser};
use rand::SeedableRng;
use rand::seq::SliceRandom;
use rayon::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::error::Error;
use std::fs::{self, File};
use std::io::{BufRead, BufReader, BufWriter, Write};
use std::path::PathBuf;
use std::sync::Arc;
use tct::TctCompiler;
use tct::bpe::{BpeMergeTable, encoder::SequentialBpeEncoder};
use tct::instance_encoder::{
    EncodeFailure, InstanceEncoder, UnencodableReport, expand_glob_pattern,
};
use tct::provenance::{FileProvenance, file_provenance};

/// Result of parallel encoding: (source paths, token sequences, structured failures)
/// Source paths are in same order as token sequences for traceability
type EncodeResult = Result<(Vec<PathBuf>, Vec<Vec<u32>>, Vec<EncodeFailure>), Box<dyn Error>>;

/// Metadata about the prepared dataset
#[derive(Debug, Serialize, Deserialize)]
struct DatasetMetadata {
    /// Exact command invocation used to create the dataset.
    command: Vec<String>,
    /// Encoding mode: "tct" or "utf8"
    mode: String,
    /// Schema file used for encoding (None for utf8 mode)
    schema: Option<String>,
    /// Base vocabulary size (257 for TCT, 256 for UTF-8)
    base_vocab_size: u32,
    /// Source glob patterns
    sources: Vec<String>,
    /// Total files found across all sources
    total_files: usize,
    /// Files successfully encoded
    encoded_files: usize,
    /// Files that failed encoding
    failed_files: usize,
    /// Number of sequences in training set
    train_count: usize,
    /// Number of sequences in validation set
    validate_count: usize,
    /// Training ratio (0.0 - 1.0)
    train_ratio: f64,
    /// Validation ratio (0.0 - 1.0)
    validate_ratio: f64,
    /// Random seed for reproducibility
    seed: u64,
    /// Whether order was preserved (no shuffling)
    preserve_order: bool,
    /// Whether source traceability file was generated
    has_source_traceability: bool,
    /// Optimization level used (only for TCT mode)
    optimization_level: Option<String>,
    /// BPE merges file (if used)
    bpe_merges: Option<String>,
    /// Schema file provenance (if used)
    schema_file: Option<FileProvenance>,
    /// Pre-encoded input file provenance (if used)
    encoded_input_file: Option<FileProvenance>,
    /// BPE merge file provenance (if used)
    bpe_merges_file: Option<FileProvenance>,
    /// Whether parallel role bitmaps were written (structure-only BPE input)
    #[serde(default)]
    has_role_bitmap: bool,
    /// Creation timestamp
    created_at: String,
    /// TCT version
    tct_version: String,
}

/// Statistics about the encoded dataset
#[derive(Debug, Serialize, Deserialize)]
struct DatasetStats {
    /// Total tokens across all sequences
    total_tokens: usize,
    /// Tokens in training set
    train_tokens: usize,
    /// Tokens in validation set
    validate_tokens: usize,
    /// Average tokens per sequence
    avg_tokens_per_sequence: f64,
    /// Minimum sequence length
    min_sequence_length: usize,
    /// Maximum sequence length
    max_sequence_length: usize,
    /// Sequence length percentiles
    percentiles: HashMap<String, usize>,
}

fn main() -> Result<(), Box<dyn Error>> {
    let command: Vec<String> = std::env::args().collect();
    let matches = Command::new("tct-prepare-data")
        .about("Prepare encoded token sequences for TCT training pipelines")
        .version("0.1.0")
        .arg(
            Arg::new("mode")
                .long("mode")
                .help("Encoding mode: 'tct' (schema-aware, default) or 'utf8' (raw bytes)")
                .value_parser(value_parser!(String))
                .default_value("tct"),
        )
        .arg(
            Arg::new("schema")
                .short('s')
                .long("schema")
                .help("JSON schema file (required for TCT mode)")
                .value_parser(value_parser!(String)),
        )
        .arg(
            Arg::new("sources")
                .long("sources")
                .help("Source glob patterns (can be specified multiple times)")
                .action(ArgAction::Append)
                .value_parser(value_parser!(String)),
        )
        .arg(
            Arg::new("encoded")
                .short('e')
                .long("encoded")
                .help("Pre-encoded JSONL file (reshuffle, or apply BPE with --bpe-merges)")
                .value_parser(value_parser!(String))
                .conflicts_with("sources"),
        )
        .arg(
            Arg::new("output")
                .short('o')
                .long("output")
                .help("Output directory for prepared data")
                .required(true)
                .value_parser(value_parser!(String)),
        )
        .arg(
            Arg::new("train-ratio")
                .long("train-ratio")
                .help("Ratio of data for training (default: 0.9)")
                .value_parser(value_parser!(f64))
                .default_value("0.9"),
        )
        .arg(
            Arg::new("validate-ratio")
                .long("validate-ratio")
                .help("Ratio of data for validation (default: 0.1)")
                .value_parser(value_parser!(f64))
                .default_value("0.1"),
        )
        .arg(
            Arg::new("seed")
                .long("seed")
                .help("Random seed for reproducibility (default: 42)")
                .value_parser(value_parser!(u64))
                .default_value("42"),
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
            Arg::new("max-vocab")
                .long("max-vocab")
                .help("Cap the raw structural vocabulary (minimum 257)")
                .value_parser(value_parser!(usize)),
        )
        .arg(
            Arg::new("bpe-merges")
                .long("bpe-merges")
                .help("BPE merge file (optional, for BPE-compressed encoding)")
                .value_parser(value_parser!(String)),
        )
        .arg(
            Arg::new("quiet")
                .short('q')
                .long("quiet")
                .help("Quiet mode - minimal output")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new("shuffle")
                .long("shuffle")
                .help("Shuffle sequences randomly (default: preserve source order for reproducibility)")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new("decimal-numbers")
                .long("decimal-numbers")
                .help("Digit-aligned decimal text encoding for unbounded i64/u64/f64 (encoding must match any --bpe-merges and downstream wheels)")
                .action(clap::ArgAction::SetTrue),
        )
        .arg(
            Arg::new("export-roles")
                .long("export-roles")
                .help(
                    "Write parallel role bitmaps (train.roles.jsonl / validate.roles.jsonl / \
                     all.roles.jsonl) aligned 1:1 with base-token sequences for structure-only \
                     BPE training. Requires TCT sources mode without --bpe-merges.",
                )
                .action(ArgAction::SetTrue),
        )
        .get_matches();

    let mode = required_arg::<String>(&matches, "mode")?;
    let schema_path = matches.get_one::<String>("schema");
    let sources: Option<Vec<&String>> = matches.get_many::<String>("sources").map(|v| v.collect());
    let encoded_path = matches.get_one::<String>("encoded");
    let output_dir = required_arg::<String>(&matches, "output")?;
    let train_ratio = *required_arg::<f64>(&matches, "train-ratio")?;
    let validate_ratio = *required_arg::<f64>(&matches, "validate-ratio")?;
    let seed = *required_arg::<u64>(&matches, "seed")?;
    let optimization_str = required_arg::<String>(&matches, "optimization")?;
    let optimization_level = tct::OptimizationLevel::parse_cli(optimization_str)
        .map_err(|error| format!("{}.", error))?;
    let max_vocab = matches.get_one::<usize>("max-vocab").copied();
    let decimal_numbers = matches.get_flag("decimal-numbers");
    let export_roles = matches.get_flag("export-roles");
    let bpe_merges = matches.get_one::<String>("bpe-merges");
    let quiet = matches.get_flag("quiet");
    let shuffle = matches.get_flag("shuffle");

    let is_utf8_mode = mode == "utf8";
    let is_encoded_mode = encoded_path.is_some();

    // Validate input mode
    if sources.is_none() && encoded_path.is_none() {
        eprintln!("Error: Must specify either --sources or --encoded");
        std::process::exit(1);
    }
    if export_roles {
        if is_utf8_mode || is_encoded_mode {
            eprintln!("Error: --export-roles requires TCT sources mode (not utf8 or --encoded)");
            std::process::exit(1);
        }
        if bpe_merges.is_some() {
            eprintln!("Error: --export-roles requires base-token sequences; omit --bpe-merges");
            std::process::exit(1);
        }
    }

    // Note: --encoded can be used with or without --bpe-merges
    // Without --bpe-merges: just reshuffle and split the data
    // With --bpe-merges: apply BPE to pre-encoded base tokens

    // Validate mode-specific requirements
    if !is_encoded_mode && !is_utf8_mode && schema_path.is_none() {
        eprintln!("Error: TCT mode requires --schema");
        std::process::exit(1);
    }
    // Validate ratios
    let total_ratio = train_ratio + validate_ratio;
    if total_ratio > 1.0 {
        eprintln!(
            "Error: train-ratio ({}) + validate-ratio ({}) = {} > 1.0",
            train_ratio, validate_ratio, total_ratio
        );
        std::process::exit(1);
    }

    if !quiet {
        eprintln!("TCT Data Preparation");
        eprintln!("====================");
        if is_encoded_mode {
            if bpe_merges.is_some() {
                eprintln!("Mode: Pre-encoded (applying BPE)");
            } else {
                eprintln!("Mode: Pre-encoded (reshuffle only)");
            }
            let path =
                encoded_path.ok_or_else(|| input_error("encoded mode requires --encoded"))?;
            eprintln!("Input: {}", path);
        } else {
            eprintln!(
                "Mode: {}",
                if is_utf8_mode {
                    "UTF-8 (raw bytes)"
                } else {
                    "TCT (schema-aware)"
                }
            );
            if let Some(sp) = schema_path {
                eprintln!("Schema: {}", sp);
            }
            if let Some(ref srcs) = sources {
                eprintln!("Sources:");
                for src in srcs {
                    eprintln!("  - {}", src);
                }
            }
        }
        eprintln!("Output: {}", output_dir);
        eprintln!(
            "Split: {:.0}% train / {:.0}% validate",
            train_ratio * 100.0,
            validate_ratio * 100.0
        );
        if shuffle {
            eprintln!("Shuffle: enabled (seed {})", seed);
        } else {
            eprintln!("Order: preserved (use --shuffle to randomize)");
        }
        if !is_utf8_mode && !is_encoded_mode {
            eprintln!("Optimization: {}", optimization_level);
        }
        if let Some(bpe) = bpe_merges {
            eprintln!("BPE merges: {}", bpe);
        }
        eprintln!();
    }

    // Variables for final output (filled by either encoded or sources path)
    let train_sequences: Vec<Vec<u32>>;
    let validate_sequences: Vec<Vec<u32>>;
    let train_source_paths: Vec<PathBuf>;
    let validate_source_paths: Vec<PathBuf>;
    let mut train_roles: Option<Vec<Vec<u8>>> = None;
    let mut validate_roles: Option<Vec<Vec<u8>>> = None;
    let all_failed: Vec<EncodeFailure>;
    let base_vocab_size: u32;
    let total_files: usize;

    if is_encoded_mode {
        // ================================================================
        // ENCODED MODE: Load pre-encoded tokens, optionally apply BPE
        // ================================================================
        let enc_path =
            encoded_path.ok_or_else(|| input_error("encoded mode requires --encoded"))?;

        // Load BPE encoder if provided
        let bpe_encoder: Option<SequentialBpeEncoder>;
        let bpe_vocab: u32;

        if let Some(bpe_path) = bpe_merges {
            if !quiet {
                eprintln!("Step 1: Loading BPE merge table...");
            }

            let merge_table = BpeMergeTable::from_file(bpe_path)?;
            bpe_vocab = merge_table.bpe_start() + merge_table.merges.len() as u32;
            // Use sequential encoder for much faster processing on large datasets
            bpe_encoder = Some(SequentialBpeEncoder::new(&merge_table, bpe_vocab));

            if !quiet {
                eprintln!("  Base vocab: {}", merge_table.base_vocab_size);
                eprintln!("  BPE merges: {}", merge_table.merges.len());
                eprintln!("  Total vocab: {}", bpe_vocab);
                eprintln!();
            }
        } else {
            if !quiet {
                eprintln!("Step 1: Reshuffle-only mode (no BPE)...");
                eprintln!();
            }
            bpe_encoder = None;
            bpe_vocab = 0; // Will be determined from sequences
        }

        if !quiet {
            eprintln!("Step 2: Loading pre-encoded sequences from {}...", enc_path);
        }

        let mut all_sequences = load_jsonl_sequences(enc_path)?;

        if !quiet {
            eprintln!("  Loaded {} sequences", all_sequences.len());
            eprintln!();
        }

        total_files = all_sequences.len();

        // Shuffle sequences if --shuffle flag is set
        if shuffle {
            if !quiet {
                eprintln!("Step 3: Shuffling with seed {}...", seed);
            }

            let mut rng = rand::rngs::StdRng::seed_from_u64(seed);
            all_sequences.shuffle(&mut rng);

            if !quiet {
                eprintln!("  Shuffled {} sequences", all_sequences.len());
                eprintln!();
            }
        } else {
            if !quiet {
                eprintln!("Step 3: Preserving order (no shuffle)...");
                eprintln!("  {} sequences in original order", all_sequences.len());
                eprintln!();
            }
        }

        // Calculate split points - assign all remaining to validation to avoid unused sequences
        let train_count = (all_sequences.len() as f64 * train_ratio) as usize;
        let validate_count = all_sequences.len() - train_count;

        if !quiet {
            eprintln!("Step 4: Split allocation:");
            eprintln!("  Training: {} sequences", train_count);
            eprintln!("  Validation: {} sequences", validate_count);
            eprintln!(
                "  Unused: {} sequences",
                all_sequences.len() - train_count - validate_count
            );
            eprintln!();
        }

        // Split sequences
        let train_base: Vec<Vec<u32>> = all_sequences[..train_count].to_vec();
        let validate_base: Vec<Vec<u32>> =
            all_sequences[train_count..train_count + validate_count].to_vec();

        // Apply BPE if encoder is provided, otherwise passthrough
        if let Some(ref encoder) = bpe_encoder {
            if !quiet {
                eprintln!("Step 5: Applying BPE to sequences...");
            }

            // Use parallel iteration for massive speedup on multi-core systems
            train_sequences = train_base
                .par_iter()
                .map(|seq| encoder.encode(seq))
                .collect();
            validate_sequences = validate_base
                .par_iter()
                .map(|seq| encoder.encode(seq))
                .collect();

            if !quiet {
                let train_base_tokens: usize = train_base.iter().map(|s| s.len()).sum();
                let train_bpe_tokens: usize = train_sequences.iter().map(|s| s.len()).sum();
                eprintln!(
                    "  Training: {} base tokens -> {} BPE tokens ({:.1}x compression)",
                    train_base_tokens,
                    train_bpe_tokens,
                    train_base_tokens as f64 / train_bpe_tokens as f64
                );
                eprintln!();
            }
            base_vocab_size = bpe_vocab;
        } else {
            if !quiet {
                eprintln!("Step 5: Passthrough (no BPE applied)...");
                eprintln!();
            }
            train_sequences = train_base;
            validate_sequences = validate_base;

            // Determine base vocab from max token ID in sequences
            let max_token = all_sequences
                .iter()
                .flat_map(|s| s.iter())
                .max()
                .copied()
                .unwrap_or(255);
            base_vocab_size = max_token + 1;
        }

        all_failed = Vec::new();

        // No source paths available in encoded mode
        train_source_paths = Vec::new();
        validate_source_paths = Vec::new();

        // Create output directory
        fs::create_dir_all(output_dir)?;
    } else {
        // ================================================================
        // SOURCES MODE: Encode from JSON files
        // ================================================================

        // Step 1: Collect all file paths from all sources
        if !quiet {
            eprintln!("Step 1: Collecting source files...");
        }

        let sources_list = sources
            .as_ref()
            .ok_or_else(|| input_error("sources required when not using --encoded"))?;

        let mut all_paths: Vec<PathBuf> = Vec::new();
        for pattern in sources_list {
            let paths = expand_glob_pattern(pattern)?;
            if !quiet {
                eprintln!("  {} -> {} files", pattern, paths.len());
            }
            all_paths.extend(paths);
        }

        if all_paths.is_empty() {
            eprintln!("Error: No files found matching source patterns");
            std::process::exit(1);
        }

        total_files = all_paths.len();
        if !quiet {
            eprintln!("  Total: {} files", total_files);
            eprintln!();
        }

        // Step 2: Shuffle if --shuffle flag is set
        if shuffle {
            if !quiet {
                eprintln!("Step 2: Shuffling with seed {}...", seed);
            }

            let mut rng = rand::rngs::StdRng::seed_from_u64(seed);
            all_paths.shuffle(&mut rng);

            if !quiet {
                eprintln!("  Shuffled {} files", all_paths.len());
                eprintln!();
            }
        } else {
            if !quiet {
                eprintln!("Step 2: Preserving order (no shuffle)...");
                eprintln!("  {} files in original order", all_paths.len());
                eprintln!();
            }
        }

        // Step 3: Calculate split points - assign all remaining to validation to avoid unused files
        let train_count = (all_paths.len() as f64 * train_ratio) as usize;
        let validate_count = all_paths.len() - train_count;

        let train_paths: Vec<PathBuf> = all_paths[..train_count].to_vec();
        let validate_paths: Vec<PathBuf> = all_paths[train_count..].to_vec();

        if !quiet {
            eprintln!("Step 3: Split allocation:");
            eprintln!("  Training: {} files", train_paths.len());
            eprintln!("  Validation: {} files", validate_paths.len());
            eprintln!(
                "  Unused: {} files",
                all_paths.len() - train_count - validate_count
            );
            eprintln!();
        }

        // Step 4: Setup encoder (TCT mode) or prepare for raw bytes (UTF-8 mode)
        let encoder: Option<Arc<InstanceEncoder>>;
        let utf8_bpe_encoder: Option<SequentialBpeEncoder>;

        if is_utf8_mode {
            if !quiet {
                eprintln!("Step 4: UTF-8 mode (raw bytes)...");
                eprintln!("  Base vocabulary: 256 (byte values)");
            }

            // Load BPE encoder for UTF-8 mode if provided
            if let Some(bpe_path) = bpe_merges {
                let merge_table = BpeMergeTable::from_file(bpe_path)?;
                let bpe_vocab_size = merge_table.bpe_start() + merge_table.merges.len() as u32;
                if !quiet {
                    eprintln!(
                        "  BPE merges: {} ({} merges)",
                        bpe_path,
                        merge_table.merges.len()
                    );
                    eprintln!("  BPE vocabulary: {} tokens", bpe_vocab_size);
                }
                utf8_bpe_encoder = Some(SequentialBpeEncoder::new(&merge_table, bpe_vocab_size));
                base_vocab_size = bpe_vocab_size;
            } else {
                utf8_bpe_encoder = None;
                base_vocab_size = 256;
            }

            if !quiet {
                eprintln!();
            }
            encoder = None;
        } else {
            if !quiet {
                eprintln!("Step 4: Compiling schema...");
            }

            let sp = schema_path.ok_or_else(|| input_error("schema required for TCT mode"))?;
            let schema_content = fs::read_to_string(sp)?;
            let schema: serde_json::Value = serde_json::from_str(&schema_content)?;

            let mut compiler = TctCompiler::new(schema, "PreparedType")?;

            if let Some(bpe_path) = bpe_merges {
                compiler = compiler.with_bpe_merges(bpe_path)?;
                if !quiet {
                    eprintln!("  BPE merges loaded: {}", bpe_path);
                }
            }

            compiler = compiler.with_optimization_level(optimization_level);
            if let Some(max_vocab) = max_vocab {
                compiler = compiler.with_max_vocabulary(max_vocab)?;
            }
            if decimal_numbers {
                compiler = compiler.with_decimal_number_encoding();
            }

            let grammar = compiler.compile()?;
            let code = grammar.generate_code()?;
            base_vocab_size = grammar.base_vocab;
            encoder = Some(Arc::new(InstanceEncoder::new(
                &code.to_string(),
                &grammar.root_type_name,
            )?));

            if !quiet {
                eprintln!("  Schema compiled successfully");
                eprintln!("  Base vocabulary: {} tokens", base_vocab_size);
                eprintln!();
            }
            utf8_bpe_encoder = None; // Not used in TCT mode
        }

        // Step 5: Create output directory
        fs::create_dir_all(output_dir)?;

        // Step 6: Encode training set
        if !quiet {
            eprintln!(
                "Step 5: Encoding training set ({} files)...",
                train_paths.len()
            );
        }

        let (train_src_paths, train_seqs, train_failed_paths) = if is_utf8_mode {
            encode_files_utf8(&train_paths, utf8_bpe_encoder.as_ref(), quiet)?
        } else {
            let encoder = encoder
                .as_ref()
                .ok_or_else(|| input_error("TCT mode requires a compiled encoder"))?;
            encode_files_parallel(&train_paths, encoder, quiet)?
        };

        if !quiet {
            eprintln!(
                "  Encoded: {} sequences ({} failed)",
                train_seqs.len(),
                train_failed_paths.len()
            );
            eprintln!();
        }

        // Step 7: Encode validation set
        if !quiet {
            eprintln!(
                "Step 6: Encoding validation set ({} files)...",
                validate_paths.len()
            );
        }

        let (validate_src_paths, validate_seqs, validate_failed_paths) = if is_utf8_mode {
            encode_files_utf8(&validate_paths, utf8_bpe_encoder.as_ref(), quiet)?
        } else {
            let encoder = encoder
                .as_ref()
                .ok_or_else(|| input_error("TCT mode requires a compiled encoder"))?;
            encode_files_parallel(&validate_paths, encoder, quiet)?
        };

        if !quiet {
            eprintln!(
                "  Encoded: {} sequences ({} failed)",
                validate_seqs.len(),
                validate_failed_paths.len()
            );
            eprintln!();
        }

        if export_roles {
            let encoder = encoder.as_ref().ok_or_else(|| {
                input_error("TCT mode requires a compiled encoder for --export-roles")
            })?;
            if !quiet {
                eprintln!("Step 6b: Exporting parallel role bitmaps...");
            }
            train_roles = Some(encoder.roles_batch(&train_seqs)?);
            validate_roles = Some(encoder.roles_batch(&validate_seqs)?);
            if !quiet {
                eprintln!(
                    "  Roles: {} train + {} validate sequences",
                    train_seqs.len(),
                    validate_seqs.len()
                );
                eprintln!();
            }
        }

        train_sequences = train_seqs;
        train_source_paths = train_src_paths;
        validate_sequences = validate_seqs;
        validate_source_paths = validate_src_paths;

        // Combine structured failures from both sets
        let mut failed = train_failed_paths;
        failed.extend(validate_failed_paths);
        all_failed = failed;
    }

    // Step 8: Write JSONL files
    if !quiet {
        eprintln!("Step 7: Writing output files...");
    }

    let train_path = format!("{}/train.jsonl", output_dir);
    let validate_path = format!("{}/validate.jsonl", output_dir);
    let all_path = format!("{}/all.jsonl", output_dir);

    write_jsonl(&train_path, &train_sequences)?;
    write_jsonl(&validate_path, &validate_sequences)?;

    // Write combined file for BPE training (BPE should train on full dataset)
    {
        let file = File::create(&all_path)?;
        let mut writer = BufWriter::new(file);
        for seq in &train_sequences {
            let line = serde_json::to_string(seq)?;
            writeln!(writer, "{}", line)?;
        }
        for seq in &validate_sequences {
            let line = serde_json::to_string(seq)?;
            writeln!(writer, "{}", line)?;
        }
        writer.flush()?;
    }

    let has_role_bitmap = train_roles.is_some() || validate_roles.is_some();
    if has_role_bitmap {
        let train_roles_path = format!("{}/train.roles.jsonl", output_dir);
        let validate_roles_path = format!("{}/validate.roles.jsonl", output_dir);
        let all_roles_path = format!("{}/all.roles.jsonl", output_dir);
        if let Some(ref roles) = train_roles {
            write_roles_jsonl(&train_roles_path, roles)?;
        } else {
            write_roles_jsonl(&train_roles_path, &[])?;
        }
        if let Some(ref roles) = validate_roles {
            write_roles_jsonl(&validate_roles_path, roles)?;
        } else {
            write_roles_jsonl(&validate_roles_path, &[])?;
        }
        {
            let file = File::create(&all_roles_path)?;
            let mut writer = BufWriter::new(file);
            if let Some(ref roles) = train_roles {
                for seq in roles {
                    writeln!(writer, "{}", serde_json::to_string(seq)?)?;
                }
            }
            if let Some(ref roles) = validate_roles {
                for seq in roles {
                    writeln!(writer, "{}", serde_json::to_string(seq)?)?;
                }
            }
            writer.flush()?;
        }
    }

    // Write failed paths + structured unencodable inventory when any encode fails
    if !all_failed.is_empty() {
        let failed_path = format!("{}/failed.txt", output_dir);
        let mut failed_content = String::new();
        for failure in &all_failed {
            failed_content.push_str(&format!("{}\n", failure.path.display()));
        }
        fs::write(&failed_path, failed_content)?;

        let attempted = train_sequences.len() + validate_sequences.len() + all_failed.len();
        let report = UnencodableReport::from_failures_default(attempted, &all_failed);
        let report_path = format!("{}/unencodable-report.json", output_dir);
        report.write_json(&report_path)?;
    }

    // Write source traceability file (only for SOURCES mode where we have paths)
    let has_source_paths = !train_source_paths.is_empty() || !validate_source_paths.is_empty();
    if has_source_paths {
        let sources_path = format!("{}/sources.jsonl", output_dir);
        let file = File::create(&sources_path)?;
        let mut writer = BufWriter::new(file);

        // Write train sources
        for (idx, (path, seq)) in train_source_paths
            .iter()
            .zip(train_sequences.iter())
            .enumerate()
        {
            let entry = serde_json::json!({
                "idx": idx,
                "split": "train",
                "source": path.display().to_string(),
                "file": file_provenance(path)?,
                "tokens": seq.len()
            });
            writeln!(writer, "{}", entry)?;
        }

        // Write validate sources (continue indexing from train)
        let train_count = train_source_paths.len();
        for (idx, (path, seq)) in validate_source_paths
            .iter()
            .zip(validate_sequences.iter())
            .enumerate()
        {
            let entry = serde_json::json!({
                "idx": train_count + idx,
                "split": "validate",
                "source": path.display().to_string(),
                "file": file_provenance(path)?,
                "tokens": seq.len()
            });
            writeln!(writer, "{}", entry)?;
        }
        writer.flush()?;
    }

    if !quiet {
        eprintln!(
            "  train.jsonl:    {} sequences (for LLM training)",
            train_sequences.len()
        );
        eprintln!(
            "  validate.jsonl: {} sequences (for LLM validation)",
            validate_sequences.len()
        );
        eprintln!(
            "  all.jsonl:      {} sequences (for BPE training)",
            train_sequences.len() + validate_sequences.len()
        );
        if !all_failed.is_empty() {
            let attempted = train_sequences.len() + validate_sequences.len() + all_failed.len();
            let report = UnencodableReport::from_failures_default(attempted, &all_failed);
            eprintln!(
                "  failed.txt:     {} paths (encoding failures)",
                all_failed.len()
            );
            eprintln!("  unencodable-report.json: {}", report.summary_line());
        }
        if has_source_paths {
            eprintln!(
                "  sources.jsonl:  {} entries (source traceability)",
                train_source_paths.len() + validate_source_paths.len()
            );
        }
        if has_role_bitmap {
            eprintln!("  *.roles.jsonl:  parallel role bitmaps (structure-only BPE)");
        }
        eprintln!();
    }

    // Step 9: Calculate statistics
    let train_tokens: usize = train_sequences.iter().map(|s| s.len()).sum();
    let validate_tokens: usize = validate_sequences.iter().map(|s| s.len()).sum();
    let total_tokens = train_tokens + validate_tokens;

    let all_sequences: Vec<&Vec<u32>> = train_sequences
        .iter()
        .chain(validate_sequences.iter())
        .collect();
    let total_sequences = all_sequences.len();

    let mut lengths: Vec<usize> = all_sequences.iter().map(|s| s.len()).collect();
    lengths.sort();

    let min_len = *lengths.first().unwrap_or(&0);
    let max_len = *lengths.last().unwrap_or(&0);
    let avg_len = if total_sequences > 0 {
        total_tokens as f64 / total_sequences as f64
    } else {
        0.0
    };

    let mut percentiles = HashMap::new();
    if !lengths.is_empty() {
        percentiles.insert("p50".to_string(), lengths[lengths.len() / 2]);
        percentiles.insert("p90".to_string(), lengths[lengths.len() * 9 / 10]);
        percentiles.insert("p95".to_string(), lengths[lengths.len() * 95 / 100]);
        percentiles.insert("p99".to_string(), lengths[lengths.len() * 99 / 100]);
    }

    let stats = DatasetStats {
        total_tokens,
        train_tokens,
        validate_tokens,
        avg_tokens_per_sequence: avg_len,
        min_sequence_length: min_len,
        max_sequence_length: max_len,
        percentiles,
    };

    let schema_file = schema_path
        .map(|path| file_provenance(path.as_str()))
        .transpose()?;
    let encoded_input_file = encoded_path
        .map(|path| file_provenance(path.as_str()))
        .transpose()?;
    let bpe_merges_file = bpe_merges
        .map(|path| file_provenance(path.as_str()))
        .transpose()?;

    // Step 10: Write metadata
    let metadata = DatasetMetadata {
        command,
        mode: if is_encoded_mode {
            if bpe_merges.is_some() {
                "encoded-bpe".to_string()
            } else {
                "reshuffle".to_string()
            }
        } else if is_utf8_mode {
            "utf8".to_string()
        } else {
            "tct".to_string()
        },
        schema: schema_path.cloned(),
        base_vocab_size,
        sources: if is_encoded_mode {
            let path =
                encoded_path.ok_or_else(|| input_error("encoded mode requires --encoded"))?;
            vec![path.to_string()]
        } else {
            sources
                .as_ref()
                .map(|s| s.iter().map(|p| p.to_string()).collect())
                .unwrap_or_default()
        },
        total_files,
        encoded_files: train_sequences.len() + validate_sequences.len(),
        failed_files: all_failed.len(),
        train_count: train_sequences.len(),
        validate_count: validate_sequences.len(),
        train_ratio,
        validate_ratio,
        seed,
        preserve_order: !shuffle,
        has_source_traceability: has_source_paths,
        optimization_level: if is_utf8_mode || is_encoded_mode {
            None
        } else {
            Some(optimization_level.to_string())
        },
        bpe_merges: bpe_merges.cloned(),
        schema_file,
        encoded_input_file,
        bpe_merges_file,
        has_role_bitmap,
        created_at: Utc::now().to_rfc3339(),
        tct_version: env!("CARGO_PKG_VERSION").to_string(),
    };

    let metadata_path = format!("{}/metadata.json", output_dir);
    let stats_path = format!("{}/stats.json", output_dir);

    fs::write(&metadata_path, serde_json::to_string_pretty(&metadata)?)?;
    fs::write(&stats_path, serde_json::to_string_pretty(&stats)?)?;

    if !quiet {
        eprintln!("Step 8: Writing metadata...");
        eprintln!("  metadata.json");
        eprintln!("  stats.json");
        eprintln!();
    }

    // Final summary
    if !quiet {
        eprintln!("========================================");
        eprintln!("Data Preparation Complete!");
        eprintln!("========================================");
        eprintln!("Output directory: {}", output_dir);
        eprintln!();
        eprintln!("Files:");
        eprintln!(
            "  train.jsonl:    {:>8} sequences (LLM training)",
            train_sequences.len()
        );
        eprintln!(
            "  validate.jsonl: {:>8} sequences (LLM validation)",
            validate_sequences.len()
        );
        eprintln!(
            "  all.jsonl:      {:>8} sequences (BPE training)",
            train_sequences.len() + validate_sequences.len()
        );
        eprintln!();
        eprintln!("Tokens:");
        eprintln!("  Training:   {:>12}", format_number(train_tokens));
        eprintln!("  Validation: {:>12}", format_number(validate_tokens));
        eprintln!("  Total:      {:>12}", format_number(total_tokens));
        eprintln!();
        eprintln!("Sequence lengths:");
        eprintln!("  Min: {}", min_len);
        eprintln!("  Avg: {:.1}", avg_len);
        eprintln!("  Max: {}", max_len);
        eprintln!(
            "  P50/P90/P99: {}/{}/{}",
            stats.percentiles.get("p50").unwrap_or(&0),
            stats.percentiles.get("p90").unwrap_or(&0),
            stats.percentiles.get("p99").unwrap_or(&0)
        );
        eprintln!();
        if all_failed.is_empty() {
            eprintln!("Encoding failures: 0");
        } else {
            let attempted = train_sequences.len() + validate_sequences.len() + all_failed.len();
            let report = UnencodableReport::from_failures_default(attempted, &all_failed);
            eprintln!("Encoding failures: {}", report.summary_line());
        }
        eprintln!();
        eprintln!("Usage:");
        eprintln!("  BPE training: tct-train-bpe --encoded all.jsonl ...");
        if has_role_bitmap {
            eprintln!(
                "  Structure-only BPE: tct-train-bpe --encoded all.jsonl --structure-only-merges ..."
            );
        }
        eprintln!("  LLM training: Use train.jsonl for training, validate.jsonl for validation");
    } else {
        // Quiet mode: just print key stats as JSON
        println!(
            "{{\"train_sequences\":{},\"validate_sequences\":{},\"all_sequences\":{},\"total_tokens\":{}}}",
            train_sequences.len(),
            validate_sequences.len(),
            train_sequences.len() + validate_sequences.len(),
            total_tokens
        );
    }

    Ok(())
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

/// Encode files using efficient batch mode (single subprocess)
///
/// This uses the encode-batch command which streams file paths via stdin
/// and reads JSONL output, eliminating subprocess-per-file overhead.
fn encode_files_parallel(
    paths: &[PathBuf],
    encoder: &Arc<InstanceEncoder>,
    quiet: bool,
) -> EncodeResult {
    if !quiet {
        eprintln!("  Using efficient batch mode (single subprocess)...");
    }

    // Use efficient batch encoding - single subprocess for all files
    let (sequences, failures) = encoder.encode_batch_efficient_with_failures(paths)?;

    // Build source paths list: paths that succeeded (not in failures)
    // The batch encoder maintains order - sequences[i] corresponds to paths[i]
    // for non-failed paths
    let failed_set: std::collections::HashSet<_> =
        failures.iter().map(|failure| &failure.path).collect();
    let source_paths: Vec<PathBuf> = paths
        .iter()
        .filter(|p| !failed_set.contains(p))
        .cloned()
        .collect();

    Ok((source_paths, sequences, failures))
}

/// Encode files as raw UTF-8 bytes (for standard BPE training)
///
/// Each byte of the file becomes a token (0-255).
/// If BPE encoder is provided, tokens are compressed with BPE merges.
fn encode_files_utf8(
    paths: &[PathBuf],
    bpe_encoder: Option<&SequentialBpeEncoder>,
    quiet: bool,
) -> EncodeResult {
    use rayon::prelude::*;

    if !quiet {
        if bpe_encoder.is_some() {
            eprintln!("  Reading raw file bytes and applying BPE...");
        } else {
            eprintln!("  Reading raw file bytes...");
        }
    }

    let results: Vec<Result<Vec<u32>, PathBuf>> = paths
        .par_iter()
        .map(|path| match fs::read(path) {
            Ok(bytes) => {
                // Convert each byte to a u32 token
                let tokens: Vec<u32> = bytes.iter().map(|&b| b as u32).collect();
                // Apply BPE compression if encoder is provided
                if let Some(encoder) = bpe_encoder {
                    Ok(encoder.encode(&tokens))
                } else {
                    Ok(tokens)
                }
            }
            Err(_) => Err(path.clone()),
        })
        .collect();

    let mut source_paths = Vec::new();
    let mut sequences = Vec::new();
    let mut failed = Vec::new();

    // Iterate with index to track which paths succeeded
    for (i, result) in results.into_iter().enumerate() {
        match result {
            Ok(tokens) => {
                source_paths.push(paths[i].clone());
                sequences.push(tokens);
            }
            Err(path) => failed.push(EncodeFailure::new(
                path,
                "utf8 read or encode failed".to_string(),
            )),
        }
    }

    Ok((source_paths, sequences, failed))
}

/// Write token sequences to JSONL file
fn write_jsonl(path: &str, sequences: &[Vec<u32>]) -> Result<(), Box<dyn Error>> {
    let file = File::create(path)?;
    let mut writer = BufWriter::new(file);

    for seq in sequences {
        let line = serde_json::to_string(seq)?;
        writeln!(writer, "{}", line)?;
    }

    writer.flush()?;
    Ok(())
}

/// Write parallel role bitmaps (u8 per base token) to JSONL.
fn write_roles_jsonl(path: &str, sequences: &[Vec<u8>]) -> Result<(), Box<dyn Error>> {
    let file = File::create(path)?;
    let mut writer = BufWriter::new(file);

    for seq in sequences {
        let line = serde_json::to_string(seq)?;
        writeln!(writer, "{}", line)?;
    }

    writer.flush()?;
    Ok(())
}

/// Format large numbers with commas
fn format_number(n: usize) -> String {
    let s = n.to_string();
    let mut result = String::new();
    for (i, c) in s.chars().rev().enumerate() {
        if i > 0 && i % 3 == 0 {
            result.push(',');
        }
        result.push(c);
    }
    result.chars().rev().collect()
}

/// Load token sequences from JSONL file
fn load_jsonl_sequences(path: &str) -> Result<Vec<Vec<u32>>, Box<dyn Error>> {
    let file = File::open(path)?;
    let reader = BufReader::new(file);
    let mut sequences = Vec::new();

    for (i, line) in reader.lines().enumerate() {
        let line = line?;
        if line.trim().is_empty() {
            continue;
        }
        let tokens: Vec<u32> = serde_json::from_str(&line)
            .map_err(|e| format!("Failed to parse line {}: {}", i + 1, e))?;
        sequences.push(tokens);
    }

    Ok(sequences)
}
