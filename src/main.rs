use clap::{Arg, ArgAction, ArgMatches, Command, value_parser};
use serde_json::Value;
use std::{error::Error, fs, path::Path};
use tct::provenance::sha256_hex;
use tct::vocabulary_analyzer::VocabularyAnalyzer;
use tct::{OptimizationLevel, TctCompiler};

fn main() -> Result<(), Box<dyn Error>> {
    let matches = Command::new("tct")
        .about("Type-Constrained Tokenization - Generate schema-aware tokenizers")
        .version("0.1.0")
        .arg(
            Arg::new("input")
                .help("Input JSON schema file")
                .required(true)
                .value_parser(value_parser!(String)),
        )
        .arg(
            Arg::new("output")
                .short('o')
                .long("output")
                .help("Output file for generated Rust code (default: stdout)")
                .value_parser(value_parser!(String)),
        )
        .arg(
            Arg::new("dry-run")
                .long("dry-run")
                .help("Compile and render Rust output without writing generated code")
                .long_help(
                    "Compile and render Rust output without writing generated code:\n\
                     Validates schema parsing, compiler setup, BPE merge loading, and Rust code rendering.\n\
                     Suppresses generated-code stdout and skips --output file writes while reporting output bytes and SHA-256.",
                )
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new("name")
                .short('n')
                .long("name")
                .help("Root type name (default: the schema title, else the file name)")
                .value_parser(value_parser!(String)),
        )
        .arg(
            Arg::new("benchmark")
                .short('b')
                .long("benchmark")
                .help("Run compression benchmark on generated code")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new("quiet")
                .short('q')
                .long("quiet")
                .help("Suppress informational output")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new("optimization")
                .short('O')
                .long("optimization")
                .help("Optimization level")
                .long_help(
                    "Optimization level:\n\
                     O0 - Development mode: no optimization passes (enums are still one index token each)\n\
                     O2 - Balanced: bounded-integer offset encodings and zero-token single-variant enums\n\
                     O3 - Aggressive optimization (default): O2 plus format-aware strings and bit-packed optional fields\n\
                     Os - Size profile: currently identical to OVocab (O3 without format-aware strings)\n\
                     OVocab - Vocabulary profile: currently identical to Os\n\
                     Accepted spellings include 0/O0, 2/O2, 3/O3, s/Os/size, and vocab/OVocab",
                )
                .value_parser(value_parser!(String))
                .default_value("3"),
        )
        .arg(
            Arg::new("max-vocab")
                .long("max-vocab")
                .help("Cap the structural vocabulary of enums and bounded integers via multi-token codecs")
                .long_help(
                    "Maximum raw vocabulary. Values must be at least 257 so UTF-8 bytes plus the value-end marker remain representable. Use 257 to preserve a shared grammar-action alphabet across schemas.",
                )
                .value_parser(value_parser!(usize)),
        )
        .arg(
            Arg::new("enable-pass")
                .long("enable-pass")
                .help("Enable a named built-in optimization pass (overrides -O level)")
                .long_help(
                    "Enable specific named built-in optimization passes:\n\
                     enum_optimization - Zero-token single-variant enums; multi-token enum codec under --max-vocab\n\
                     integer_optimization - Compact offset encoding for bounded integers\n\
                     vocabulary_optimization - Report vocabulary statistics (does not change the generated encodings)\n\
                     \n\
                     Can be specified multiple times. Overrides optimization level.\n\
                     This does not load custom TokenStream-returning optimization passes.\n\
                     pattern_optimization is reserved and rejected until direct finite-regex encoding is implemented.",
                )
                .action(ArgAction::Append)
                .value_parser(value_parser!(String)),
        )
        .arg(
            Arg::new("disable-all-passes")
                .long("disable-all-passes")
                .help("Disable all optimizations (equivalent to -O0)")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new("enable-all-passes")
                .long("enable-all-passes")
                .help("Enable all production optimization passes (equivalent to -O2)")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new("enum-optimization")
                .long("enum-optimization")
                .help("Enable enum index optimization")
                .long_help(
                    "Enable enum index optimization:\n\
                     Encodes enum values as indices (0, 1, 2, ...) instead of strings.\n\
                     Overrides the optimization level's default for this specific pass.",
                )
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new("bounded-int-optimization")
                .long("bounded-int-optimization")
                .help("Enable bounded integer optimization")
                .long_help(
                    "Enable bounded integer optimization:\n\
                     Integers with min/max constraints use optimized compact encoding.\n\
                     Overrides the optimization level's default for this specific pass.",
                )
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new("pattern-optimization")
                .long("pattern-optimization")
                .help("Reserved legacy flag for pattern optimization")
                .long_help(
                    "Reserved legacy flag for pattern optimization:\n\
                     Direct finite-regex encoding is not implemented as an optimization pass.\n\
                     patternProperties validation is always schema-driven and does not require this flag.",
                )
                .hide(true)
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new("bit-packing")
                .long("bit-packing")
                .help("Enable bit-packing optimization (experimental)")
                .long_help(
                     "Enable bit-packing optimization (experimental):\n\
                     Optional field presence flags are packed into u32 tokens.\n\
                     This experimental optimization is enabled by default at O3, Os, and OVocab.\n\
                     Overrides the optimization level's default for this specific pass.",
                )
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new("bpe-merges")
                .long("bpe-merges")
                .help("BPE merge file for token compression")
                .long_help(
                    "Enable Byte Pair Encoding (BPE) compression using a merge file:\n\
                     BPE merges frequently co-occurring token pairs to reduce sequence length.\n\
                     The merge file should be generated by 'tct-train-bpe' from training data.\n\
                     Example: --bpe-merges schemas/bpe/github-workflow-tct-bpe-500.json",
                )
                .value_parser(value_parser!(String)),
        )
        .arg(
            Arg::new("analyze")
                .long("analyze")
                .help("Analyze schema vocabulary requirements instead of generating code")
                .long_help(
                    "Analyze schema vocabulary requirements:\n\
                     Reports per-position vocabulary requirements and provides optimization recommendations.\n\
                     Useful for determining optimal --max-vocab values and understanding tokenization efficiency.\n\
                     Can be combined with --format and --compare options for detailed analysis.",
                )
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new("format")
                .long("format")
                .help("Output format for analysis results")
                .long_help(
                    "Specify output format for vocabulary analysis:\n\
                     text - Human-readable text format (default)\n\
                     json - Machine-readable JSON format for scripting",
                )
                .value_parser(["text", "json"])
                .default_value("text")
                .requires("analyze"),
        )
        .arg(
            Arg::new("compare")
                .long("compare")
                .help("Compare vocabulary efficiency across different max-vocab sizes")
                .long_help(
                    "Compare vocabulary efficiency across different vocabulary sizes:\n\
                     Accepts comma-separated values (e.g., --compare 64,128,256,512)\n\
                     Shows compression ratios and token usage for each vocabulary size.\n\
                     Helps identify optimal vocabulary size for your schema.",
                )
                .value_parser(value_parser!(String))
                .requires("analyze"),
        )
        .arg(
            Arg::new("metrics")
                .long("metrics")
                .help("Display detailed compilation performance metrics")
                .long_help(
                    "Display detailed performance metrics:\n\
                     Shows compilation timing breakdown, token optimization statistics,\n\
                     vocabulary usage, and complexity analysis. Useful for performance\n\
                     analysis and optimization validation.",
                )
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new("target")
                .long("target")
                .help("Output target format")
                .long_help(
                    "Target format for code generation:\n\
                     rust - Generate Rust code (default)\n\
                     python - Generate Python wheel package\n\
                     \n\
                     Python-specific options: --python-name, --python-version, --description",
                )
                .value_parser(["rust", "python"])
                .default_value("rust"),
        )
        .arg(
            Arg::new("python-name")
                .long("python-name")
                .help("Python package name (only for --target python)")
                .long_help(
                    "Name for the generated Python package (e.g., 'tct-workflow-tokenizer').\n\
                     Only used when --target python is specified.\n\
                     Default: derived from schema filename with 'tct-' prefix",
                )
                .value_parser(value_parser!(String))
                .requires("target"),
        )
        .arg(
            Arg::new("python-version")
                .long("python-version")
                .help("Python package version (only for --target python)")
                .long_help(
                    "Version for the generated Python package (e.g., '1.0.0').\n\
                     Only used when --target python is specified.\n\
                     Default: 0.1.0",
                )
                .value_parser(value_parser!(String))
                .default_value("0.1.0")
                .requires("target"),
        )
        .arg(
            Arg::new("python-description")
                .long("description")
                .help("Python package description (only for --target python)")
                .long_help(
                    "Description for the generated Python package.\n\
                     Only used when --target python is specified.\n\
                     Default: auto-generated from schema",
                )
                .value_parser(value_parser!(String))
                .requires("target"),
        )
        .arg(
            Arg::new("streaming")
                .long("streaming")
                .help("Enable streaming-friendly encoding for prefix decoding")
                .long_help(
                    "Enable streaming-friendly encoding format:\n\
                     \n\
                     Token format is IDENTICAL to non-streaming mode, enabling:\n\
                     - Full compatibility between streaming and non-streaming wheels\n\
                     - decode_prefix() for truncation-tolerant decoding\n\
                     - LLM autocompletion of partial JSON\n\
                     \n\
                     When enabled, generates:\n\
                     - Default impl for all types\n\
                     - decode_prefix() -> PrefixDecodeResult<T>\n\
                     \n\
                     Use for LLM autocompletion workflows.",
                )
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new("decimal-numbers")
                .long("decimal-numbers")
                .help("Digit-aligned decimal text encoding for unbounded i64/u64/f64")
                .long_help(
                    "Encode unbounded i64/u64/f64 as shortest round-trip decimal text\n\
                     (one byte token per ASCII character, END_STRING_MARKER terminated)\n\
                     instead of fixed-width bit splits. Opt-in: the token stream layout\n\
                     is INCOMPATIBLE with the default encoding. Bounded integers keep\n\
                     their dedicated compact path.",
                )
                .action(ArgAction::SetTrue),
        )
        .get_matches();

    let input_path = required_arg::<String>(&matches, "input")?;
    let output_path = matches.get_one::<String>("output");
    let dry_run = matches.get_flag("dry-run");
    let type_name = matches.get_one::<String>("name");
    let run_benchmark = matches.get_flag("benchmark");
    let quiet = matches.get_flag("quiet");
    let optimization_str = required_arg::<String>(&matches, "optimization")?;
    let max_vocab = matches.get_one::<usize>("max-vocab").copied();

    // Parse individual pass flags
    let enabled_passes: Vec<String> = matches
        .get_many::<String>("enable-pass")
        .map(|vals| vals.cloned().collect())
        .unwrap_or_default();
    let disable_all_passes = matches.get_flag("disable-all-passes");
    let enable_all_passes = matches.get_flag("enable-all-passes");
    let bpe_merges_path = matches.get_one::<String>("bpe-merges");

    // Parse individual optimization pass flags
    let enum_optimization = matches.get_flag("enum-optimization");
    let bounded_int_optimization = matches.get_flag("bounded-int-optimization");
    let pattern_optimization = matches.get_flag("pattern-optimization");
    let bit_packing = matches.get_flag("bit-packing");
    let streaming = matches.get_flag("streaming");
    let decimal_numbers = matches.get_flag("decimal-numbers");

    // Parse analysis mode arguments
    let analyze_mode = matches.get_flag("analyze");
    let output_format = required_arg::<String>(&matches, "format")?;
    let compare_sizes = matches.get_one::<String>("compare");

    // Parse metrics flag
    let show_metrics = matches.get_flag("metrics");

    // Parse target and Python-specific arguments
    let target = required_arg::<String>(&matches, "target")?;
    let python_name = matches.get_one::<String>("python-name");
    let python_version = required_arg::<String>(&matches, "python-version")?;
    let python_description = matches.get_one::<String>("python-description");
    if dry_run && target != "rust" {
        return Err("--dry-run is only supported for --target rust; Python wheel dry-run preflight is not implemented".into());
    }

    // Parse optimization level
    let optimization_level = OptimizationLevel::parse_cli(optimization_str)
        .map_err(|error| format!("{}\nUse --help for optimization profile details.", error))?;

    // Read and parse the input schema
    if !quiet {
        eprintln!("Reading schema from: {}", input_path);
    }

    let content = fs::read_to_string(input_path)
        .map_err(|e| format!("Failed to read input file '{}': {}", input_path, e))?;

    let schema: Value = serde_json::from_str(&content)
        .map_err(|e| format!("Failed to parse JSON schema: {}", e))?;

    // Handle analysis mode
    if analyze_mode {
        return run_vocabulary_analysis(&schema, input_path, output_format, compare_sizes, quiet);
    }

    // Handle Python wheel generation
    if target == "python" {
        return build_python_wheel(
            input_path,
            &schema,
            type_name,
            python_name,
            python_version,
            python_description,
            output_path,
            bpe_merges_path,
            optimization_level,
            max_vocab,
            streaming,
            decimal_numbers,
            quiet,
        );
    }

    // Determine the root type name
    let root_name = if let Some(name) = type_name {
        // User-provided name, sanitize it
        tct::utils::to_upper_camel_case(name)
    } else if let Some(title) = schema.get("title").and_then(Value::as_str) {
        // Schema title, needs sanitization for valid Rust identifier
        tct::utils::to_upper_camel_case(title)
    } else {
        // Use file stem as fallback
        let file_stem = Path::new(input_path)
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("SchemaRoot");
        tct::utils::to_upper_camel_case(file_stem)
    };

    if !quiet {
        eprintln!(
            "Generating tokenizer for type: {} with optimization level {}",
            root_name, optimization_level
        );
    }

    // Generate the tokenizer code with specified optimization configuration
    let mut compiler = TctCompiler::new(schema.clone(), &root_name)
        .map_err(|e| format!("Compiler initialization failed: {}", e))?;

    // Configure optimization passes based on command line flags
    if disable_all_passes {
        compiler = compiler.with_no_optimization();
    } else if enable_all_passes {
        compiler = compiler.with_all_optimizations();
    } else if !enabled_passes.is_empty() {
        // Add each enabled pass individually
        for pass in enabled_passes {
            compiler = compiler.with_optimization_pass(pass);
        }
    } else {
        // Use optimization level when no custom passes specified
        compiler = compiler.with_optimization_level(optimization_level);
    }

    // Apply BPE compression if merge file specified
    if let Some(bpe_path) = bpe_merges_path {
        compiler = compiler
            .with_bpe_merges(bpe_path)
            .map_err(|e| format!("Invalid BPE merge file: {}", e))?;
    }

    // Apply individual optimization pass overrides
    if enum_optimization {
        compiler = compiler.with_enum_optimization();
    }
    if bounded_int_optimization {
        compiler = compiler.with_bounded_int_optimization();
    }
    if pattern_optimization {
        compiler = compiler.with_pattern_optimization();
    }
    if bit_packing {
        compiler = compiler.with_bit_packing_optimization();
    }
    if streaming {
        compiler = compiler.with_streaming_encode();
    }
    if decimal_numbers {
        compiler = compiler.with_decimal_number_encoding();
    }
    if let Some(max_vocab) = max_vocab {
        compiler = compiler.with_max_vocabulary(max_vocab)?;
    }

    let grammar = compiler
        .compile()
        .map_err(|e| format!("Code generation failed: {}", e))?;

    // Display performance metrics if requested
    if show_metrics && !quiet {
        display_performance_metrics(&grammar);
    }

    let generated_code = grammar
        .generate_code()
        .map_err(|e| format!("Code formatting failed: {}", e))?;

    // Run benchmark if requested
    if run_benchmark {
        if !quiet {
            eprintln!("Running compression benchmark...");
        }
        run_compression_benchmark(&schema, &root_name)?;
    }

    // Output the generated code
    if dry_run {
        if !quiet {
            let output_target = output_path
                .map(|path| format!("would write to: {path}"))
                .unwrap_or_else(|| "stdout suppressed".to_string());
            eprintln!(
                "Dry run completed: generated Rust code for {} (bytes={}, sha256={}); {}",
                root_name,
                generated_code.len(),
                sha256_hex(generated_code.as_bytes()),
                output_target
            );
        }
    } else {
        match output_path {
            Some(path) => {
                fs::write(path, &generated_code)
                    .map_err(|e| format!("Failed to write output file '{}': {}", path, e))?;
                if !quiet {
                    eprintln!("Generated code written to: {}", path);
                }
            }
            None => {
                // Generated code is the only thing on stdout so that
                // `tct schema.json > tokenizer.rs` yields a valid Rust file.
                if !quiet && !run_benchmark {
                    eprintln!("--- Generated Rust Code ---");
                }
                println!("{}", generated_code);
            }
        }
    }

    Ok(())
}

fn required_arg<'a, T>(matches: &'a ArgMatches, name: &str) -> Result<&'a T, Box<dyn Error>>
where
    T: Clone + Send + Sync + 'static,
{
    matches.get_one::<T>(name).ok_or_else(|| {
        std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            format!("missing required or defaulted CLI argument {name:?}"),
        )
        .into()
    })
}

fn run_compression_benchmark(schema: &Value, type_name: &str) -> Result<(), Box<dyn Error>> {
    // Simple benchmark using a default test case
    let test_data = create_default_test_data(schema);

    // Serialize to JSON to measure baseline
    let json_str = serde_json::to_string(&test_data)?;
    let json_bytes = json_str.len();

    eprintln!("Compression Analysis:");
    eprintln!("  JSON size: {} bytes", json_bytes);
    eprintln!("  Schema type: {}", type_name);
    eprintln!(
        "  Note: measure real token counts for a corpus with the tct-measure-tokens binary (see DEVELOPMENT.md)"
    );

    Ok(())
}

fn create_default_test_data(schema: &Value) -> Value {
    // Create a minimal valid instance based on the schema
    if let Some(properties) = schema.get("properties").and_then(Value::as_object) {
        let mut obj = serde_json::Map::new();

        // Add required fields with default values
        if let Some(required) = schema.get("required").and_then(Value::as_array) {
            for req in required {
                if let Some(field_name) = req.as_str()
                    && let Some(field_schema) = properties.get(field_name)
                {
                    obj.insert(field_name.to_string(), create_default_value(field_schema));
                }
            }
        }

        serde_json::Value::Object(obj)
    } else {
        serde_json::Value::Null
    }
}

fn create_default_value(schema: &Value) -> Value {
    if let Some(type_str) = schema.get("type").and_then(Value::as_str) {
        match type_str {
            "string" => Value::String("test".to_string()),
            "number" => Value::Number(serde_json::Number::from(42)),
            "integer" => Value::Number(serde_json::Number::from(42)),
            "boolean" => Value::Bool(true),
            "array" => Value::Array(vec![]),
            "object" => Value::Object(serde_json::Map::new()),
            _ => Value::Null,
        }
    } else if schema.get("enum").is_some() {
        // Use first enum value if available
        if let Some(enums) = schema.get("enum").and_then(Value::as_array) {
            enums.first().cloned().unwrap_or(Value::Null)
        } else {
            Value::Null
        }
    } else {
        Value::Null
    }
}

fn run_vocabulary_analysis(
    schema: &Value,
    input_path: &str,
    output_format: &str,
    compare_sizes: Option<&String>,
    quiet: bool,
) -> Result<(), Box<dyn Error>> {
    if !quiet {
        eprintln!("Running vocabulary analysis for: {}", input_path);
    }

    // Parse comparison sizes if provided
    let vocab_sizes = if let Some(sizes_str) = compare_sizes {
        sizes_str
            .split(',')
            .map(|s| s.trim().parse::<usize>())
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| format!("Invalid vocabulary size in --compare: {}", e))?
    } else {
        tct::vocabulary_analyzer::DEFAULT_COMPARISON_SIZES.to_vec()
    };

    // Perform vocabulary analysis
    let mut analyzer = VocabularyAnalyzer::new();
    let analysis = analyzer
        .analyze(schema)
        .map_err(|e| format!("Vocabulary analysis failed: {}", e))?;

    // Generate recommendations
    let recommendations = analyzer.generate_recommendations(&analysis, &vocab_sizes);

    // Format and output results
    match output_format {
        "json" => {
            output_json_format(&analysis, &recommendations, input_path)?;
        }
        "text" => {
            output_text_format(&analysis, &recommendations, input_path, &vocab_sizes)?;
        }
        _ => {
            output_text_format(&analysis, &recommendations, input_path, &vocab_sizes)?;
        }
    }

    Ok(())
}

/// Output vocabulary analysis results in JSON format
fn output_json_format(
    analysis: &tct::vocabulary_analyzer::VocabularyAnalysis,
    recommendations: &tct::vocabulary_analyzer::OptimizationRecommendations,
    input_path: &str,
) -> Result<(), Box<dyn Error>> {
    let output =
        tct::vocabulary_analyzer::analysis_report_json(analysis, recommendations, input_path);
    println!("{}", serde_json::to_string_pretty(&output)?);
    Ok(())
}

/// Output vocabulary analysis results in human-readable text format
fn output_text_format(
    analysis: &tct::vocabulary_analyzer::VocabularyAnalysis,
    recommendations: &tct::vocabulary_analyzer::OptimizationRecommendations,
    input_path: &str,
    vocab_sizes: &[usize],
) -> Result<(), Box<dyn Error>> {
    println!("Schema Vocabulary Analysis for {}", input_path);
    println!("{}", "=".repeat(50 + input_path.len()));
    println!();

    // Overview section
    println!("📊 Overview");
    println!("  Total positions analyzed: {}", analysis.total_positions);
    println!(
        "  String encoding baseline: {} tokens (UTF-8: bytes 0-255 + END_MARKER)",
        analysis.string_encoding_baseline
    );
    println!(
        "  Maximum structural vocabulary: {} tokens",
        analysis.max_structural_vocab
    );
    println!(
        "  Total vocabulary required: {} tokens (max of string baseline and structural)",
        analysis.max_vocabulary_size
    );
    println!(
        "  Position requiring max vocab: {}",
        analysis.max_vocabulary_position
    );
    println!(
        "  Schema complexity score: {:.2}/1.0",
        analysis.complexity_metrics.complexity_score
    );
    println!(
        "  Average vocabulary size: {:.0} tokens",
        analysis.complexity_metrics.average_vocabulary_size
    );
    println!();

    // Schema composition
    println!("🏗️  Schema Composition");
    println!(
        "  Total properties: {}",
        analysis.complexity_metrics.total_properties
    );
    println!("  Enum types: {}", analysis.complexity_metrics.enum_count);
    println!(
        "  String types: {}",
        analysis.complexity_metrics.string_count
    );
    println!(
        "  Numeric types: {}",
        analysis.complexity_metrics.numeric_count
    );
    println!("  Array types: {}", analysis.complexity_metrics.array_count);
    println!(
        "  Maximum depth: {} levels",
        analysis.complexity_metrics.max_depth
    );
    println!();

    // Recommendations section
    if let Some(optimal_size) = recommendations.optimal_vocab_size {
        println!("🎯 Optimal Vocabulary Size: {} tokens", optimal_size);
    } else {
        println!("🎯 Optimal Vocabulary Size: Not determined (complex schema)");
    }
    println!();

    // Compression strategies
    if !recommendations.compression_strategies.is_empty() {
        println!("🔧 Recommended Compression Strategies");
        for (i, strategy) in recommendations.compression_strategies.iter().enumerate() {
            println!(
                "  {}. {} ({})",
                i + 1,
                strategy.strategy_type,
                strategy.complexity
            );
            println!("     {}", strategy.description);
            println!("     Expected savings: {}", strategy.expected_savings);
            println!();
        }
    }

    // Vocabulary efficiency analysis
    println!("📈 Vocabulary Efficiency Analysis");
    println!(
        "  {:<12} {:<15} {:<18} {:<12} {:<10}",
        "Vocab Size", "Max Required", "Constrained Pos", "Efficiency", "Multi-Token"
    );
    println!("  {}", "-".repeat(70));
    for efficiency in &recommendations.efficiency_analysis {
        println!(
            "  {:<12} {:<15} {:<18} {:<11.1}% {:<10}",
            efficiency.vocab_size,
            efficiency.max_required_vocab,
            efficiency.constrained_positions,
            efficiency.efficiency_score,
            if efficiency.multi_token_encoding_needed {
                "Yes"
            } else {
                "No"
            }
        );
    }
    println!();

    // Warnings section
    if !recommendations.warnings.is_empty() {
        println!("⚠️  Warnings");
        for (i, warning) in recommendations.warnings.iter().enumerate() {
            println!("  {}. {}", i + 1, warning);
        }
        println!();
    }

    // Summary and next steps
    println!("📝 Summary");
    println!("  Schema analysis completed successfully");
    println!(
        "  Analyzed {} vocabulary sizes: {:?}",
        vocab_sizes.len(),
        vocab_sizes
    );
    if let Some(optimal_size) = recommendations.optimal_vocab_size {
        println!("  Recommended starting point: {} tokens", optimal_size);
    }
    if !recommendations.compression_strategies.is_empty() {
        println!(
            "  {} optimization strategies available",
            recommendations.compression_strategies.len()
        );
    }

    Ok(())
}

/// Display detailed performance metrics for compilation analysis
fn display_performance_metrics(grammar: &tct::Grammar) {
    println!();
    println!("🚀 Performance Metrics");
    println!("═══════════════════════");

    // Check if performance metrics are available
    if let Some(metrics) = &grammar.performance_metrics {
        // Timing metrics
        println!();
        println!("⏱️  Compilation Timing");
        println!("   ┌─────────────────────────────────────┐");
        println!("   │ Phase              │ Time (ms)      │");
        println!("   ├─────────────────────────────────────┤");
        println!(
            "   │ Schema Analysis    │ {:>9.2}    │",
            metrics.timing.schema_analysis.as_secs_f64() * 1000.0
        );
        println!(
            "   │ Code Generation    │ {:>9.2}    │",
            metrics.timing.code_generation.as_secs_f64() * 1000.0
        );
        println!(
            "   │ Optimization       │ {:>9.2}    │",
            metrics.timing.optimization.as_secs_f64() * 1000.0
        );
        println!(
            "   │ Total Compilation  │ {:>9.2}    │",
            metrics.timing.total_time.as_secs_f64() * 1000.0
        );
        println!("   └─────────────────────────────────────┘");

        // Show individual phase timings if available
        if !metrics.timing.phase_timings.is_empty() {
            println!();
            println!("📊 Detailed Phase Breakdown");
            println!("   ┌─────────────────────────────────────┐");
            println!("   │ Component          │ Time (ms)      │");
            println!("   ├─────────────────────────────────────┤");
            for (phase, duration) in &metrics.timing.phase_timings {
                println!(
                    "   │ {:18} │ {:>9.2}    │",
                    phase,
                    duration.as_secs_f64() * 1000.0
                );
            }
            println!("   └─────────────────────────────────────┘");
        }

        // Token metrics
        println!();
        println!("🪙 Token Usage");
        println!("   ┌─────────────────────────────────────┐");
        println!("   │ Metric             │ Count          │");
        println!("   ├─────────────────────────────────────┤");
        println!(
            "   │ Total Tokens       │ {:>12}   │",
            metrics.tokens.total_tokens
        );
        println!(
            "   │ Structure Tokens   │ {:>12}   │",
            metrics.tokens.structure_tokens
        );
        println!(
            "   │ Value Tokens       │ {:>12}   │",
            metrics.tokens.value_tokens
        );
        println!(
            "   │ Tokens Saved       │ {:>12}   │",
            metrics.tokens.tokens_saved
        );
        if metrics.tokens.unoptimized_tokens > 0 {
            let compression_ratio = metrics.tokens.compression_ratio();
            println!("   │ Compression Ratio  │ {:>10.1}%   │", compression_ratio);
        }
        println!("   └─────────────────────────────────────┘");

        // Type breakdown if available
        if !metrics.tokens.type_breakdown.is_empty() {
            println!();
            println!("📋 Token Distribution by Type");
            println!("   ┌─────────────────────────────────────┐");
            println!("   │ Type               │ Tokens         │");
            println!("   ├─────────────────────────────────────┤");
            for (type_name, count) in &metrics.tokens.type_breakdown {
                println!("   │ {:18} │ {:>12}   │", type_name, count);
            }
            println!("   └─────────────────────────────────────┘");
        }

        // Complexity metrics
        println!();
        println!("🧩 Schema Complexity");
        println!("   ┌─────────────────────────────────────┐");
        println!("   │ Metric             │ Count          │");
        println!("   ├─────────────────────────────────────┤");
        println!(
            "   │ Object Types       │ {:>12}   │",
            metrics.complexity.object_count
        );
        println!(
            "   │ Enum Types         │ {:>12}   │",
            metrics.complexity.enum_count
        );
        println!(
            "   │ Max Schema Depth   │ {:>12}   │",
            metrics.complexity.max_depth
        );
        println!(
            "   │ Constraints        │ {:>12}   │",
            metrics.complexity.constraint_count
        );
        println!(
            "   │ Additional Props   │ {:>12}   │",
            metrics.complexity.additional_properties_count
        );
        println!(
            "   │ Unconstrained AP   │ {:>12}   │",
            metrics.complexity.unconstrained_additional_properties
        );
        println!("   └─────────────────────────────────────┘");
    } else {
        println!("⚠️  Performance metrics not available");
        println!("    Metrics collection may not be enabled or compilation failed");
    }

    // Vocabulary optimization stats
    if let Some(vocab_stats) = &grammar.vocabulary_stats {
        println!();
        println!("🎯 Vocabulary Optimization");
        println!("   ┌─────────────────────────────────────┐");
        println!("   │ Metric             │ Value          │");
        println!("   ├─────────────────────────────────────┤");
        println!(
            "   │ Optimized Fields   │ {:>12}   │",
            vocab_stats.optimized_fields
        );
        println!(
            "   │ Constrained Values │ {:>12}   │",
            vocab_stats.total_constrained_values
        );
        println!(
            "   │ Naive Vocab Size   │ {:>12}   │",
            vocab_stats.naive_vocabulary_size
        );
        println!(
            "   │ Optimized Size     │ {:>12}   │",
            vocab_stats.distinct_structural_tokens_count
        );
        println!(
            "   │ Tokens Saved       │ {:>12}   │",
            vocab_stats.tokens_saved
        );
        println!(
            "   │ Reduction %        │ {:>10.1}%   │",
            vocab_stats.reduction_percentage
        );
        println!("   └─────────────────────────────────────┘");
    }

    // Warnings summary
    if let Some(warnings) = &grammar.warnings
        && !warnings.warnings.is_empty()
    {
        println!();
        println!(
            "⚠️  Schema Quality Warnings ({} total)",
            warnings.warnings.len()
        );
        for (i, warning) in warnings.warnings.iter().enumerate() {
            println!(
                "   {}. {} - {}",
                i + 1,
                warning.warning_type,
                warning.message
            );
            if !warning.suggestion.is_empty() {
                println!("      💡 {}", warning.suggestion);
            }
        }
    }

    println!();
}

/// Build Python wheel package for the schema
#[allow(clippy::too_many_arguments)]
fn build_python_wheel(
    input_path: &str,
    _schema: &Value,
    type_name: Option<&String>,
    python_name: Option<&String>,
    python_version: &str,
    python_description: Option<&String>,
    output_path: Option<&String>,
    bpe_merges_path: Option<&String>,
    optimization_level: OptimizationLevel,
    max_vocab: Option<usize>,
    streaming: bool,
    decimal_numbers: bool,
    quiet: bool,
) -> Result<(), Box<dyn Error>> {
    let config = python_wheel_config(PythonWheelOptions {
        input_path,
        type_name,
        python_name,
        python_version,
        python_description,
        output_path,
        bpe_merges_path,
        optimization_level,
        max_vocab,
        streaming,
        decimal_numbers,
        quiet,
    });

    // Build the wheel
    let wheel_path = tct::wheel_builder::build_wheel(&config)?;

    if !quiet {
        eprintln!("\n✅ Python wheel built successfully!");
        eprintln!("   Wheel: {}", wheel_path.display());
    }

    Ok(())
}

/// CLI options that shape a Python wheel build.
struct PythonWheelOptions<'a> {
    input_path: &'a str,
    type_name: Option<&'a String>,
    python_name: Option<&'a String>,
    python_version: &'a str,
    python_description: Option<&'a String>,
    output_path: Option<&'a String>,
    bpe_merges_path: Option<&'a String>,
    optimization_level: OptimizationLevel,
    max_vocab: Option<usize>,
    streaming: bool,
    decimal_numbers: bool,
    quiet: bool,
}

/// Translate CLI options into a `WheelConfig`.
///
/// Kept separate from the build so the flag wiring is testable: a flag that is
/// parsed but never forwarded produces a wheel that silently contradicts what
/// the operator asked for.
fn python_wheel_config(options: PythonWheelOptions<'_>) -> tct::wheel_builder::WheelConfig {
    // Determine package name
    let package_name = if let Some(name) = options.python_name {
        name.clone()
    } else {
        // Auto-generate from schema filename: "github-workflow.json" -> "tct-github-workflow"
        let file_stem = Path::new(options.input_path)
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("schema");
        format!("tct-{}", file_stem.to_lowercase().replace('_', "-"))
    };

    // Determine root type name
    let root_type_name = if let Some(name) = options.type_name {
        tct::utils::to_upper_camel_case(name)
    } else {
        // Use package name without "tct-" prefix: "tct-github-workflow" -> "GithubWorkflow"
        let base_name = package_name.strip_prefix("tct-").unwrap_or(&package_name);
        tct::utils::to_upper_camel_case(base_name)
    };

    // Determine output directory
    let output_dir = options
        .output_path
        .cloned()
        .unwrap_or_else(|| "wheels".to_string());

    let mut config =
        tct::wheel_builder::WheelConfig::new(options.input_path, &root_type_name, &package_name)
            .with_version(options.python_version)
            .with_output_dir(&output_dir)
            .with_optimization_level(options.optimization_level)
            .with_quiet(options.quiet);

    if let Some(max_vocab) = options.max_vocab {
        config = config.with_max_vocabulary(max_vocab);
    }

    // Add optional fields
    if let Some(description) = options.python_description {
        config = config.with_description(description);
    }

    if options.streaming {
        config = config.with_streaming_encode(true);
    }

    if options.decimal_numbers {
        config = config.with_decimal_number_encoding();
    }

    if let Some(bpe_path) = options.bpe_merges_path {
        config = config.with_bpe_merges(bpe_path);
    }

    config
}

#[cfg(test)]
mod tests {
    use super::*;

    fn options(input_path: &'static str) -> PythonWheelOptions<'static> {
        PythonWheelOptions {
            input_path,
            type_name: None,
            python_name: None,
            python_version: "1.0.0",
            python_description: None,
            output_path: None,
            bpe_merges_path: None,
            optimization_level: OptimizationLevel::O3,
            max_vocab: None,
            streaming: false,
            decimal_numbers: false,
            quiet: true,
        }
    }

    /// Regression: `--decimal-numbers` was parsed and applied to the Rust target
    /// but never forwarded to `build_python_wheel`, so `--target python
    /// --decimal-numbers` silently produced a wheel with
    /// `decimal_number_encoding: false`. That is the worst shape of wiring bug --
    /// the build succeeds and the artifact contradicts the request, which is
    /// exactly what the stale-wheel guard exists to prevent.
    #[test]
    fn decimal_numbers_reaches_the_python_wheel_config() {
        let config = python_wheel_config(PythonWheelOptions {
            decimal_numbers: true,
            ..options("schemas/popular/package.json")
        });

        assert!(
            config.decimal_number_encoding,
            "--decimal-numbers must reach the wheel config"
        );
    }

    #[test]
    fn decimal_numbers_defaults_off() {
        let config = python_wheel_config(options("demo.json"));
        assert!(!config.decimal_number_encoding);
    }

    #[test]
    fn streaming_and_bpe_flags_reach_the_python_wheel_config() {
        let merges = "schemas/bpe/agent-calls-v2-bpe-512.json".to_string();
        let config = python_wheel_config(PythonWheelOptions {
            streaming: true,
            bpe_merges_path: Some(&merges),
            ..options("demo.json")
        });

        assert!(config.streaming_encode);
        assert_eq!(config.bpe_merges_path.as_deref(), Some(merges.as_str()));
    }

    #[test]
    fn max_vocab_reaches_the_python_wheel_config() {
        let config = python_wheel_config(PythonWheelOptions {
            max_vocab: Some(257),
            ..options("demo.json")
        });
        assert_eq!(config.max_vocab, Some(257));
    }
}
