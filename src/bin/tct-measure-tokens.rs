//! Measure actual token counts for TCT tokenizers with optional BPE compression
//!
//! This binary encodes JSON instances with TCT tokenizers and counts:
//! 1. Base token count (structural encoding without BPE)
//! 2. BPE compressed token count (if merge file provided)
//!
//! Usage:
//! ```bash
//! tct-measure-tokens \
//!   --schema schemas/popular/github-workflow.json \
//!   --instances data/*.json \
//!   [--bpe-merges merges.json]
//! ```

use clap::{Arg, ArgMatches, Command, value_parser};
use serde::Serialize;
use std::error::Error;
use std::fs;
use std::io::{BufRead, BufReader};
use std::time::Instant;
use tct::TctCompiler;
use tct::bpe::{BpeMergeTable, encoder::BpeDecoder, encoder::SequentialBpeEncoder};
use tct::instance_encoder::{
    BatchAllocationMetrics, BatchDecodePrefixMetrics, BatchEncodeHotPathMetrics,
    BatchEncodePhaseMetrics, BatchEncodeValidationPhaseMetrics, BatchPrimaryRuntimeMetrics,
    InstanceEncoder, expand_glob_pattern, token_sequence_checksum,
};
use tct::provenance::{FileProvenance, file_provenance};

#[derive(Debug, Serialize)]
struct MeasurementProvenance {
    command: Vec<String>,
    schema: Option<String>,
    instances_pattern: Option<String>,
    encoded_input: Option<String>,
    bpe_merges: Option<String>,
    schema_file: Option<FileProvenance>,
    input_files: Vec<FileProvenance>,
    encoded_input_file: Option<FileProvenance>,
    bpe_merges_file: Option<FileProvenance>,
    measured_files_output: Option<String>,
    measured_files_output_file: Option<FileProvenance>,
    base_vocab_size: u32,
    base_vocab_source: String,
    optimization_level: Option<String>,
    created_at: String,
    tct_version: String,
}

#[derive(Debug, Serialize)]
struct MeasurementReport {
    tokens: usize,
    instances: usize,
    characters: usize,
    has_bpe: bool,
    warmup_iterations: u32,
    base_vocab_size: u32,
    generated_vocab_size: u32,
    pad_token: u32,
    bpe_base_vocab_size: Option<u32>,
    bpe_merges_count: Option<usize>,
    token_length_stats: TokenLengthStats,
    timings: MeasurementTimings,
    allocation_metrics: AllocationMetrics,
    generated_batch_runtime_metrics: GeneratedBatchRuntimeMetrics,
    same_process_bpe_metrics: SameProcessBpeMetrics,
    generated_bpe_metrics: GeneratedBpeMetrics,
    generated_encode_phase_metrics: GeneratedEncodePhaseMetrics,
    generated_encode_hot_path_metrics: GeneratedEncodeHotPathMetrics,
    generated_encode_validation_phase_metrics: GeneratedEncodeValidationPhaseMetrics,
    generated_decode_prefix_metrics: GeneratedDecodePrefixMetrics,
    provenance: MeasurementProvenance,
}

#[derive(Debug, Serialize)]
struct TokenLengthStats {
    records: usize,
    total_tokens: usize,
    min_tokens: Option<usize>,
    max_tokens: Option<usize>,
    average_tokens: Option<f64>,
    p50_tokens: Option<usize>,
    p95_tokens: Option<usize>,
    p99_tokens: Option<usize>,
    context_window_tokens: usize,
    context_window_pass_count: usize,
    context_window_overflow_count: usize,
    context_window_pass_rate: Option<f64>,
    percentile_method: &'static str,
}

#[derive(Debug, Serialize)]
struct MeasurementTimings {
    schema_load_time_ms: u64,
    compiler_setup_time_ms: u64,
    schema_compile_time_ms: u64,
    code_generation_time_ms: u64,
    instance_discovery_time_ms: u64,
    generated_encoder_build_time_ms: u64,
    generated_warmup_wall_time_ms: u64,
    generated_encode_wall_time_ms: u64,
    generated_decode_wall_time_ms: u64,
    total_wall_time_ms: u64,
    note: String,
}

#[derive(Debug, Serialize)]
struct AllocationMetrics {
    available: bool,
    generated_encode_allocations: Option<u64>,
    generated_encode_allocated_bytes: Option<u64>,
    generated_decode_allocations: Option<u64>,
    generated_decode_allocated_bytes: Option<u64>,
    note: String,
}

#[derive(Debug, Serialize)]
struct GeneratedBatchRuntimeMetrics {
    available: bool,
    generated_encode_child_wall_time_ms: Option<u64>,
    generated_decode_child_wall_time_ms: Option<u64>,
    generated_encode_parent_wall_time_ms: Option<u64>,
    generated_decode_parent_wall_time_ms: Option<u64>,
    generated_encode_parent_transport_delta_wall_time_ms: Option<u64>,
    generated_decode_parent_transport_delta_wall_time_ms: Option<u64>,
    generated_encode_parent_stdin_write_ns: Option<u64>,
    generated_decode_parent_stdin_write_ns: Option<u64>,
    generated_encode_parent_stdout_read_parse_ns: Option<u64>,
    generated_decode_parent_stdout_read_parse_ns: Option<u64>,
    generated_encode_parent_stdout_json_parse_ns: Option<u64>,
    generated_decode_parent_stdout_json_parse_ns: Option<u64>,
    generated_encode_parent_wait_ns: Option<u64>,
    generated_decode_parent_wait_ns: Option<u64>,
    note: String,
}

#[derive(Debug, Serialize)]
struct SameProcessBpeMetrics {
    available: bool,
    input_tokens: Option<usize>,
    output_tokens: Option<usize>,
    compression_ratio: Option<f64>,
    warmup_wall_time_ms: Option<u64>,
    encode_wall_time_ms: Option<u64>,
    decode_wall_time_ms: Option<u64>,
    decode_round_trip_ok: Option<bool>,
    merges_count: Option<usize>,
    bpe_vocab_size: Option<u32>,
    note: String,
}

#[derive(Debug, Serialize)]
struct GeneratedBpeMetrics {
    available: bool,
    input_tokens: Option<usize>,
    output_tokens: Option<usize>,
    compression_ratio: Option<f64>,
    encode_wall_time_ms: Option<u64>,
    decode_wall_time_ms: Option<u64>,
    round_trip_ok: Option<bool>,
    generated_bpe_encode_allocations: Option<u64>,
    generated_bpe_encode_allocated_bytes: Option<u64>,
    generated_bpe_decode_allocations: Option<u64>,
    generated_bpe_decode_allocated_bytes: Option<u64>,
    note: String,
}

#[derive(Debug, Serialize)]
struct GeneratedEncodePhaseMetrics {
    available: bool,
    records: Option<u64>,
    failures: Option<u64>,
    profile_wall_time_ns: Option<u64>,
    path_input_ns: Option<u64>,
    json_read_parse_ns: Option<u64>,
    base_encode_ns: Option<u64>,
    bpe_encode_ns: Option<u64>,
    output_serialization_ns: Option<u64>,
    stdout_write_ns: Option<u64>,
    token_sequences_match_primary: Option<bool>,
    failed_paths_match_primary: Option<bool>,
    note: String,
}

#[derive(Debug, Serialize)]
struct GeneratedEncodeHotPathMetrics {
    available: bool,
    records: Option<u64>,
    failures: Option<u64>,
    token_count: Option<u64>,
    token_checksum: Option<u64>,
    profile_wall_time_ns: Option<u64>,
    path_input_ns: Option<u64>,
    json_read_parse_ns: Option<u64>,
    base_encode_ns: Option<u64>,
    bpe_encode_ns: Option<u64>,
    generated_encode_hot_path_allocations: Option<u64>,
    generated_encode_hot_path_allocated_bytes: Option<u64>,
    token_count_matches_primary: Option<bool>,
    token_checksum_matches_primary: Option<bool>,
    failed_count_matches_primary: Option<bool>,
    note: String,
}

#[derive(Debug, Serialize)]
struct GeneratedEncodeValidationPhaseMetrics {
    available: bool,
    records: Option<u64>,
    failures: Option<u64>,
    token_count: Option<u64>,
    token_checksum: Option<u64>,
    profile_wall_time_ns: Option<u64>,
    path_input_ns: Option<u64>,
    file_read_ns: Option<u64>,
    json_value_parse_ns: Option<u64>,
    serde_from_value_ns: Option<u64>,
    base_encode_ns: Option<u64>,
    bpe_encode_ns: Option<u64>,
    token_count_matches_primary: Option<bool>,
    token_checksum_matches_primary: Option<bool>,
    failed_count_matches_primary: Option<bool>,
    note: String,
}

#[derive(Debug, Serialize)]
struct GeneratedDecodePrefixMetrics {
    available: bool,
    warmup_iterations: Option<u32>,
    records: Option<u64>,
    prefix_cuts: Option<u64>,
    complete_prefixes: Option<u64>,
    partial_prefixes: Option<u64>,
    failed_prefixes: Option<u64>,
    full_sequences_complete: Option<u64>,
    consumed_tokens: Option<u64>,
    total_prefix_tokens: Option<u64>,
    fields_decoded_sum: Option<u64>,
    partial_fields_reported: Option<u64>,
    profile_wall_time_ns: Option<u64>,
    token_input_ns: Option<u64>,
    bpe_decode_ns: Option<u64>,
    decode_prefix_ns: Option<u64>,
    note: String,
}

const DEFAULT_CONTEXT_WINDOW_TOKENS: usize = 2048;

fn main() -> Result<(), Box<dyn Error>> {
    let total_start = Instant::now();
    let command: Vec<String> = std::env::args().collect();
    let matches = Command::new("tct-measure-tokens")
        .about("Measure actual token counts for TCT tokenizers")
        .version("0.1.0")
        .arg(
            Arg::new("schema")
                .short('s')
                .long("schema")
                .help("JSON schema file")
                .value_parser(value_parser!(String)),
        )
        .arg(
            Arg::new("instances")
                .short('i')
                .long("instances")
                .help("Instance JSON files (glob pattern)")
                .value_parser(value_parser!(String)),
        )
        .arg(
            Arg::new("encoded")
                .long("encoded")
                .help("Pre-encoded token JSONL file for same-process BPE measurement")
                .value_parser(value_parser!(String)),
        )
        .arg(
            Arg::new("bpe-merges")
                .long("bpe-merges")
                .help("BPE merge file (optional)")
                .value_parser(value_parser!(String)),
        )
        .arg(
            Arg::new("quiet")
                .short('q')
                .long("quiet")
                .help("Quiet mode - only output CSV result")
                .action(clap::ArgAction::SetTrue),
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
            Arg::new("output-files")
                .long("output-files")
                .help("Output file to write list of successfully measured files (one per line)")
                .value_parser(value_parser!(String)),
        )
        .arg(
            Arg::new("warmup-iterations")
                .long("warmup-iterations")
                .help("Generated encode/decode warmup iterations before measured timing")
                .value_parser(value_parser!(u32))
                .default_value("0"),
        )
        .arg(
            Arg::new("track-allocations")
                .long("track-allocations")
                .help("Track allocation counts in generated subprocess batch windows and compatible generated sidecars")
                .action(clap::ArgAction::SetTrue),
        )
        .arg(
            Arg::new("generated-bpe-metrics")
                .long("generated-bpe-metrics")
                .help("Measure generated BPE-only encode/decode over token JSONL in schema-backed BPE runs")
                .action(clap::ArgAction::SetTrue),
        )
        .arg(
            Arg::new("generated-encode-phase-metrics")
                .long("generated-encode-phase-metrics")
                .help("Run an opt-in generated encode sidecar pass that reports child-process phase timings")
                .action(clap::ArgAction::SetTrue),
        )
        .arg(
            Arg::new("generated-encode-hot-path-metrics")
                .long("generated-encode-hot-path-metrics")
                .help("Run an opt-in generated encode sidecar pass without token JSONL output")
                .action(clap::ArgAction::SetTrue),
        )
        .arg(
            Arg::new("generated-encode-validation-phase-metrics")
                .long("generated-encode-validation-phase-metrics")
                .help("Run an opt-in generated encode sidecar that splits file read, JSON Value parse, and typed serde validation")
                .action(clap::ArgAction::SetTrue),
        )
        .arg(
            Arg::new("generated-decode-prefix-metrics")
                .long("generated-decode-prefix-metrics")
                .help("Run an opt-in generated decode_prefix sidecar over full and truncated measured token sequences")
                .action(clap::ArgAction::SetTrue),
        )
        .arg(
            Arg::new("cleanup-generated-helper")
                .long("cleanup-generated-helper")
                .help("Remove the successful temporary generated helper project before exit; failed helper builds remain preserved for inspection")
                .action(clap::ArgAction::SetTrue),
        )
        .arg(
            Arg::new("decimal-numbers")
                .long("decimal-numbers")
                .help("Digit-aligned decimal text encoding for unbounded i64/u64/f64 (encoding must match any --bpe-merges and downstream wheels)")
                .action(clap::ArgAction::SetTrue),
        )
        .get_matches();

    let schema_path = matches.get_one::<String>("schema");
    let instances_pattern = matches.get_one::<String>("instances");
    let encoded_path = matches.get_one::<String>("encoded");
    let bpe_merges = matches.get_one::<String>("bpe-merges");
    let quiet = matches.get_flag("quiet");
    let output_files = matches.get_one::<String>("output-files");
    let optimization_str = required_arg::<String>(&matches, "optimization")?;
    let optimization_level = tct::OptimizationLevel::parse_cli(optimization_str)
        .map_err(|error| format!("{}.", error))?;
    let max_vocab = matches.get_one::<usize>("max-vocab").copied();
    let decimal_numbers = matches.get_flag("decimal-numbers");
    let warmup_iterations = *required_arg::<u32>(&matches, "warmup-iterations")?;
    let track_allocations = matches.get_flag("track-allocations");
    let generated_bpe_metrics_requested = matches.get_flag("generated-bpe-metrics");
    let generated_encode_phase_metrics_requested =
        matches.get_flag("generated-encode-phase-metrics");
    let generated_encode_hot_path_metrics_requested =
        matches.get_flag("generated-encode-hot-path-metrics");
    let generated_encode_validation_phase_metrics_requested =
        matches.get_flag("generated-encode-validation-phase-metrics");
    let generated_decode_prefix_metrics_requested =
        matches.get_flag("generated-decode-prefix-metrics");
    let cleanup_generated_helper_requested = matches.get_flag("cleanup-generated-helper");

    if let Some(encoded_path) = encoded_path {
        if schema_path.is_some() || instances_pattern.is_some() {
            return Err("Use either --encoded or --schema/--instances, not both".into());
        }
        let Some(bpe_merges) = bpe_merges else {
            return Err("--encoded requires --bpe-merges for same-process BPE measurement".into());
        };
        if output_files.is_some() {
            return Err("--output-files is only supported with --schema/--instances".into());
        }
        if track_allocations {
            return Err(
                "--track-allocations measures generated helper subprocesses and is not supported with --encoded"
                    .into(),
            );
        }
        if generated_bpe_metrics_requested {
            return Err("--generated-bpe-metrics requires --schema/--instances".into());
        }
        if generated_encode_phase_metrics_requested {
            return Err("--generated-encode-phase-metrics requires --schema/--instances".into());
        }
        if generated_encode_hot_path_metrics_requested {
            return Err("--generated-encode-hot-path-metrics requires --schema/--instances".into());
        }
        if generated_encode_validation_phase_metrics_requested {
            return Err(
                "--generated-encode-validation-phase-metrics requires --schema/--instances".into(),
            );
        }
        if generated_decode_prefix_metrics_requested {
            return Err("--generated-decode-prefix-metrics requires --schema/--instances".into());
        }
        if cleanup_generated_helper_requested {
            return Err("--cleanup-generated-helper requires --schema/--instances".into());
        }

        return run_encoded_bpe_measurement(
            total_start,
            command,
            encoded_path,
            bpe_merges,
            warmup_iterations,
            quiet,
        );
    }

    let Some(schema_path) = schema_path else {
        return Err("Missing --schema; use --schema with --instances or use --encoded".into());
    };
    let Some(instances_pattern) = instances_pattern else {
        return Err("Missing --instances; use --instances with --schema or use --encoded".into());
    };
    if generated_bpe_metrics_requested && bpe_merges.is_none() {
        return Err("--generated-bpe-metrics requires --bpe-merges".into());
    }
    if generated_encode_phase_metrics_requested && quiet {
        return Err("--generated-encode-phase-metrics requires JSON report output and is not supported with --quiet".into());
    }
    if generated_encode_hot_path_metrics_requested && quiet {
        return Err("--generated-encode-hot-path-metrics requires JSON report output and is not supported with --quiet".into());
    }
    if generated_encode_validation_phase_metrics_requested && quiet {
        return Err("--generated-encode-validation-phase-metrics requires JSON report output and is not supported with --quiet".into());
    }
    if generated_decode_prefix_metrics_requested && quiet {
        return Err("--generated-decode-prefix-metrics requires JSON report output and is not supported with --quiet".into());
    }

    // Load and compile schema
    let schema_load_start = Instant::now();
    let schema_content = fs::read_to_string(schema_path)?;
    let schema: serde_json::Value = serde_json::from_str(&schema_content)?;
    let schema_load_time_ms = elapsed_ms_u64(schema_load_start);

    let compiler_setup_start = Instant::now();
    let mut compiler = TctCompiler::new(schema, "TestType")?;

    // Apply BPE if merge file provided
    let has_bpe = if let Some(merge_path) = bpe_merges {
        compiler = compiler.with_bpe_merges(merge_path)?;
        if !quiet {
            eprintln!("🔧 BPE compression enabled from: {}", merge_path);
        }
        true
    } else {
        false
    };

    // Apply optimization configuration
    compiler = compiler.with_optimization_level(optimization_level);
    if let Some(max_vocab) = max_vocab {
        compiler = compiler.with_max_vocabulary(max_vocab)?;
    }
    if decimal_numbers {
        compiler = compiler.with_decimal_number_encoding();
    }
    if generated_decode_prefix_metrics_requested {
        compiler = compiler.with_streaming_encode();
    }
    let compiler_setup_time_ms = elapsed_ms_u64(compiler_setup_start);

    if !quiet {
        eprintln!("  ⚙️  Using optimization level: {}", optimization_level);
    }

    let schema_compile_start = Instant::now();
    let grammar = compiler.compile()?;
    let schema_compile_time_ms = elapsed_ms_u64(schema_compile_start);

    let code_generation_start = Instant::now();
    let code = grammar.generate_code()?;
    let code_generation_time_ms = elapsed_ms_u64(code_generation_start);

    // Load instances
    let instance_discovery_start = Instant::now();
    let instance_paths = expand_glob_pattern(instances_pattern)?;
    let instance_discovery_time_ms = elapsed_ms_u64(instance_discovery_start);
    if !quiet {
        eprintln!(
            "📂 Loaded {} instances from {}",
            instance_paths.len(),
            instances_pattern
        );
    }

    // Encode instances
    let generated_encoder_build_start = Instant::now();
    let encoder = if track_allocations {
        InstanceEncoder::new_with_allocation_metrics(&code.to_string(), &grammar.root_type_name)?
    } else {
        InstanceEncoder::new(&code.to_string(), &grammar.root_type_name)?
    };
    let generated_encoder_build_time_ms = elapsed_ms_u64(generated_encoder_build_start);

    let generated_warmup_wall_time_ms = if warmup_iterations == 0 {
        0
    } else {
        let generated_warmup_start = Instant::now();
        for _ in 0..warmup_iterations {
            let (warmup_token_sequences, warmup_failed_paths) =
                encoder.encode_batch_efficient(&instance_paths)?;
            if !quiet && !warmup_failed_paths.is_empty() {
                eprintln!(
                    "  ⚠️  Warning: warmup encode skipped {} instances",
                    warmup_failed_paths.len()
                );
            }

            let (_, warmup_failed_decodes) =
                encoder.decode_batch_efficient(&warmup_token_sequences)?;
            if !quiet && !warmup_failed_decodes.is_empty() {
                eprintln!(
                    "  ⚠️  Warning: warmup decode failed for {} token sequences",
                    warmup_failed_decodes.len()
                );
            }
        }
        elapsed_ms_u64(generated_warmup_start)
    };

    let generated_encode_start = Instant::now();
    let (token_sequences, failed_paths, generated_encode_runtime_metrics) = if track_allocations {
        let (token_sequences, failed_paths, metrics) =
            encoder.encode_batch_efficient_with_runtime_and_allocation_metrics(&instance_paths)?;
        let metrics = metrics.ok_or(
            "Generated encode runtime/allocation tracking was requested, but no metrics were emitted",
        )?;
        if metrics.allocation_metrics.is_none() {
            return Err(
                "Generated encode allocation tracking was requested, but no allocation metrics were emitted"
                    .into(),
            );
        }
        (token_sequences, failed_paths, Some(metrics))
    } else {
        let (token_sequences, failed_paths, metrics) =
            encoder.encode_batch_efficient_with_runtime_metrics(&instance_paths)?;
        (token_sequences, failed_paths, metrics)
    };
    let generated_encode_wall_time_ms = elapsed_ms_u64(generated_encode_start);
    let generated_encode_allocations =
        generated_encode_runtime_metrics.and_then(|metrics| metrics.allocation_metrics);

    if !quiet && !failed_paths.is_empty() {
        eprintln!(
            "  ⚠️  Warning: skipped {} instances that failed generated encoding",
            failed_paths.len()
        );
    }

    if token_sequences.is_empty() {
        return Err("No instances were successfully encoded".into());
    }

    let generated_encode_phase_metrics = if generated_encode_phase_metrics_requested {
        measure_generated_encode_phase_metrics(
            &encoder,
            &instance_paths,
            &token_sequences,
            &failed_paths,
            has_bpe,
        )?
    } else {
        unavailable_generated_encode_phase_metrics()
    };
    let generated_encode_hot_path_metrics = if generated_encode_hot_path_metrics_requested {
        measure_generated_encode_hot_path_metrics(
            &encoder,
            &instance_paths,
            &token_sequences,
            failed_paths.len(),
            has_bpe,
            track_allocations,
        )?
    } else {
        unavailable_generated_encode_hot_path_metrics()
    };
    let generated_encode_validation_phase_metrics =
        if generated_encode_validation_phase_metrics_requested {
            measure_generated_encode_validation_phase_metrics(
                &encoder,
                &instance_paths,
                &token_sequences,
                failed_paths.len(),
                has_bpe,
            )?
        } else {
            unavailable_generated_encode_validation_phase_metrics()
        };

    let generated_decode_start = Instant::now();
    let (_, failed_decodes, generated_decode_runtime_metrics) = if track_allocations {
        let (_, failed_decodes, metrics) =
            encoder.decode_batch_efficient_with_runtime_and_allocation_metrics(&token_sequences)?;
        let metrics = metrics.ok_or(
            "Generated decode runtime/allocation tracking was requested, but no metrics were emitted",
        )?;
        if metrics.allocation_metrics.is_none() {
            return Err(
                "Generated decode allocation tracking was requested, but no allocation metrics were emitted"
                    .into(),
            );
        }
        ((), failed_decodes, Some(metrics))
    } else {
        let (_, failed_decodes, metrics) =
            encoder.decode_batch_efficient_with_runtime_metrics(&token_sequences)?;
        ((), failed_decodes, metrics)
    };
    let generated_decode_wall_time_ms = elapsed_ms_u64(generated_decode_start);
    let generated_decode_allocations =
        generated_decode_runtime_metrics.and_then(|metrics| metrics.allocation_metrics);

    if !failed_decodes.is_empty() {
        return Err(format!(
            "Generated decode failed for {} measured token sequences",
            failed_decodes.len()
        )
        .into());
    }

    let generated_bpe_metrics = if generated_bpe_metrics_requested {
        measure_generated_bpe_metrics(
            &encoder,
            &token_sequences,
            warmup_iterations,
            track_allocations,
        )?
    } else {
        unavailable_generated_bpe_metrics()
    };
    let generated_decode_prefix_metrics = if generated_decode_prefix_metrics_requested {
        measure_generated_decode_prefix_metrics(
            &encoder,
            &token_sequences,
            warmup_iterations,
            has_bpe,
        )?
    } else {
        unavailable_generated_decode_prefix_metrics()
    };

    // Count tokens (these are already BPE-compressed if BPE was enabled)
    let token_count: usize = token_sequences.iter().map(|seq| seq.len()).sum();
    let instance_count = token_sequences.len();
    let measured_paths: Vec<_> = instance_paths
        .iter()
        .filter(|path| !failed_paths.contains(path))
        .cloned()
        .collect();

    let mut measured_files_output_file = None;
    // Write successfully measured files if output-files specified
    if let Some(output_path) = output_files {
        use std::io::Write;
        let mut output_file = std::fs::File::create(output_path)?;

        for path in &measured_paths {
            writeln!(output_file, "{}", path.display())?;
        }

        if !quiet {
            eprintln!(
                "📝 Wrote {} successfully measured files to {}",
                instance_count, output_path
            );
        }
        measured_files_output_file = Some(file_provenance(output_path)?);
    }

    if cleanup_generated_helper_requested {
        let generated_helper_project = encoder.project_dir().to_path_buf();
        encoder.cleanup_project_dir().map_err(|error| {
            format!(
                "Failed to clean up generated helper project at {}: {}",
                generated_helper_project.display(),
                error
            )
        })?;
        if !quiet {
            eprintln!(
                "Removed generated helper project: {}",
                generated_helper_project.display()
            );
        }
    }

    if quiet {
        // CSV output: tokens,instances,avg_tokens
        let avg_tokens = token_count as f64 / instance_count as f64;
        println!("{},{},{:.2}", token_count, instance_count, avg_tokens);
    } else {
        eprintln!("\n📊 Token Count Results:");
        if has_bpe {
            eprintln!("   BPE-compressed tokens: {}", token_count);
        } else {
            eprintln!("   Base tokens: {}", token_count);
        }
        eprintln!("   Instances: {}", instance_count);
        eprintln!(
            "   Avg tokens/instance: {:.1}",
            token_count as f64 / instance_count as f64
        );

        // Calculate character count for compression ratio
        let mut total_chars = 0;
        for path in &measured_paths {
            if let Ok(content) = fs::read_to_string(path) {
                total_chars += content.len();
            }
        }

        if total_chars > 0 {
            eprintln!("   Characters: {}", total_chars);
            eprintln!(
                "   Chars/token: {:.2}",
                total_chars as f64 / token_count as f64
            );
        }

        let input_files = measured_paths
            .iter()
            .map(file_provenance)
            .collect::<Result<Vec<_>, _>>()?;
        let bpe_merges_file = bpe_merges
            .map(|path| file_provenance(path.as_str()))
            .transpose()?;
        let bpe_merges_count = grammar
            .bpe_merge_table
            .as_ref()
            .map(|table| table.merges.len());
        let (generated_vocab_size, pad_token) =
            generated_vocab_layout(grammar.base_vocab, bpe_merges_count)?;
        let token_length_stats = token_length_stats(&token_sequences);
        let report = MeasurementReport {
            tokens: token_count,
            instances: instance_count,
            characters: total_chars,
            has_bpe,
            warmup_iterations,
            base_vocab_size: grammar.base_vocab,
            generated_vocab_size,
            pad_token,
            bpe_base_vocab_size: grammar
                .bpe_merge_table
                .as_ref()
                .map(|table| table.base_vocab_size),
            bpe_merges_count,
            token_length_stats,
            timings: MeasurementTimings {
                schema_load_time_ms,
                compiler_setup_time_ms,
                schema_compile_time_ms,
                code_generation_time_ms,
                instance_discovery_time_ms,
                generated_encoder_build_time_ms,
                generated_warmup_wall_time_ms,
                generated_encode_wall_time_ms,
                generated_decode_wall_time_ms,
                total_wall_time_ms: elapsed_ms_u64(total_start),
                note: measurement_timing_note(),
            },
            allocation_metrics: allocation_metrics(
                generated_encode_allocations,
                generated_decode_allocations,
            ),
            generated_batch_runtime_metrics: generated_batch_runtime_metrics(
                generated_encode_wall_time_ms,
                generated_decode_wall_time_ms,
                generated_encode_runtime_metrics,
                generated_decode_runtime_metrics,
            ),
            same_process_bpe_metrics: unavailable_same_process_bpe_metrics(),
            generated_bpe_metrics,
            generated_encode_phase_metrics,
            generated_encode_hot_path_metrics,
            generated_encode_validation_phase_metrics,
            generated_decode_prefix_metrics,
            provenance: MeasurementProvenance {
                command,
                schema: Some(schema_path.clone()),
                instances_pattern: Some(instances_pattern.clone()),
                encoded_input: None,
                bpe_merges: bpe_merges.cloned(),
                schema_file: Some(file_provenance(schema_path)?),
                input_files,
                encoded_input_file: None,
                bpe_merges_file,
                measured_files_output: output_files.cloned(),
                measured_files_output_file,
                base_vocab_size: grammar.base_vocab,
                base_vocab_source: "grammar".to_string(),
                optimization_level: Some(optimization_level.to_string()),
                created_at: chrono::Utc::now().to_rfc3339(),
                tct_version: env!("CARGO_PKG_VERSION").to_string(),
            },
        };
        println!("{}", serde_json::to_string_pretty(&report)?);
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

fn measurement_timing_note() -> String {
    "top-level generated encode/decode wall times, when present, are parent-observed full-contract envelopes over generated helper execution, including subprocess transport and JSONL report handling; use generated_encode_phase_metrics, generated_encode_hot_path_metrics, generated_encode_validation_phase_metrics, generated_batch_runtime_metrics, and generated_bpe_metrics for scoped attribution before citing raw hot-path behavior".to_string()
}

fn run_encoded_bpe_measurement(
    total_start: Instant,
    command: Vec<String>,
    encoded_path: &str,
    bpe_merges: &str,
    warmup_iterations: u32,
    quiet: bool,
) -> Result<(), Box<dyn Error>> {
    let input_load_start = Instant::now();
    let base_sequences = load_jsonl_sequences(encoded_path)?;
    let instance_discovery_time_ms = elapsed_ms_u64(input_load_start);
    if base_sequences.is_empty() {
        return Err("No token sequences were loaded from --encoded input".into());
    }

    let merge_load_start = Instant::now();
    let merge_table = BpeMergeTable::from_file(bpe_merges)?;
    let compiler_setup_time_ms = elapsed_ms_u64(merge_load_start);
    let bpe_vocab_size = merge_table
        .bpe_start()
        .checked_add(
            merge_table
                .merges
                .len()
                .try_into()
                .map_err(|_| "BPE merge count exceeds u32 token ID range")?,
        )
        .ok_or("BPE vocabulary size exceeds u32 token ID range")?;
    let encoder = SequentialBpeEncoder::new(&merge_table, bpe_vocab_size);
    let decoder = BpeDecoder::new(&merge_table, bpe_vocab_size);

    let generated_warmup_wall_time_ms = if warmup_iterations == 0 {
        0
    } else {
        let warmup_start = Instant::now();
        for _ in 0..warmup_iterations {
            let warmup_encoded = encode_bpe_sequences(&encoder, &base_sequences);
            let warmup_decoded = decode_bpe_sequences(&decoder, &warmup_encoded);
            if warmup_decoded != base_sequences {
                return Err("Same-process BPE warmup decode did not reproduce input tokens".into());
            }
        }
        elapsed_ms_u64(warmup_start)
    };

    let encode_start = Instant::now();
    let bpe_sequences = encode_bpe_sequences(&encoder, &base_sequences);
    let encode_wall_time_ms = elapsed_ms_u64(encode_start);

    let decode_start = Instant::now();
    let decoded_sequences = decode_bpe_sequences(&decoder, &bpe_sequences);
    let decode_wall_time_ms = elapsed_ms_u64(decode_start);
    if decoded_sequences != base_sequences {
        return Err("Same-process BPE decode did not reproduce input tokens".into());
    }

    let input_token_count = token_count(&base_sequences);
    let output_token_count = token_count(&bpe_sequences);
    let compression_ratio =
        (output_token_count > 0).then_some(input_token_count as f64 / output_token_count as f64);
    let instance_count = base_sequences.len();
    let (generated_vocab_size, pad_token) =
        generated_vocab_layout(merge_table.base_vocab_size, Some(merge_table.merges.len()))?;

    if quiet {
        let avg_tokens = output_token_count as f64 / instance_count as f64;
        println!(
            "{},{},{:.2}",
            output_token_count, instance_count, avg_tokens
        );
    } else {
        let token_length_stats = token_length_stats(&bpe_sequences);
        let report = MeasurementReport {
            tokens: output_token_count,
            instances: instance_count,
            characters: 0,
            has_bpe: true,
            warmup_iterations,
            base_vocab_size: merge_table.base_vocab_size,
            generated_vocab_size,
            pad_token,
            bpe_base_vocab_size: Some(merge_table.base_vocab_size),
            bpe_merges_count: Some(merge_table.merges.len()),
            token_length_stats,
            timings: MeasurementTimings {
                schema_load_time_ms: 0,
                compiler_setup_time_ms,
                schema_compile_time_ms: 0,
                code_generation_time_ms: 0,
                instance_discovery_time_ms,
                generated_encoder_build_time_ms: 0,
                generated_warmup_wall_time_ms,
                generated_encode_wall_time_ms: 0,
                generated_decode_wall_time_ms: 0,
                total_wall_time_ms: elapsed_ms_u64(total_start),
                note: measurement_timing_note(),
            },
            allocation_metrics: allocation_metrics(None, None),
            generated_batch_runtime_metrics: unavailable_generated_batch_runtime_metrics(),
            same_process_bpe_metrics: SameProcessBpeMetrics {
                available: true,
                input_tokens: Some(input_token_count),
                output_tokens: Some(output_token_count),
                compression_ratio,
                warmup_wall_time_ms: Some(generated_warmup_wall_time_ms),
                encode_wall_time_ms: Some(encode_wall_time_ms),
                decode_wall_time_ms: Some(decode_wall_time_ms),
                decode_round_trip_ok: Some(true),
                merges_count: Some(merge_table.merges.len()),
                bpe_vocab_size: Some(bpe_vocab_size),
                note: "same-process metrics isolate SequentialBpeEncoder and BpeDecoder over pre-encoded token sequences; allocation counters are not collected".to_string(),
            },
            generated_bpe_metrics: unavailable_generated_bpe_metrics(),
            generated_encode_phase_metrics: unavailable_generated_encode_phase_metrics(),
            generated_encode_hot_path_metrics: unavailable_generated_encode_hot_path_metrics(),
            generated_encode_validation_phase_metrics:
                unavailable_generated_encode_validation_phase_metrics(),
            generated_decode_prefix_metrics: unavailable_generated_decode_prefix_metrics(),
            provenance: MeasurementProvenance {
                command,
                schema: None,
                instances_pattern: None,
                encoded_input: Some(encoded_path.to_string()),
                bpe_merges: Some(bpe_merges.to_string()),
                schema_file: None,
                input_files: Vec::new(),
                encoded_input_file: Some(file_provenance(encoded_path)?),
                bpe_merges_file: Some(file_provenance(bpe_merges)?),
                measured_files_output: None,
                measured_files_output_file: None,
                base_vocab_size: merge_table.base_vocab_size,
                base_vocab_source: "bpe_merges".to_string(),
                optimization_level: None,
                created_at: chrono::Utc::now().to_rfc3339(),
                tct_version: env!("CARGO_PKG_VERSION").to_string(),
            },
        };
        println!("{}", serde_json::to_string_pretty(&report)?);
    }

    Ok(())
}

fn load_jsonl_sequences(path: &str) -> Result<Vec<Vec<u32>>, Box<dyn Error>> {
    let file = fs::File::open(path)?;
    let reader = BufReader::new(file);
    let mut sequences = Vec::new();

    for (index, line) in reader.lines().enumerate() {
        let line = line?;
        if line.trim().is_empty() {
            continue;
        }
        let tokens: Vec<u32> = serde_json::from_str(&line)
            .map_err(|error| format!("Failed to parse line {}: {}", index + 1, error))?;
        sequences.push(tokens);
    }

    Ok(sequences)
}

fn encode_bpe_sequences(encoder: &SequentialBpeEncoder, sequences: &[Vec<u32>]) -> Vec<Vec<u32>> {
    sequences
        .iter()
        .map(|sequence| encoder.encode(sequence))
        .collect()
}

fn decode_bpe_sequences(decoder: &BpeDecoder, sequences: &[Vec<u32>]) -> Vec<Vec<u32>> {
    sequences
        .iter()
        .map(|sequence| decoder.decode(sequence))
        .collect()
}

fn measure_generated_bpe_metrics(
    encoder: &InstanceEncoder,
    bpe_sequences: &[Vec<u32>],
    warmup_iterations: u32,
    track_allocations: bool,
) -> Result<GeneratedBpeMetrics, Box<dyn Error>> {
    let (base_sequences, failed_decodes, generated_bpe_decode_metrics) = if track_allocations {
        let (base_sequences, failed_decodes, metrics) =
            encoder.bpe_decode_batch_efficient_with_metrics(bpe_sequences, warmup_iterations)?;
        let metrics = metrics
            .ok_or("Generated BPE decode metrics were requested, but no metrics were emitted")?;
        if metrics.allocation_metrics.is_none() {
            return Err(
                "Generated BPE decode allocation tracking was requested, but no allocation metrics were emitted"
                    .into(),
            );
        }
        (base_sequences, failed_decodes, Some(metrics))
    } else {
        let (base_sequences, failed_decodes, metrics) = encoder
            .bpe_decode_batch_efficient_with_runtime_metrics(bpe_sequences, warmup_iterations)?;
        let metrics = metrics
            .ok_or("Generated BPE decode metrics were requested, but no metrics were emitted")?;
        (base_sequences, failed_decodes, Some(metrics))
    };
    if !failed_decodes.is_empty() {
        return Err(format!(
            "Generated BPE decode failed for {} measured token sequences",
            failed_decodes.len()
        )
        .into());
    }

    let (reencoded_sequences, failed_encodes, generated_bpe_encode_metrics) = if track_allocations {
        let (reencoded_sequences, failed_encodes, metrics) =
            encoder.bpe_encode_batch_efficient_with_metrics(&base_sequences, warmup_iterations)?;
        let metrics = metrics
            .ok_or("Generated BPE encode metrics were requested, but no metrics were emitted")?;
        if metrics.allocation_metrics.is_none() {
            return Err(
                "Generated BPE encode allocation tracking was requested, but no allocation metrics were emitted"
                    .into(),
            );
        }
        (reencoded_sequences, failed_encodes, Some(metrics))
    } else {
        let (reencoded_sequences, failed_encodes, metrics) = encoder
            .bpe_encode_batch_efficient_with_runtime_metrics(&base_sequences, warmup_iterations)?;
        let metrics = metrics
            .ok_or("Generated BPE encode metrics were requested, but no metrics were emitted")?;
        (reencoded_sequences, failed_encodes, Some(metrics))
    };
    if !failed_encodes.is_empty() {
        return Err(format!(
            "Generated BPE encode failed for {} measured token sequences",
            failed_encodes.len()
        )
        .into());
    }

    let round_trip_ok = reencoded_sequences == bpe_sequences;
    if !round_trip_ok {
        return Err(
            "Generated BPE decode->encode did not reproduce measured token sequences".into(),
        );
    }

    let input_tokens = token_count(&base_sequences);
    let output_tokens = token_count(&reencoded_sequences);
    let compression_ratio =
        (output_tokens > 0).then_some(input_tokens as f64 / output_tokens as f64);
    let encode_metrics =
        generated_bpe_encode_metrics.ok_or("Generated BPE encode metrics were unavailable")?;
    let decode_metrics =
        generated_bpe_decode_metrics.ok_or("Generated BPE decode metrics were unavailable")?;

    Ok(GeneratedBpeMetrics {
        available: true,
        input_tokens: Some(input_tokens),
        output_tokens: Some(output_tokens),
        compression_ratio,
        encode_wall_time_ms: Some(encode_metrics.wall_time_ms),
        decode_wall_time_ms: Some(decode_metrics.wall_time_ms),
        round_trip_ok: Some(true),
        generated_bpe_encode_allocations: encode_metrics
            .allocation_metrics
            .map(|metrics| metrics.allocations),
        generated_bpe_encode_allocated_bytes: encode_metrics
            .allocation_metrics
            .map(|metrics| metrics.allocated_bytes),
        generated_bpe_decode_allocations: decode_metrics
            .allocation_metrics
            .map(|metrics| metrics.allocations),
        generated_bpe_decode_allocated_bytes: decode_metrics
            .allocation_metrics
            .map(|metrics| metrics.allocated_bytes),
        note: "generated BPE metrics isolate embedded generated bpe::encode/decode after helper JSONL parsing and before stdout serialization; allocation counters cover the measured generated BPE batch window".to_string(),
    })
}

fn measure_generated_encode_phase_metrics(
    encoder: &InstanceEncoder,
    instance_paths: &[std::path::PathBuf],
    primary_token_sequences: &[Vec<u32>],
    primary_failed_paths: &[std::path::PathBuf],
    has_bpe: bool,
) -> Result<GeneratedEncodePhaseMetrics, Box<dyn Error>> {
    let (profiled_token_sequences, profiled_failed_paths, metrics) =
        encoder.encode_batch_efficient_with_phase_metrics(instance_paths)?;

    let token_sequences_match_primary = profiled_token_sequences == primary_token_sequences;
    let failed_paths_match_primary = profiled_failed_paths == primary_failed_paths;
    if !token_sequences_match_primary || !failed_paths_match_primary {
        return Err(
            "Generated encode phase sidecar did not reproduce primary encode results".into(),
        );
    }

    Ok(generated_encode_phase_metrics(
        metrics,
        token_sequences_match_primary,
        failed_paths_match_primary,
        has_bpe,
    ))
}

fn generated_encode_phase_metrics(
    metrics: BatchEncodePhaseMetrics,
    token_sequences_match_primary: bool,
    failed_paths_match_primary: bool,
    has_bpe: bool,
) -> GeneratedEncodePhaseMetrics {
    GeneratedEncodePhaseMetrics {
        available: true,
        records: Some(metrics.records),
        failures: Some(metrics.failures),
        profile_wall_time_ns: Some(metrics.profile_wall_time_ns),
        path_input_ns: Some(metrics.path_input_ns),
        json_read_parse_ns: Some(metrics.json_read_parse_ns),
        base_encode_ns: Some(metrics.base_encode_ns),
        bpe_encode_ns: metrics.bpe_encode_ns,
        output_serialization_ns: Some(metrics.output_serialization_ns),
        stdout_write_ns: Some(metrics.stdout_write_ns),
        token_sequences_match_primary: Some(token_sequences_match_primary),
        failed_paths_match_primary: Some(failed_paths_match_primary),
        note: if has_bpe {
            "generated encode phase metrics come from an opt-in sidecar pass after the primary encode; per-record phase nanoseconds are summed across Rayon workers and include BPE encode attribution".to_string()
        } else {
            "generated encode phase metrics come from an opt-in sidecar pass after the primary encode; per-record phase nanoseconds are summed across Rayon workers and bpe_encode_ns is null for non-BPE runs".to_string()
        },
    }
}

fn measure_generated_encode_hot_path_metrics(
    encoder: &InstanceEncoder,
    instance_paths: &[std::path::PathBuf],
    primary_token_sequences: &[Vec<u32>],
    primary_failed_count: usize,
    has_bpe: bool,
    track_allocations: bool,
) -> Result<GeneratedEncodeHotPathMetrics, Box<dyn Error>> {
    let (metrics, allocation_metrics) = if track_allocations {
        let (metrics, allocation_metrics) =
            encoder.encode_batch_hot_path_metrics_with_allocation_metrics(instance_paths)?;
        if allocation_metrics.is_none() {
            return Err(
                "Generated encode hot-path allocation tracking was requested, but no allocation metrics were emitted"
                    .into(),
            );
        }
        (metrics, allocation_metrics)
    } else {
        (encoder.encode_batch_hot_path_metrics(instance_paths)?, None)
    };
    let primary_token_count = token_count(primary_token_sequences);
    let primary_token_checksum = token_sequence_checksum(primary_token_sequences);
    let token_count_matches_primary = metrics.token_count == primary_token_count as u64;
    let token_checksum_matches_primary = metrics.token_checksum == primary_token_checksum;
    let failed_count_matches_primary = metrics.failures == primary_failed_count as u64;

    if !token_count_matches_primary
        || !token_checksum_matches_primary
        || !failed_count_matches_primary
    {
        return Err(
            "Generated encode hot-path sidecar did not reproduce primary encode summary".into(),
        );
    }

    Ok(generated_encode_hot_path_metrics(
        metrics,
        allocation_metrics,
        token_count_matches_primary,
        token_checksum_matches_primary,
        failed_count_matches_primary,
        has_bpe,
    ))
}

fn generated_encode_hot_path_metrics(
    metrics: BatchEncodeHotPathMetrics,
    allocation_metrics: Option<BatchAllocationMetrics>,
    token_count_matches_primary: bool,
    token_checksum_matches_primary: bool,
    failed_count_matches_primary: bool,
    has_bpe: bool,
) -> GeneratedEncodeHotPathMetrics {
    GeneratedEncodeHotPathMetrics {
        available: true,
        records: Some(metrics.records),
        failures: Some(metrics.failures),
        token_count: Some(metrics.token_count),
        token_checksum: Some(metrics.token_checksum),
        profile_wall_time_ns: Some(metrics.profile_wall_time_ns),
        path_input_ns: Some(metrics.path_input_ns),
        json_read_parse_ns: Some(metrics.json_read_parse_ns),
        base_encode_ns: Some(metrics.base_encode_ns),
        bpe_encode_ns: metrics.bpe_encode_ns,
        generated_encode_hot_path_allocations: allocation_metrics
            .map(|metrics| metrics.allocations),
        generated_encode_hot_path_allocated_bytes: allocation_metrics
            .map(|metrics| metrics.allocated_bytes),
        token_count_matches_primary: Some(token_count_matches_primary),
        token_checksum_matches_primary: Some(token_checksum_matches_primary),
        failed_count_matches_primary: Some(failed_count_matches_primary),
        note: if has_bpe {
            hot_path_note(allocation_metrics.is_some(), true)
        } else {
            hot_path_note(allocation_metrics.is_some(), false)
        },
    }
}

fn hot_path_note(allocation_metrics_available: bool, has_bpe: bool) -> String {
    let bpe_note = if has_bpe {
        ""
    } else {
        ", bpe_encode_ns is null"
    };
    let allocation_note = if allocation_metrics_available {
        " Allocation counters cover generated helper allocations after path ingestion, including JSON parsing, token encoding, optional BPE, and retained token vectors used for aggregate verification; they do not include parent-process memory."
    } else {
        " Allocation counters are disabled; pass --track-allocations with --generated-encode-hot-path-metrics to measure the sidecar allocation window."
    };
    format!(
        "generated encode hot-path metrics come from an opt-in sidecar pass after the primary encode; it emits only aggregate token count/checksum metrics{bpe_note}, so stdout token JSONL serialization is excluded.{allocation_note}"
    )
}

fn measure_generated_encode_validation_phase_metrics(
    encoder: &InstanceEncoder,
    instance_paths: &[std::path::PathBuf],
    primary_token_sequences: &[Vec<u32>],
    primary_failed_count: usize,
    has_bpe: bool,
) -> Result<GeneratedEncodeValidationPhaseMetrics, Box<dyn Error>> {
    let metrics = encoder.encode_batch_validation_phase_metrics(instance_paths)?;
    let primary_token_count = token_count(primary_token_sequences);
    let primary_token_checksum = token_sequence_checksum(primary_token_sequences);
    let token_count_matches_primary = metrics.token_count == primary_token_count as u64;
    let token_checksum_matches_primary = metrics.token_checksum == primary_token_checksum;
    let failed_count_matches_primary = metrics.failures == primary_failed_count as u64;

    if !token_count_matches_primary
        || !token_checksum_matches_primary
        || !failed_count_matches_primary
    {
        return Err(
            "Generated encode validation phase sidecar did not reproduce primary encode summary"
                .into(),
        );
    }

    Ok(generated_encode_validation_phase_metrics(
        metrics,
        token_count_matches_primary,
        token_checksum_matches_primary,
        failed_count_matches_primary,
        has_bpe,
    ))
}

fn generated_encode_validation_phase_metrics(
    metrics: BatchEncodeValidationPhaseMetrics,
    token_count_matches_primary: bool,
    token_checksum_matches_primary: bool,
    failed_count_matches_primary: bool,
    has_bpe: bool,
) -> GeneratedEncodeValidationPhaseMetrics {
    GeneratedEncodeValidationPhaseMetrics {
        available: true,
        records: Some(metrics.records),
        failures: Some(metrics.failures),
        token_count: Some(metrics.token_count),
        token_checksum: Some(metrics.token_checksum),
        profile_wall_time_ns: Some(metrics.profile_wall_time_ns),
        path_input_ns: Some(metrics.path_input_ns),
        file_read_ns: Some(metrics.file_read_ns),
        json_value_parse_ns: Some(metrics.json_value_parse_ns),
        serde_from_value_ns: Some(metrics.serde_from_value_ns),
        base_encode_ns: Some(metrics.base_encode_ns),
        bpe_encode_ns: metrics.bpe_encode_ns,
        token_count_matches_primary: Some(token_count_matches_primary),
        token_checksum_matches_primary: Some(token_checksum_matches_primary),
        failed_count_matches_primary: Some(failed_count_matches_primary),
        note: if has_bpe {
            "generated encode validation phase metrics are diagnostic; they parse JSON into serde_json::Value, deserialize with serde_json::from_value::<RootType>, then encode with BPE while verifying aggregate output against the primary direct from_str encode path".to_string()
        } else {
            "generated encode validation phase metrics are diagnostic; they parse JSON into serde_json::Value, deserialize with serde_json::from_value::<RootType>, then encode without BPE while verifying aggregate output against the primary direct from_str encode path".to_string()
        },
    }
}

fn measure_generated_decode_prefix_metrics(
    encoder: &InstanceEncoder,
    token_sequences: &[Vec<u32>],
    warmup_iterations: u32,
    has_bpe: bool,
) -> Result<GeneratedDecodePrefixMetrics, Box<dyn Error>> {
    let metrics = encoder.decode_prefix_batch_metrics(token_sequences, warmup_iterations)?;
    if metrics.records != token_sequences.len() as u64 {
        return Err("Generated decode_prefix sidecar did not process every token sequence".into());
    }
    if metrics.failed_prefixes > 0 {
        return Err(format!(
            "Generated decode_prefix sidecar reported {} failed prefixes",
            metrics.failed_prefixes
        )
        .into());
    }
    if metrics.full_sequences_complete != token_sequences.len() as u64 {
        return Err(
            "Generated decode_prefix sidecar did not complete every full measured sequence".into(),
        );
    }

    Ok(generated_decode_prefix_metrics(
        metrics,
        warmup_iterations,
        has_bpe,
    ))
}

fn generated_decode_prefix_metrics(
    metrics: BatchDecodePrefixMetrics,
    warmup_iterations: u32,
    has_bpe: bool,
) -> GeneratedDecodePrefixMetrics {
    GeneratedDecodePrefixMetrics {
        available: true,
        warmup_iterations: Some(warmup_iterations),
        records: Some(metrics.records),
        prefix_cuts: Some(metrics.prefix_cuts),
        complete_prefixes: Some(metrics.complete_prefixes),
        partial_prefixes: Some(metrics.partial_prefixes),
        failed_prefixes: Some(metrics.failed_prefixes),
        full_sequences_complete: Some(metrics.full_sequences_complete),
        consumed_tokens: Some(metrics.consumed_tokens),
        total_prefix_tokens: Some(metrics.total_prefix_tokens),
        fields_decoded_sum: Some(metrics.fields_decoded_sum),
        partial_fields_reported: Some(metrics.partial_fields_reported),
        profile_wall_time_ns: Some(metrics.profile_wall_time_ns),
        token_input_ns: Some(metrics.token_input_ns),
        bpe_decode_ns: metrics.bpe_decode_ns,
        decode_prefix_ns: Some(metrics.decode_prefix_ns),
        note: if has_bpe {
            "generated decode_prefix metrics are opt-in, honor warmup_iterations before measured timing, and expand each complete BPE sequence to raw TCT tokens before taking prefix cuts; they benchmark raw generated decode_prefix behavior, not BPE-prefix streaming".to_string()
        } else {
            "generated decode_prefix metrics are opt-in, honor warmup_iterations before measured timing, and run generated decode_prefix over full and truncated raw TCT token sequences after the primary encode/decode gate has passed".to_string()
        },
    }
}

fn token_count(sequences: &[Vec<u32>]) -> usize {
    sequences.iter().map(|sequence| sequence.len()).sum()
}

fn token_length_stats(sequences: &[Vec<u32>]) -> TokenLengthStats {
    let lengths: Vec<usize> = sequences.iter().map(Vec::len).collect();
    token_length_stats_from_lengths(&lengths)
}

fn token_length_stats_from_lengths(lengths: &[usize]) -> TokenLengthStats {
    let records = lengths.len();
    let total_tokens: usize = lengths.iter().sum();
    let mut sorted_lengths = lengths.to_vec();
    sorted_lengths.sort_unstable();

    let context_window_pass_count = lengths
        .iter()
        .filter(|&&length| length <= DEFAULT_CONTEXT_WINDOW_TOKENS)
        .count();
    let context_window_overflow_count = records.saturating_sub(context_window_pass_count);

    TokenLengthStats {
        records,
        total_tokens,
        min_tokens: sorted_lengths.first().copied(),
        max_tokens: sorted_lengths.last().copied(),
        average_tokens: (records > 0).then_some(total_tokens as f64 / records as f64),
        p50_tokens: nearest_rank_percentile(&sorted_lengths, 50, 100),
        p95_tokens: nearest_rank_percentile(&sorted_lengths, 95, 100),
        p99_tokens: nearest_rank_percentile(&sorted_lengths, 99, 100),
        context_window_tokens: DEFAULT_CONTEXT_WINDOW_TOKENS,
        context_window_pass_count,
        context_window_overflow_count,
        context_window_pass_rate: (records > 0)
            .then_some(context_window_pass_count as f64 / records as f64),
        percentile_method: "nearest_rank",
    }
}

fn nearest_rank_percentile(
    sorted_lengths: &[usize],
    percentile_numerator: usize,
    percentile_denominator: usize,
) -> Option<usize> {
    if sorted_lengths.is_empty() || percentile_denominator == 0 {
        return None;
    }

    let rank = sorted_lengths
        .len()
        .saturating_mul(percentile_numerator)
        .div_ceil(percentile_denominator);
    let index = rank.saturating_sub(1).min(sorted_lengths.len() - 1);
    sorted_lengths.get(index).copied()
}

fn elapsed_ms_u64(start: Instant) -> u64 {
    start.elapsed().as_millis().try_into().unwrap_or(u64::MAX)
}

fn generated_vocab_layout(
    base_vocab_size: u32,
    bpe_merges_count: Option<usize>,
) -> Result<(u32, u32), Box<dyn Error>> {
    let bpe_merges_count = bpe_merges_count
        .map(u32::try_from)
        .transpose()
        .map_err(|_| "BPE merge count exceeds u32 token ID range")?
        .unwrap_or(0);
    let pad_token = base_vocab_size
        .checked_add(bpe_merges_count)
        .ok_or("Generated PAD token exceeds u32 token ID range")?;
    let generated_vocab_size = pad_token
        .checked_add(1)
        .ok_or("Generated VOCAB_SIZE exceeds u32 token ID range")?;
    Ok((generated_vocab_size, pad_token))
}

fn allocation_metrics(
    generated_encode_allocations: Option<BatchAllocationMetrics>,
    generated_decode_allocations: Option<BatchAllocationMetrics>,
) -> AllocationMetrics {
    let available =
        generated_encode_allocations.is_some() && generated_decode_allocations.is_some();
    AllocationMetrics {
        available,
        generated_encode_allocations: generated_encode_allocations
            .map(|metrics| metrics.allocations),
        generated_encode_allocated_bytes: generated_encode_allocations
            .map(|metrics| metrics.allocated_bytes),
        generated_decode_allocations: generated_decode_allocations
            .map(|metrics| metrics.allocations),
        generated_decode_allocated_bytes: generated_decode_allocations
            .map(|metrics| metrics.allocated_bytes),
        note: if available {
            "allocation counts cover generated helper subprocess batch windows, including JSON parsing, generated encode/decode, optional BPE, and result serialization".to_string()
        } else {
            "allocation counters are disabled; pass --track-allocations to measure generated helper subprocess batch windows".to_string()
        },
    }
}

fn generated_batch_runtime_metrics(
    generated_encode_parent_wall_time_ms: u64,
    generated_decode_parent_wall_time_ms: u64,
    generated_encode_runtime_metrics: Option<BatchPrimaryRuntimeMetrics>,
    generated_decode_runtime_metrics: Option<BatchPrimaryRuntimeMetrics>,
) -> GeneratedBatchRuntimeMetrics {
    let generated_encode_child_wall_time_ms =
        generated_encode_runtime_metrics.map(|metrics| metrics.child_wall_time_ms);
    let generated_decode_child_wall_time_ms =
        generated_decode_runtime_metrics.map(|metrics| metrics.child_wall_time_ms);
    let available = generated_encode_child_wall_time_ms.is_some()
        && generated_decode_child_wall_time_ms.is_some();
    GeneratedBatchRuntimeMetrics {
        available,
        generated_encode_child_wall_time_ms,
        generated_decode_child_wall_time_ms,
        generated_encode_parent_wall_time_ms: Some(generated_encode_parent_wall_time_ms),
        generated_decode_parent_wall_time_ms: Some(generated_decode_parent_wall_time_ms),
        generated_encode_parent_transport_delta_wall_time_ms: generated_encode_child_wall_time_ms
            .map(|child| generated_encode_parent_wall_time_ms.saturating_sub(child)),
        generated_decode_parent_transport_delta_wall_time_ms: generated_decode_child_wall_time_ms
            .map(|child| generated_decode_parent_wall_time_ms.saturating_sub(child)),
        generated_encode_parent_stdin_write_ns: generated_encode_runtime_metrics
            .map(|metrics| metrics.parent_io.stdin_write_ns),
        generated_decode_parent_stdin_write_ns: generated_decode_runtime_metrics
            .map(|metrics| metrics.parent_io.stdin_write_ns),
        generated_encode_parent_stdout_read_parse_ns: generated_encode_runtime_metrics
            .map(|metrics| metrics.parent_io.stdout_read_parse_ns),
        generated_decode_parent_stdout_read_parse_ns: generated_decode_runtime_metrics
            .map(|metrics| metrics.parent_io.stdout_read_parse_ns),
        generated_encode_parent_stdout_json_parse_ns: generated_encode_runtime_metrics
            .map(|metrics| metrics.parent_io.stdout_json_parse_ns),
        generated_decode_parent_stdout_json_parse_ns: generated_decode_runtime_metrics
            .map(|metrics| metrics.parent_io.stdout_json_parse_ns),
        generated_encode_parent_wait_ns: generated_encode_runtime_metrics
            .map(|metrics| metrics.parent_io.child_wait_ns),
        generated_decode_parent_wait_ns: generated_decode_runtime_metrics
            .map(|metrics| metrics.parent_io.child_wait_ns),
        note: if available {
            "child wall times are emitted by the generated helper after stdin collection and before stdout JSONL writing; generated decode child timing covers the full public decode contract, including post-decode schema validation such as allOf checks, and is not a raw field-decode hot-path metric. parent stdin/stdout/wait fields are measured in tct-measure-tokens and may overlap with child execution. transport_delta_wall_time_ms is a saturating parent-minus-child envelope diagnostic, not an additive phase total".to_string()
        } else {
            "generated helper child timing was unavailable; parent wall times remain in timings.generated_encode_wall_time_ms and timings.generated_decode_wall_time_ms".to_string()
        },
    }
}

fn unavailable_generated_batch_runtime_metrics() -> GeneratedBatchRuntimeMetrics {
    GeneratedBatchRuntimeMetrics {
        available: false,
        generated_encode_child_wall_time_ms: None,
        generated_decode_child_wall_time_ms: None,
        generated_encode_parent_wall_time_ms: None,
        generated_decode_parent_wall_time_ms: None,
        generated_encode_parent_transport_delta_wall_time_ms: None,
        generated_decode_parent_transport_delta_wall_time_ms: None,
        generated_encode_parent_stdin_write_ns: None,
        generated_decode_parent_stdin_write_ns: None,
        generated_encode_parent_stdout_read_parse_ns: None,
        generated_decode_parent_stdout_read_parse_ns: None,
        generated_encode_parent_stdout_json_parse_ns: None,
        generated_decode_parent_stdout_json_parse_ns: None,
        generated_encode_parent_wait_ns: None,
        generated_decode_parent_wait_ns: None,
        note: "generated helper batch runtime metrics are unavailable outside schema-backed generated-tokenizer runs".to_string(),
    }
}

fn unavailable_same_process_bpe_metrics() -> SameProcessBpeMetrics {
    SameProcessBpeMetrics {
        available: false,
        input_tokens: None,
        output_tokens: None,
        compression_ratio: None,
        warmup_wall_time_ms: None,
        encode_wall_time_ms: None,
        decode_wall_time_ms: None,
        decode_round_trip_ok: None,
        merges_count: None,
        bpe_vocab_size: None,
        note: "same-process BPE metrics are disabled; pass --encoded with --bpe-merges to isolate runtime BPE over pre-encoded token sequences".to_string(),
    }
}

fn unavailable_generated_bpe_metrics() -> GeneratedBpeMetrics {
    GeneratedBpeMetrics {
        available: false,
        input_tokens: None,
        output_tokens: None,
        compression_ratio: None,
        encode_wall_time_ms: None,
        decode_wall_time_ms: None,
        round_trip_ok: None,
        generated_bpe_encode_allocations: None,
        generated_bpe_encode_allocated_bytes: None,
        generated_bpe_decode_allocations: None,
        generated_bpe_decode_allocated_bytes: None,
        note: "generated BPE-only metrics are disabled; pass --generated-bpe-metrics with a schema-backed BPE run to isolate embedded generated bpe::encode/decode over token JSONL".to_string(),
    }
}

fn unavailable_generated_encode_phase_metrics() -> GeneratedEncodePhaseMetrics {
    GeneratedEncodePhaseMetrics {
        available: false,
        records: None,
        failures: None,
        profile_wall_time_ns: None,
        path_input_ns: None,
        json_read_parse_ns: None,
        base_encode_ns: None,
        bpe_encode_ns: None,
        output_serialization_ns: None,
        stdout_write_ns: None,
        token_sequences_match_primary: None,
        failed_paths_match_primary: None,
        note: "generated encode phase metrics are disabled; pass --generated-encode-phase-metrics with a schema-backed run to profile JSON read/parse, base encoding, optional BPE, and output emission in a sidecar pass".to_string(),
    }
}

fn unavailable_generated_encode_hot_path_metrics() -> GeneratedEncodeHotPathMetrics {
    GeneratedEncodeHotPathMetrics {
        available: false,
        records: None,
        failures: None,
        token_count: None,
        token_checksum: None,
        profile_wall_time_ns: None,
        path_input_ns: None,
        json_read_parse_ns: None,
        base_encode_ns: None,
        bpe_encode_ns: None,
        generated_encode_hot_path_allocations: None,
        generated_encode_hot_path_allocated_bytes: None,
        token_count_matches_primary: None,
        token_checksum_matches_primary: None,
        failed_count_matches_primary: None,
        note: "generated encode hot-path metrics are disabled; pass --generated-encode-hot-path-metrics with a schema-backed run to profile JSON-to-token work without token JSONL output transport".to_string(),
    }
}

fn unavailable_generated_encode_validation_phase_metrics() -> GeneratedEncodeValidationPhaseMetrics
{
    GeneratedEncodeValidationPhaseMetrics {
        available: false,
        records: None,
        failures: None,
        token_count: None,
        token_checksum: None,
        profile_wall_time_ns: None,
        path_input_ns: None,
        file_read_ns: None,
        json_value_parse_ns: None,
        serde_from_value_ns: None,
        base_encode_ns: None,
        bpe_encode_ns: None,
        token_count_matches_primary: None,
        token_checksum_matches_primary: None,
        failed_count_matches_primary: None,
        note: "generated encode validation phase metrics are disabled; pass --generated-encode-validation-phase-metrics with a schema-backed run to split file read, JSON Value parse, typed serde validation, and token encoding in a diagnostic sidecar".to_string(),
    }
}

fn unavailable_generated_decode_prefix_metrics() -> GeneratedDecodePrefixMetrics {
    GeneratedDecodePrefixMetrics {
        available: false,
        warmup_iterations: None,
        records: None,
        prefix_cuts: None,
        complete_prefixes: None,
        partial_prefixes: None,
        failed_prefixes: None,
        full_sequences_complete: None,
        consumed_tokens: None,
        total_prefix_tokens: None,
        fields_decoded_sum: None,
        partial_fields_reported: None,
        profile_wall_time_ns: None,
        token_input_ns: None,
        bpe_decode_ns: None,
        decode_prefix_ns: None,
        note: "generated decode_prefix metrics are disabled; pass --generated-decode-prefix-metrics with a schema-backed run to measure raw generated prefix decoding over measured token sequences".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tct::instance_encoder::BatchParentIoMetrics;

    fn fake_file(path: &str) -> FileProvenance {
        FileProvenance {
            path: path.to_string(),
            bytes: 3,
            sha256: "0".repeat(64),
        }
    }

    fn test_timings() -> MeasurementTimings {
        MeasurementTimings {
            schema_load_time_ms: 1,
            compiler_setup_time_ms: 2,
            schema_compile_time_ms: 3,
            code_generation_time_ms: 4,
            instance_discovery_time_ms: 5,
            generated_encoder_build_time_ms: 6,
            generated_warmup_wall_time_ms: 7,
            generated_encode_wall_time_ms: 8,
            generated_decode_wall_time_ms: 9,
            total_wall_time_ms: 37,
            note: measurement_timing_note(),
        }
    }

    fn unavailable_allocations() -> AllocationMetrics {
        allocation_metrics(None, None)
    }

    fn available_allocations() -> AllocationMetrics {
        allocation_metrics(
            Some(BatchAllocationMetrics {
                allocations: 10,
                allocated_bytes: 1000,
            }),
            Some(BatchAllocationMetrics {
                allocations: 20,
                allocated_bytes: 2000,
            }),
        )
    }

    fn unavailable_same_process_bpe() -> SameProcessBpeMetrics {
        unavailable_same_process_bpe_metrics()
    }

    fn unavailable_generated_bpe() -> GeneratedBpeMetrics {
        unavailable_generated_bpe_metrics()
    }

    fn unavailable_generated_batch_runtime() -> GeneratedBatchRuntimeMetrics {
        unavailable_generated_batch_runtime_metrics()
    }

    fn unavailable_generated_encode_phases() -> GeneratedEncodePhaseMetrics {
        unavailable_generated_encode_phase_metrics()
    }

    fn unavailable_generated_encode_hot_path() -> GeneratedEncodeHotPathMetrics {
        unavailable_generated_encode_hot_path_metrics()
    }

    fn unavailable_generated_encode_validation_phase() -> GeneratedEncodeValidationPhaseMetrics {
        unavailable_generated_encode_validation_phase_metrics()
    }

    fn unavailable_generated_decode_prefix() -> GeneratedDecodePrefixMetrics {
        unavailable_generated_decode_prefix_metrics()
    }

    #[test]
    fn measurement_report_serializes_legacy_counts_and_provenance() {
        let report = MeasurementReport {
            tokens: 42,
            instances: 2,
            characters: 100,
            has_bpe: true,
            warmup_iterations: 2,
            base_vocab_size: 257,
            generated_vocab_size: 259,
            pad_token: 258,
            bpe_base_vocab_size: Some(257),
            bpe_merges_count: Some(1),
            token_length_stats: token_length_stats_from_lengths(&[20, 22]),
            timings: test_timings(),
            allocation_metrics: unavailable_allocations(),
            generated_batch_runtime_metrics: unavailable_generated_batch_runtime(),
            same_process_bpe_metrics: unavailable_same_process_bpe(),
            generated_bpe_metrics: unavailable_generated_bpe(),
            generated_encode_phase_metrics: unavailable_generated_encode_phases(),
            generated_encode_hot_path_metrics: unavailable_generated_encode_hot_path(),
            generated_encode_validation_phase_metrics:
                unavailable_generated_encode_validation_phase(),
            generated_decode_prefix_metrics: unavailable_generated_decode_prefix(),
            provenance: MeasurementProvenance {
                command: vec!["tct-measure-tokens".to_string()],
                schema: Some("schema.json".to_string()),
                instances_pattern: Some("instances/*.json".to_string()),
                encoded_input: None,
                bpe_merges: Some("merges.json".to_string()),
                schema_file: Some(fake_file("schema.json")),
                input_files: vec![fake_file("instances/one.json")],
                encoded_input_file: None,
                bpe_merges_file: Some(fake_file("merges.json")),
                measured_files_output: Some("measured.txt".to_string()),
                measured_files_output_file: Some(fake_file("measured.txt")),
                base_vocab_size: 257,
                base_vocab_source: "grammar".to_string(),
                optimization_level: Some("O3".to_string()),
                created_at: "2026-04-26T00:00:00Z".to_string(),
                tct_version: "0.1.0".to_string(),
            },
        };

        let value = serde_json::to_value(&report).unwrap();

        assert_eq!(value["tokens"], 42);
        assert_eq!(value["instances"], 2);
        assert_eq!(value["characters"], 100);
        assert_eq!(value["has_bpe"], true);
        assert_eq!(value["warmup_iterations"], 2);
        assert_eq!(value["base_vocab_size"], 257);
        assert_eq!(value["generated_vocab_size"], 259);
        assert_eq!(value["pad_token"], 258);
        assert_eq!(value["bpe_base_vocab_size"], 257);
        assert_eq!(value["bpe_merges_count"], 1);
        assert_eq!(value["token_length_stats"]["records"], 2);
        assert_eq!(value["token_length_stats"]["total_tokens"], 42);
        assert_eq!(value["token_length_stats"]["average_tokens"], 21.0);
        assert_eq!(value["token_length_stats"]["p95_tokens"], 22);
        assert_eq!(value["token_length_stats"]["p99_tokens"], 22);
        assert_eq!(value["token_length_stats"]["context_window_tokens"], 2048);
        assert_eq!(value["token_length_stats"]["context_window_pass_count"], 2);
        assert_eq!(value["token_length_stats"]["context_window_pass_rate"], 1.0);
        assert_eq!(
            value["token_length_stats"]["percentile_method"],
            "nearest_rank"
        );
        assert_eq!(value["timings"]["schema_load_time_ms"], 1);
        assert_eq!(value["timings"]["generated_warmup_wall_time_ms"], 7);
        assert_eq!(value["timings"]["generated_encode_wall_time_ms"], 8);
        assert_eq!(value["timings"]["generated_decode_wall_time_ms"], 9);
        assert_eq!(value["timings"]["total_wall_time_ms"], 37);
        assert!(
            value["timings"]["note"]
                .as_str()
                .unwrap()
                .contains("parent-observed full-contract envelopes")
        );
        assert_eq!(value["allocation_metrics"]["available"], false);
        assert_eq!(value["generated_batch_runtime_metrics"]["available"], false);
        assert_eq!(value["same_process_bpe_metrics"]["available"], false);
        assert_eq!(value["generated_bpe_metrics"]["available"], false);
        assert_eq!(value["generated_encode_phase_metrics"]["available"], false);
        assert_eq!(
            value["generated_encode_hot_path_metrics"]["available"],
            false
        );
        assert_eq!(
            value["generated_encode_validation_phase_metrics"]["available"],
            false
        );
        assert_eq!(value["generated_decode_prefix_metrics"]["available"], false);
        assert_eq!(
            value["allocation_metrics"]["generated_encode_allocations"],
            serde_json::Value::Null
        );
        assert_eq!(
            value["allocation_metrics"]["generated_encode_allocated_bytes"],
            serde_json::Value::Null
        );
        assert_eq!(value["provenance"]["base_vocab_source"], "grammar");
        assert_eq!(value["provenance"]["schema_file"]["path"], "schema.json");
        assert_eq!(
            value["provenance"]["bpe_merges_file"]["path"],
            "merges.json"
        );
        assert_eq!(
            value["provenance"]["measured_files_output_file"]["path"],
            "measured.txt"
        );
    }

    #[test]
    fn measurement_report_serializes_no_bpe_as_null_artifacts() {
        let report = MeasurementReport {
            tokens: 7,
            instances: 1,
            characters: 20,
            has_bpe: false,
            warmup_iterations: 0,
            base_vocab_size: 257,
            generated_vocab_size: 258,
            pad_token: 257,
            bpe_base_vocab_size: None,
            bpe_merges_count: None,
            token_length_stats: token_length_stats_from_lengths(&[7]),
            timings: test_timings(),
            allocation_metrics: unavailable_allocations(),
            generated_batch_runtime_metrics: unavailable_generated_batch_runtime(),
            same_process_bpe_metrics: unavailable_same_process_bpe(),
            generated_bpe_metrics: unavailable_generated_bpe(),
            generated_encode_phase_metrics: unavailable_generated_encode_phases(),
            generated_encode_hot_path_metrics: unavailable_generated_encode_hot_path(),
            generated_encode_validation_phase_metrics:
                unavailable_generated_encode_validation_phase(),
            generated_decode_prefix_metrics: unavailable_generated_decode_prefix(),
            provenance: MeasurementProvenance {
                command: vec!["tct-measure-tokens".to_string()],
                schema: Some("schema.json".to_string()),
                instances_pattern: Some("instances/*.json".to_string()),
                encoded_input: None,
                bpe_merges: None,
                schema_file: Some(fake_file("schema.json")),
                input_files: vec![fake_file("instances/one.json")],
                encoded_input_file: None,
                bpe_merges_file: None,
                measured_files_output: None,
                measured_files_output_file: None,
                base_vocab_size: 257,
                base_vocab_source: "grammar".to_string(),
                optimization_level: Some("O3".to_string()),
                created_at: "2026-04-26T00:00:00Z".to_string(),
                tct_version: "0.1.0".to_string(),
            },
        };

        let value = serde_json::to_value(&report).unwrap();

        assert_eq!(value["has_bpe"], false);
        assert_eq!(value["warmup_iterations"], 0);
        assert_eq!(value["generated_vocab_size"], 258);
        assert_eq!(value["pad_token"], 257);
        assert_eq!(value["bpe_base_vocab_size"], serde_json::Value::Null);
        assert_eq!(value["bpe_merges_count"], serde_json::Value::Null);
        assert_eq!(value["token_length_stats"]["records"], 1);
        assert_eq!(value["token_length_stats"]["max_tokens"], 7);
        assert_eq!(value["timings"]["code_generation_time_ms"], 4);
        assert_eq!(value["timings"]["generated_decode_wall_time_ms"], 9);
        assert_eq!(value["allocation_metrics"]["available"], false);
        assert_eq!(
            value["provenance"]["bpe_merges_file"],
            serde_json::Value::Null
        );
        assert_eq!(
            value["provenance"]["measured_files_output_file"],
            serde_json::Value::Null
        );
    }

    #[test]
    fn measurement_report_serializes_available_allocation_metrics() {
        let report = MeasurementReport {
            tokens: 7,
            instances: 1,
            characters: 20,
            has_bpe: false,
            warmup_iterations: 0,
            base_vocab_size: 257,
            generated_vocab_size: 258,
            pad_token: 257,
            bpe_base_vocab_size: None,
            bpe_merges_count: None,
            token_length_stats: token_length_stats_from_lengths(&[7]),
            timings: test_timings(),
            allocation_metrics: available_allocations(),
            generated_batch_runtime_metrics: generated_batch_runtime_metrics(
                12,
                15,
                Some(BatchPrimaryRuntimeMetrics {
                    child_wall_time_ms: 7,
                    parent_io: BatchParentIoMetrics {
                        stdin_write_ns: 100,
                        stdout_read_parse_ns: 200,
                        stdout_json_parse_ns: 30,
                        child_wait_ns: 40,
                    },
                    allocation_metrics: Some(BatchAllocationMetrics {
                        allocations: 10,
                        allocated_bytes: 1000,
                    }),
                }),
                Some(BatchPrimaryRuntimeMetrics {
                    child_wall_time_ms: 9,
                    parent_io: BatchParentIoMetrics {
                        stdin_write_ns: 110,
                        stdout_read_parse_ns: 210,
                        stdout_json_parse_ns: 31,
                        child_wait_ns: 41,
                    },
                    allocation_metrics: Some(BatchAllocationMetrics {
                        allocations: 20,
                        allocated_bytes: 2000,
                    }),
                }),
            ),
            same_process_bpe_metrics: unavailable_same_process_bpe(),
            generated_bpe_metrics: unavailable_generated_bpe(),
            generated_encode_phase_metrics: unavailable_generated_encode_phases(),
            generated_encode_hot_path_metrics: unavailable_generated_encode_hot_path(),
            generated_encode_validation_phase_metrics:
                unavailable_generated_encode_validation_phase(),
            generated_decode_prefix_metrics: unavailable_generated_decode_prefix(),
            provenance: MeasurementProvenance {
                command: vec!["tct-measure-tokens".to_string()],
                schema: Some("schema.json".to_string()),
                instances_pattern: Some("instances/*.json".to_string()),
                encoded_input: None,
                bpe_merges: None,
                schema_file: Some(fake_file("schema.json")),
                input_files: vec![fake_file("instances/one.json")],
                encoded_input_file: None,
                bpe_merges_file: None,
                measured_files_output: None,
                measured_files_output_file: None,
                base_vocab_size: 257,
                base_vocab_source: "grammar".to_string(),
                optimization_level: Some("O3".to_string()),
                created_at: "2026-04-26T00:00:00Z".to_string(),
                tct_version: "0.1.0".to_string(),
            },
        };

        let value = serde_json::to_value(&report).unwrap();

        assert_eq!(value["allocation_metrics"]["available"], true);
        assert_eq!(
            value["allocation_metrics"]["generated_encode_allocations"],
            10
        );
        assert_eq!(
            value["allocation_metrics"]["generated_encode_allocated_bytes"],
            1000
        );
        assert_eq!(
            value["allocation_metrics"]["generated_decode_allocations"],
            20
        );
        assert_eq!(
            value["allocation_metrics"]["generated_decode_allocated_bytes"],
            2000
        );
        assert!(
            value["allocation_metrics"]["note"]
                .as_str()
                .unwrap()
                .contains("generated helper subprocess batch windows")
        );
        assert_eq!(value["generated_batch_runtime_metrics"]["available"], true);
        assert_eq!(
            value["generated_batch_runtime_metrics"]["generated_encode_child_wall_time_ms"],
            7
        );
        assert_eq!(
            value["generated_batch_runtime_metrics"]["generated_decode_child_wall_time_ms"],
            9
        );
        assert_eq!(
            value["generated_batch_runtime_metrics"]["generated_encode_parent_transport_delta_wall_time_ms"],
            5
        );
        assert_eq!(
            value["generated_batch_runtime_metrics"]["generated_encode_parent_stdout_json_parse_ns"],
            30
        );
        assert!(
            value["generated_batch_runtime_metrics"]["note"]
                .as_str()
                .unwrap()
                .contains("post-decode schema validation")
        );
    }

    #[test]
    fn generated_encode_phase_metrics_reports_sidecar_fields() {
        let value = serde_json::to_value(generated_encode_phase_metrics(
            BatchEncodePhaseMetrics {
                records: 3,
                failures: 1,
                profile_wall_time_ns: 100,
                path_input_ns: 2,
                json_read_parse_ns: 30,
                base_encode_ns: 40,
                bpe_encode_ns: Some(5),
                output_serialization_ns: 10,
                stdout_write_ns: 7,
            },
            true,
            true,
            true,
        ))
        .unwrap();

        assert_eq!(value["available"], true);
        assert_eq!(value["records"], 3);
        assert_eq!(value["failures"], 1);
        assert_eq!(value["profile_wall_time_ns"], 100);
        assert_eq!(value["path_input_ns"], 2);
        assert_eq!(value["json_read_parse_ns"], 30);
        assert_eq!(value["base_encode_ns"], 40);
        assert_eq!(value["bpe_encode_ns"], 5);
        assert_eq!(value["output_serialization_ns"], 10);
        assert_eq!(value["stdout_write_ns"], 7);
        assert_eq!(value["token_sequences_match_primary"], true);
        assert_eq!(value["failed_paths_match_primary"], true);
    }

    #[test]
    fn generated_encode_hot_path_metrics_reports_sidecar_fields() {
        let value = serde_json::to_value(generated_encode_hot_path_metrics(
            BatchEncodeHotPathMetrics {
                records: 3,
                failures: 1,
                token_count: 42,
                token_checksum: 123,
                profile_wall_time_ns: 100,
                path_input_ns: 2,
                json_read_parse_ns: 30,
                base_encode_ns: 40,
                bpe_encode_ns: None,
            },
            Some(BatchAllocationMetrics {
                allocations: 4,
                allocated_bytes: 2048,
            }),
            true,
            true,
            true,
            false,
        ))
        .unwrap();

        assert_eq!(value["available"], true);
        assert_eq!(value["records"], 3);
        assert_eq!(value["failures"], 1);
        assert_eq!(value["token_count"], 42);
        assert_eq!(value["token_checksum"], 123);
        assert_eq!(value["profile_wall_time_ns"], 100);
        assert_eq!(value["path_input_ns"], 2);
        assert_eq!(value["json_read_parse_ns"], 30);
        assert_eq!(value["base_encode_ns"], 40);
        assert_eq!(value["bpe_encode_ns"], serde_json::Value::Null);
        assert_eq!(value["generated_encode_hot_path_allocations"], 4);
        assert_eq!(value["generated_encode_hot_path_allocated_bytes"], 2048);
        assert_eq!(value["token_count_matches_primary"], true);
        assert_eq!(value["token_checksum_matches_primary"], true);
        assert_eq!(value["failed_count_matches_primary"], true);
    }

    #[test]
    fn generated_encode_validation_phase_metrics_reports_sidecar_fields() {
        let value = serde_json::to_value(generated_encode_validation_phase_metrics(
            BatchEncodeValidationPhaseMetrics {
                records: 3,
                failures: 1,
                token_count: 42,
                token_checksum: 123,
                profile_wall_time_ns: 100,
                path_input_ns: 2,
                file_read_ns: 11,
                json_value_parse_ns: 22,
                serde_from_value_ns: 33,
                base_encode_ns: 44,
                bpe_encode_ns: Some(55),
            },
            true,
            true,
            true,
            true,
        ))
        .unwrap();

        assert_eq!(value["available"], true);
        assert_eq!(value["records"], 3);
        assert_eq!(value["failures"], 1);
        assert_eq!(value["token_count"], 42);
        assert_eq!(value["token_checksum"], 123);
        assert_eq!(value["profile_wall_time_ns"], 100);
        assert_eq!(value["path_input_ns"], 2);
        assert_eq!(value["file_read_ns"], 11);
        assert_eq!(value["json_value_parse_ns"], 22);
        assert_eq!(value["serde_from_value_ns"], 33);
        assert_eq!(value["base_encode_ns"], 44);
        assert_eq!(value["bpe_encode_ns"], 55);
        assert_eq!(value["token_count_matches_primary"], true);
        assert_eq!(value["token_checksum_matches_primary"], true);
        assert_eq!(value["failed_count_matches_primary"], true);
    }

    #[test]
    fn token_length_stats_reports_percentiles_and_context_pass_rate() {
        let stats = token_length_stats_from_lengths(&[4, 8, 16, 2048, 2049]);
        let value = serde_json::to_value(&stats).unwrap();

        assert_eq!(value["records"], 5);
        assert_eq!(value["total_tokens"], 4125);
        assert_eq!(value["min_tokens"], 4);
        assert_eq!(value["max_tokens"], 2049);
        assert_eq!(value["average_tokens"], 825.0);
        assert_eq!(value["p50_tokens"], 16);
        assert_eq!(value["p95_tokens"], 2049);
        assert_eq!(value["p99_tokens"], 2049);
        assert_eq!(value["context_window_tokens"], 2048);
        assert_eq!(value["context_window_pass_count"], 4);
        assert_eq!(value["context_window_overflow_count"], 1);
        assert_eq!(value["context_window_pass_rate"], 0.8);
        assert_eq!(value["percentile_method"], "nearest_rank");
    }

    #[test]
    fn token_length_stats_handles_empty_sequences() {
        let stats = token_length_stats_from_lengths(&[]);
        let value = serde_json::to_value(&stats).unwrap();

        assert_eq!(value["records"], 0);
        assert_eq!(value["total_tokens"], 0);
        assert_eq!(value["min_tokens"], serde_json::Value::Null);
        assert_eq!(value["p95_tokens"], serde_json::Value::Null);
        assert_eq!(value["context_window_pass_rate"], serde_json::Value::Null);
    }

    #[test]
    fn generated_vocab_layout_matches_generated_constants() {
        assert_eq!(generated_vocab_layout(257, None).unwrap(), (258, 257));
        assert_eq!(generated_vocab_layout(257, Some(2)).unwrap(), (260, 259));
    }

    #[test]
    fn same_process_bpe_metrics_serialize_isolated_counts() {
        let metrics = SameProcessBpeMetrics {
            available: true,
            input_tokens: Some(7),
            output_tokens: Some(4),
            compression_ratio: Some(1.75),
            warmup_wall_time_ms: Some(1),
            encode_wall_time_ms: Some(2),
            decode_wall_time_ms: Some(3),
            decode_round_trip_ok: Some(true),
            merges_count: Some(1),
            bpe_vocab_size: Some(258),
            note: "same-process".to_string(),
        };

        let value = serde_json::to_value(&metrics).unwrap();

        assert_eq!(value["available"], true);
        assert_eq!(value["input_tokens"], 7);
        assert_eq!(value["output_tokens"], 4);
        assert_eq!(value["compression_ratio"], 1.75);
        assert_eq!(value["decode_round_trip_ok"], true);
        assert_eq!(value["merges_count"], 1);
        assert_eq!(value["bpe_vocab_size"], 258);
    }
}
