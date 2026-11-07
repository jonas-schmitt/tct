//! BPE-mode helper binary template for instance encoding.

pub(super) fn render_bpe_helper_main(
    code_str: &str,
    allocation_tracking_code: &str,
    root_type: &str,
    decode_prefix_batch_arm: &str,
) -> String {
    format!(
        r##"{}

{}

use std::env;
use std::fs;
use std::io::{{self, BufRead, Read, Write}};
use std::time::Instant;
use crate::tokenizer::JsonTokenEncoder;

fn decode_success_record(index: usize, json_value: serde_json::Value) -> String {{
    let mut record = serde_json::Map::new();
    record.insert("index".to_string(), serde_json::json!(index));
    record.insert("json".to_string(), json_value);
    serde_json::Value::Object(record).to_string()
}}

fn decode_error_record(index: usize, error: impl ToString) -> String {{
    let mut record = serde_json::Map::new();
    record.insert("index".to_string(), serde_json::json!(index));
    record.insert("error".to_string(), serde_json::Value::String(error.to_string()));
    serde_json::Value::Object(record).to_string()
}}

fn token_success_record(index: usize, tokens: Vec<u32>) -> String {{
    let mut record = serde_json::Map::new();
    record.insert("index".to_string(), serde_json::json!(index));
    record.insert("tokens".to_string(), serde_json::json!(tokens));
    serde_json::Value::Object(record).to_string()
}}

fn token_error_record(index: usize, error: impl ToString) -> String {{
    let mut record = serde_json::Map::new();
    record.insert("index".to_string(), serde_json::json!(index));
    record.insert("error".to_string(), serde_json::Value::String(error.to_string()));
    serde_json::Value::Object(record).to_string()
}}

fn main() -> Result<(), Box<dyn std::error::Error>> {{
    let args: Vec<String> = env::args().collect();

    if args.len() < 2 {{
        eprintln!("Usage: {{}} <encode|decode|encode-batch|encode-batch-profile|encode-batch-hot-path|encode-batch-validation-profile|decode-prefix-batch|decode-batch|bpe-encode-batch|bpe-decode-batch|roles-batch> [instance.json]", args[0]);
        std::process::exit(1);
    }}

    match args[1].as_str() {{
        "encode" => {{
            if args.len() != 3 {{
                eprintln!("Usage: {{}} encode <instance.json>", args[0]);
                std::process::exit(1);
            }}
            let allocation_tracking = start_allocation_tracking();

            // Read and deserialize JSON instance
            let content = fs::read_to_string(&args[2])?;
            let instance: {} = serde_json::from_str(&content)?;

            // Encode using TCT tokenizer with BPE compression
            let tokens = codec::encode(&instance);

            // Output tokens as JSON array
            println!("{{}}", serde_json::to_string(&tokens)?);
            finish_allocation_tracking("encode", allocation_tracking);
        }}
        "encode-batch" => {{
            let allocation_tracking = start_allocation_tracking();
            // Parallel batch mode: read file paths from stdin, process with rayon, output JSONL
            use rayon::prelude::*;

            // Collect all paths first
            let paths: Vec<String> = io::stdin()
                .lock()
                .lines()
                .filter_map(|l| l.ok())
                .filter(|p| !p.trim().is_empty())
                .collect();

            // Process in parallel using rayon
            let operation_start = std::time::Instant::now();
            let results: Vec<String> = paths
                .par_iter()
                .map(|path| {{
                    match fs::read_to_string(path) {{
                        Ok(content) => {{
                            match serde_json::from_str::<{}>(&content) {{
                                Ok(instance) => {{
                                    let tokens = codec::encode(&instance);
                                    format!(r#"{{{{"path":"{{}}","tokens":{{}}}}}}"#,
                                        path.replace('\\', "\\\\").replace('"', "\\\""),
                                        serde_json::to_string(&tokens).unwrap_or_default())
                                }}
                                Err(e) => {{
                                    format!(r#"{{{{"path":"{{}}","error":"parse: {{}}"}}}}"#,
                                        path.replace('\\', "\\\\").replace('"', "\\\""),
                                        e.to_string().replace('\\', "\\\\").replace('"', "\\\""))
                                }}
                            }}
                        }}
                        Err(e) => {{
                            format!(r#"{{{{"path":"{{}}","error":"read: {{}}"}}}}"#,
                                path.replace('\\', "\\\\").replace('"', "\\\""),
                                e.to_string().replace('\\', "\\\\").replace('"', "\\\""))
                        }}
                    }}
                }})
                .collect();
            let wall_time_ms = operation_start.elapsed().as_millis();

            // Output all results
            let stdout = io::stdout();
            let mut stdout_lock = stdout.lock();
            for result in results {{
                writeln!(stdout_lock, "{{}}", result)?;
            }}
            finish_allocation_tracking("encode-batch", allocation_tracking);
            finish_batch_timing("encode-batch", wall_time_ms);
        }}
        "encode-batch-profile" => {{
            // Profiling sidecar mode: keep normal encode-batch timings stable while
            // attributing the same token output to child-process phases.
            use rayon::prelude::*;
            let profile_start = std::time::Instant::now();

            let path_input_start = std::time::Instant::now();
            let paths: Vec<String> = io::stdin()
                .lock()
                .lines()
                .filter_map(|l| l.ok())
                .filter(|p| !p.trim().is_empty())
                .collect();
            let path_input_ns = elapsed_ns_u64(path_input_start);

            let profiled_results: Vec<(String, TctEncodeRecordProfile)> = paths
                .par_iter()
                .map(|path| {{
                    let mut profile = TctEncodeRecordProfile::default();
                    let read_parse_start = std::time::Instant::now();
                    let result = match fs::read_to_string(path) {{
                        Ok(content) => {{
                            match serde_json::from_str::<{}>(&content) {{
                                Ok(instance) => {{
                                    profile.json_read_parse_ns = elapsed_ns_u64(read_parse_start);

                                    let base_encode_start = std::time::Instant::now();
                                    let mut base_tokens = Vec::new();
                                    instance.encode(&mut base_tokens);
                                    profile.base_encode_ns = elapsed_ns_u64(base_encode_start);

                                    let bpe_encode_start = std::time::Instant::now();
                                    let tokens = bpe::encode_owned(base_tokens, VOCAB_SIZE);
                                    profile.bpe_encode_ns = elapsed_ns_u64(bpe_encode_start);

                                    let serialization_start = std::time::Instant::now();
                                    let record = format!(r#"{{{{"path":"{{}}","tokens":{{}}}}}}"#,
                                        path.replace('\\', "\\\\").replace('"', "\\\""),
                                        serde_json::to_string(&tokens).unwrap_or_default());
                                    profile.output_serialization_ns =
                                        elapsed_ns_u64(serialization_start);
                                    record
                                }}
                                Err(e) => {{
                                    profile.failed = true;
                                    profile.json_read_parse_ns = elapsed_ns_u64(read_parse_start);
                                    let serialization_start = std::time::Instant::now();
                                    let record = format!(r#"{{{{"path":"{{}}","error":"parse: {{}}"}}}}"#,
                                        path.replace('\\', "\\\\").replace('"', "\\\""),
                                        e.to_string().replace('\\', "\\\\").replace('"', "\\\""));
                                    profile.output_serialization_ns =
                                        elapsed_ns_u64(serialization_start);
                                    record
                                }}
                            }}
                        }}
                        Err(e) => {{
                            profile.failed = true;
                            profile.json_read_parse_ns = elapsed_ns_u64(read_parse_start);
                            let serialization_start = std::time::Instant::now();
                            let record = format!(r#"{{{{"path":"{{}}","error":"read: {{}}"}}}}"#,
                                path.replace('\\', "\\\\").replace('"', "\\\""),
                                e.to_string().replace('\\', "\\\\").replace('"', "\\\""));
                            profile.output_serialization_ns =
                                elapsed_ns_u64(serialization_start);
                            record
                        }}
                    }};
                    (result, profile)
                }})
                .collect();

            let mut batch_profile = TctEncodeBatchProfile::default();
            for (_, profile) in &profiled_results {{
                batch_profile.add_record(*profile);
            }}

            let stdout = io::stdout();
            let mut stdout_lock = stdout.lock();
            let stdout_write_start = std::time::Instant::now();
            for (result, _) in profiled_results {{
                writeln!(stdout_lock, "{{}}", result)?;
            }}
            let stdout_write_ns = elapsed_ns_u64(stdout_write_start);
            let profile_wall_time_ns = elapsed_ns_u64(profile_start);
            finish_encode_phase_metrics(
                "encode-batch",
                profile_wall_time_ns,
                path_input_ns,
                batch_profile,
                stdout_write_ns,
                true,
            );
        }}
        "encode-batch-hot-path" => {{
            // Hot-path sidecar mode: profile JSON-to-token work without emitting
            // per-instance token JSONL.
            use rayon::prelude::*;
            let profile_start = std::time::Instant::now();

            let path_input_start = std::time::Instant::now();
            let paths: Vec<String> = io::stdin()
                .lock()
                .lines()
                .filter_map(|l| l.ok())
                .filter(|p| !p.trim().is_empty())
                .collect();
            let path_input_ns = elapsed_ns_u64(path_input_start);
            let allocation_tracking = start_allocation_tracking();

            let profiled_results: Vec<(Option<Vec<u32>>, TctEncodeRecordProfile)> = paths
                .par_iter()
                .map(|path| {{
                    let mut profile = TctEncodeRecordProfile::default();
                    let read_parse_start = std::time::Instant::now();
                    let tokens = match fs::read_to_string(path) {{
                        Ok(content) => {{
                            match serde_json::from_str::<{}>(&content) {{
                                Ok(instance) => {{
                                    profile.json_read_parse_ns = elapsed_ns_u64(read_parse_start);

                                    let base_encode_start = std::time::Instant::now();
                                    let mut base_tokens = Vec::new();
                                    instance.encode(&mut base_tokens);
                                    profile.base_encode_ns = elapsed_ns_u64(base_encode_start);

                                    let bpe_encode_start = std::time::Instant::now();
                                    let tokens = bpe::encode_owned(base_tokens, VOCAB_SIZE);
                                    profile.bpe_encode_ns = elapsed_ns_u64(bpe_encode_start);
                                    Some(tokens)
                                }}
                                Err(_) => {{
                                    profile.failed = true;
                                    profile.json_read_parse_ns = elapsed_ns_u64(read_parse_start);
                                    None
                                }}
                            }}
                        }}
                        Err(_) => {{
                            profile.failed = true;
                            profile.json_read_parse_ns = elapsed_ns_u64(read_parse_start);
                            None
                        }}
                    }};
                    (tokens, profile)
                }})
                .collect();

            let mut batch_profile = TctEncodeBatchProfile::default();
            let mut token_count: u64 = 0;
            let mut token_checksum = TCT_TOKEN_CHECKSUM_OFFSET;
            for (tokens, profile) in &profiled_results {{
                batch_profile.add_record(*profile);
                if let Some(tokens) = tokens {{
                    token_count = token_count.saturating_add(tokens.len() as u64);
                    token_checksum = update_tct_token_checksum(token_checksum, tokens);
                }}
            }}
            let profile_wall_time_ns = elapsed_ns_u64(profile_start);
            finish_encode_hot_path_metrics(
                "encode-batch",
                profile_wall_time_ns,
                path_input_ns,
                batch_profile,
                token_count,
                token_checksum,
                true,
            );
            finish_allocation_tracking("encode-batch-hot-path", allocation_tracking);
        }}
        "encode-batch-validation-profile" => {{
            // Diagnostic sidecar mode: split file read, JSON Value parsing, and
            // typed serde validation/deserialization before token encoding.
            use rayon::prelude::*;
            let profile_start = std::time::Instant::now();

            let path_input_start = std::time::Instant::now();
            let paths: Vec<String> = io::stdin()
                .lock()
                .lines()
                .filter_map(|l| l.ok())
                .filter(|p| !p.trim().is_empty())
                .collect();
            let path_input_ns = elapsed_ns_u64(path_input_start);

            let profiled_results: Vec<(Option<Vec<u32>>, TctEncodeRecordProfile)> = paths
                .par_iter()
                .map(|path| {{
                    let mut profile = TctEncodeRecordProfile::default();
                    let file_read_start = std::time::Instant::now();
                    let tokens = match fs::read_to_string(path) {{
                        Ok(content) => {{
                            profile.file_read_ns = elapsed_ns_u64(file_read_start);

                            let json_parse_start = std::time::Instant::now();
                            match serde_json::from_str::<serde_json::Value>(&content) {{
                                Ok(value) => {{
                                    profile.json_value_parse_ns =
                                        elapsed_ns_u64(json_parse_start);

                                    let from_value_start = std::time::Instant::now();
                                    match serde_json::from_value::<{}>(value) {{
                                        Ok(instance) => {{
                                            profile.serde_from_value_ns =
                                                elapsed_ns_u64(from_value_start);

                                            let base_encode_start = std::time::Instant::now();
                                            let mut base_tokens = Vec::new();
                                            instance.encode(&mut base_tokens);
                                            profile.base_encode_ns =
                                                elapsed_ns_u64(base_encode_start);

                                            let bpe_encode_start = std::time::Instant::now();
                                            let tokens = bpe::encode_owned(base_tokens, VOCAB_SIZE);
                                            profile.bpe_encode_ns =
                                                elapsed_ns_u64(bpe_encode_start);
                                            Some(tokens)
                                        }}
                                        Err(_) => {{
                                            profile.failed = true;
                                            profile.serde_from_value_ns =
                                                elapsed_ns_u64(from_value_start);
                                            None
                                        }}
                                    }}
                                }}
                                Err(_) => {{
                                    profile.failed = true;
                                    profile.json_value_parse_ns =
                                        elapsed_ns_u64(json_parse_start);
                                    None
                                }}
                            }}
                        }}
                        Err(_) => {{
                            profile.failed = true;
                            profile.file_read_ns = elapsed_ns_u64(file_read_start);
                            None
                        }}
                    }};
                    (tokens, profile)
                }})
                .collect();

            let mut batch_profile = TctEncodeBatchProfile::default();
            let mut token_count: u64 = 0;
            let mut token_checksum = TCT_TOKEN_CHECKSUM_OFFSET;
            for (tokens, profile) in &profiled_results {{
                batch_profile.add_record(*profile);
                if let Some(tokens) = tokens {{
                    token_count = token_count.saturating_add(tokens.len() as u64);
                    token_checksum = update_tct_token_checksum(token_checksum, tokens);
                }}
            }}
            let profile_wall_time_ns = elapsed_ns_u64(profile_start);
            finish_encode_validation_phase_metrics(
                "encode-batch",
                profile_wall_time_ns,
                path_input_ns,
                batch_profile,
                token_count,
                token_checksum,
                true,
            );
        }}
        {}
        "decode" => {{
            let allocation_tracking = start_allocation_tracking();
            // Read tokens from stdin as JSON array
            let mut buffer = String::new();
            io::stdin().read_to_string(&mut buffer)?;
            let tokens: Vec<u32> = serde_json::from_str(&buffer)?;

            // Decode using TCT tokenizer with BPE decompression
            let (instance, _, _) = codec::decode::<{}>(tokens)?;

            // Output decoded JSON
            println!("{{}}", serde_json::to_string_pretty(&instance)?);
            finish_allocation_tracking("decode", allocation_tracking);
        }}
        "decode-batch" => {{
            let allocation_tracking = start_allocation_tracking();
            // Parallel batch mode: read token arrays from stdin, process with rayon, output JSONL
            use rayon::prelude::*;

            let lines: Vec<String> = io::stdin()
                .lock()
                .lines()
                .filter_map(|l| l.ok())
                .filter(|line| !line.trim().is_empty())
                .collect();

            let operation_start = std::time::Instant::now();
            let results: Vec<String> = lines
                .par_iter()
                .enumerate()
                .map(|(index, line)| {{
                    match serde_json::from_str::<Vec<u32>>(line) {{
                        Ok(tokens) => match codec::decode::<{}>(tokens) {{
                            Ok((instance, _, _)) => match serde_json::to_value(&instance) {{
                                Ok(json_value) => decode_success_record(index, json_value),
                                Err(e) => decode_error_record(index, format!("serialize: {{}}", e)),
                            }},
                            Err(e) => decode_error_record(index, format!("decode: {{}}", e)),
                        }},
                        Err(e) => decode_error_record(index, format!("parse: {{}}", e)),
                    }}
                }})
                .collect();
            let wall_time_ms = operation_start.elapsed().as_millis();

            let stdout = io::stdout();
            let mut stdout_lock = stdout.lock();
            for result in results {{
                writeln!(stdout_lock, "{{}}", result)?;
            }}
            finish_allocation_tracking("decode-batch", allocation_tracking);
            finish_batch_timing("decode-batch", wall_time_ms);
        }}
        "bpe-encode-batch" => {{
            use rayon::prelude::*;
            let warmup_iterations = args
                .get(2)
                .map(|value| value.parse::<u32>())
                .transpose()?
                .unwrap_or(0);

            let records: Vec<(usize, Result<Vec<u32>, String>)> = io::stdin()
                .lock()
                .lines()
                .filter_map(|l| l.ok())
                .filter(|line| !line.trim().is_empty())
                .enumerate()
                .map(|(index, line)| {{
                    let parsed = serde_json::from_str::<Vec<u32>>(&line)
                        .map_err(|e| format!("parse: {{}}", e));
                    (index, parsed)
                }})
                .collect();

            for _ in 0..warmup_iterations {{
                let warmed: Vec<Vec<u32>> = records
                    .par_iter()
                    .filter_map(|(_, parsed)| parsed.as_ref().ok())
                    .map(|tokens| bpe::encode_owned(tokens.clone(), VOCAB_SIZE))
                    .collect();
                std::hint::black_box(&warmed);
            }}

            let allocation_tracking = start_allocation_tracking();
            let operation_start = Instant::now();
            let results: Vec<(usize, Result<Vec<u32>, String>)> = records
                .into_par_iter()
                .map(|(index, parsed)| {{
                    let result = parsed.map(|tokens| bpe::encode_owned(tokens, VOCAB_SIZE));
                    (index, result)
                }})
                .collect();
            let wall_time_ms = operation_start.elapsed().as_millis();
            finish_allocation_tracking("bpe-encode-batch", allocation_tracking);
            finish_batch_timing("bpe-encode-batch", wall_time_ms);

            let stdout = io::stdout();
            let mut stdout_lock = stdout.lock();
            for (index, result) in results {{
                let record = match result {{
                    Ok(tokens) => token_success_record(index, tokens),
                    Err(error) => token_error_record(index, error),
                }};
                writeln!(stdout_lock, "{{}}", record)?;
            }}
        }}
        "bpe-decode-batch" => {{
            use rayon::prelude::*;
            let warmup_iterations = args
                .get(2)
                .map(|value| value.parse::<u32>())
                .transpose()?
                .unwrap_or(0);

            let records: Vec<(usize, Result<Vec<u32>, String>)> = io::stdin()
                .lock()
                .lines()
                .filter_map(|l| l.ok())
                .filter(|line| !line.trim().is_empty())
                .enumerate()
                .map(|(index, line)| {{
                    let parsed = serde_json::from_str::<Vec<u32>>(&line)
                        .map_err(|e| format!("parse: {{}}", e));
                    (index, parsed)
                }})
                .collect();

            for _ in 0..warmup_iterations {{
                let warmed: Vec<Vec<u32>> = records
                    .par_iter()
                    .filter_map(|(_, parsed)| parsed.as_ref().ok())
                    .map(|tokens| bpe::decode(tokens, VOCAB_SIZE))
                    .collect();
                std::hint::black_box(&warmed);
            }}

            let allocation_tracking = start_allocation_tracking();
            let operation_start = Instant::now();
            let results: Vec<(usize, Result<Vec<u32>, String>)> = records
                .par_iter()
                .map(|(index, parsed)| {{
                    let result = parsed
                        .as_ref()
                        .map(|tokens| bpe::decode(tokens, VOCAB_SIZE))
                        .map_err(|error| error.clone());
                    (*index, result)
                }})
                .collect();
            let wall_time_ms = operation_start.elapsed().as_millis();
            finish_allocation_tracking("bpe-decode-batch", allocation_tracking);
            finish_batch_timing("bpe-decode-batch", wall_time_ms);

            let stdout = io::stdout();
            let mut stdout_lock = stdout.lock();
            for (index, result) in results {{
                let record = match result {{
                    Ok(tokens) => token_success_record(index, tokens),
                    Err(error) => token_error_record(index, error),
                }};
                writeln!(stdout_lock, "{{}}", record)?;
            }}
        }}
        "roles-batch" => {{
            // Base-token role export (BPE helper still uses raw role_export).
            use rayon::prelude::*;

            let lines: Vec<String> = io::stdin()
                .lock()
                .lines()
                .filter_map(|l| l.ok())
                .filter(|line| !line.trim().is_empty())
                .collect();

            let operation_start = Instant::now();
            let results: Vec<String> = lines
                .par_iter()
                .enumerate()
                .map(|(index, line)| {{
                    match serde_json::from_str::<Vec<u32>>(line) {{
                        Ok(tokens) => {{
                            match tokenizer::role_export::token_roles::<{}>(&tokens) {{
                                Ok(roles) => {{
                                    let mut record = serde_json::Map::new();
                                    record.insert("index".to_string(), serde_json::json!(index));
                                    record.insert("roles".to_string(), serde_json::json!(roles));
                                    serde_json::Value::Object(record).to_string()
                                }}
                                Err(e) => token_error_record(index, format!("roles: {{}}", e)),
                            }}
                        }}
                        Err(e) => token_error_record(index, format!("parse: {{}}", e)),
                    }}
                }})
                .collect();
            let wall_time_ms = operation_start.elapsed().as_millis();

            let stdout = io::stdout();
            let mut stdout_lock = stdout.lock();
            for result in results {{
                writeln!(stdout_lock, "{{}}", result)?;
            }}
            finish_batch_timing("roles-batch", wall_time_ms);
        }}
        _ => {{
            eprintln!("Error: First argument must be 'encode', 'decode', 'encode-batch', 'encode-batch-profile', 'encode-batch-hot-path', 'encode-batch-validation-profile', 'decode-prefix-batch', 'decode-batch', 'bpe-encode-batch', 'bpe-decode-batch', or 'roles-batch'");
            std::process::exit(1);
        }}
    }}

    Ok(())
}}
"##,
        code_str,
        allocation_tracking_code,
        root_type,
        root_type,
        root_type,
        root_type,
        root_type,
        decode_prefix_batch_arm,
        root_type,
        root_type,
        root_type
    )
}
