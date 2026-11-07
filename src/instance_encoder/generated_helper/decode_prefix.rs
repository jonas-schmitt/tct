//! Renders generated-helper commands for prefix decoding profiles.
//!
//! Prefix profiling runs inside the temporary generated helper so it exercises
//! the compiled schema-specific `decode_prefix` implementation. The rendered
//! match arm accepts complete token records, evaluates selected prefix cuts, and
//! records consumed-token and partial-field metrics for raw and BPE-backed
//! helper builds.

pub(super) fn render_decode_prefix_batch_arm(
    root_type: &str,
    has_bpe: bool,
    supports_decode_prefix: bool,
) -> String {
    if !supports_decode_prefix {
        return r#"
        "decode-prefix-batch" => {
            eprintln!("Error: decode-prefix-batch requires generated decode_prefix support; rebuild the helper with streaming encode enabled");
            std::process::exit(2);
        }
"#
        .to_string();
    }

    let arm = if has_bpe {
        r#"
        "decode-prefix-batch" => {
            // Prefix sidecar mode: expand BPE to raw TCT tokens before taking
            // prefix cuts. This does not claim BPE-prefix streaming support.
            let warmup_iterations = args
                .get(2)
                .map(|value| value.parse::<u32>())
                .transpose()?
                .unwrap_or(0);

            let token_input_start = std::time::Instant::now();
            let records: Vec<Result<Vec<u32>, ()>> = io::stdin()
                .lock()
                .lines()
                .filter_map(|l| l.ok())
                .filter(|line| !line.trim().is_empty())
                .map(|line| serde_json::from_str::<Vec<u32>>(&line).map_err(|_| ()))
                .collect();
            let token_input_ns = elapsed_ns_u64(token_input_start);

            for _ in 0..warmup_iterations {
                for tokens in records.iter().filter_map(|record| record.as_ref().ok()) {
                    let base_tokens = bpe::decode(tokens, VOCAB_SIZE);
                    for cut in prefix_cut_lengths(base_tokens.len()) {
                        let mut iter = base_tokens[..cut].iter().copied();
                        let result = __ROOT_TYPE__::decode_prefix(&mut iter);
                        std::hint::black_box(result.complete);
                        std::hint::black_box(result.fields_decoded);
                        std::hint::black_box(iter.count());
                    }
                }
            }

            let profile_start = std::time::Instant::now();
            let mut profile = TctDecodePrefixBatchProfile::default();
            for record in records {
                let tokens = match record {
                    Ok(tokens) => tokens,
                    Err(_) => {
                        profile.record_failure();
                        continue;
                    }
                };
                profile.records = profile.records.saturating_add(1);

                let bpe_decode_start = std::time::Instant::now();
                let base_tokens = bpe::decode(&tokens, VOCAB_SIZE);
                profile.bpe_decode_ns = profile
                    .bpe_decode_ns
                    .saturating_add(elapsed_ns_u64(bpe_decode_start));

                for cut in prefix_cut_lengths(base_tokens.len()) {
                    profile.prefix_cuts = profile.prefix_cuts.saturating_add(1);
                    profile.total_prefix_tokens = profile
                        .total_prefix_tokens
                        .saturating_add(cut as u64);
                    let mut iter = base_tokens[..cut].iter().copied();
                    let decode_prefix_start = std::time::Instant::now();
                    let result = __ROOT_TYPE__::decode_prefix(&mut iter);
                    profile.decode_prefix_ns = profile
                        .decode_prefix_ns
                        .saturating_add(elapsed_ns_u64(decode_prefix_start));
                    let remaining = iter.count();
                    profile.consumed_tokens = profile
                        .consumed_tokens
                        .saturating_add(cut.saturating_sub(remaining) as u64);
                    profile.fields_decoded_sum = profile
                        .fields_decoded_sum
                        .saturating_add(result.fields_decoded as u64);
                    if result.partial_field.is_some() {
                        profile.partial_fields_reported =
                            profile.partial_fields_reported.saturating_add(1);
                    }
                    if result.complete {
                        profile.complete_prefixes = profile.complete_prefixes.saturating_add(1);
                        if cut == base_tokens.len() {
                            profile.full_sequences_complete =
                                profile.full_sequences_complete.saturating_add(1);
                        }
                    } else {
                        profile.partial_prefixes = profile.partial_prefixes.saturating_add(1);
                    }
                }
            }

            finish_decode_prefix_metrics(
                "decode-prefix-batch",
                elapsed_ns_u64(profile_start),
                token_input_ns,
                profile,
                true,
            );
        }
"#
    } else {
        r#"
        "decode-prefix-batch" => {
            // Prefix sidecar mode: profile generated decode_prefix over full and
            // truncated raw TCT token sequences.
            let warmup_iterations = args
                .get(2)
                .map(|value| value.parse::<u32>())
                .transpose()?
                .unwrap_or(0);

            let token_input_start = std::time::Instant::now();
            let records: Vec<Result<Vec<u32>, ()>> = io::stdin()
                .lock()
                .lines()
                .filter_map(|l| l.ok())
                .filter(|line| !line.trim().is_empty())
                .map(|line| serde_json::from_str::<Vec<u32>>(&line).map_err(|_| ()))
                .collect();
            let token_input_ns = elapsed_ns_u64(token_input_start);

            for _ in 0..warmup_iterations {
                for base_tokens in records.iter().filter_map(|record| record.as_ref().ok()) {
                    for cut in prefix_cut_lengths(base_tokens.len()) {
                        let mut iter = base_tokens[..cut].iter().copied();
                        let result = __ROOT_TYPE__::decode_prefix(&mut iter);
                        std::hint::black_box(result.complete);
                        std::hint::black_box(result.fields_decoded);
                        std::hint::black_box(iter.count());
                    }
                }
            }

            let profile_start = std::time::Instant::now();
            let mut profile = TctDecodePrefixBatchProfile::default();
            for record in records {
                let base_tokens = match record {
                    Ok(tokens) => tokens,
                    Err(_) => {
                        profile.record_failure();
                        continue;
                    }
                };
                profile.records = profile.records.saturating_add(1);

                for cut in prefix_cut_lengths(base_tokens.len()) {
                    profile.prefix_cuts = profile.prefix_cuts.saturating_add(1);
                    profile.total_prefix_tokens = profile
                        .total_prefix_tokens
                        .saturating_add(cut as u64);
                    let mut iter = base_tokens[..cut].iter().copied();
                    let decode_prefix_start = std::time::Instant::now();
                    let result = __ROOT_TYPE__::decode_prefix(&mut iter);
                    profile.decode_prefix_ns = profile
                        .decode_prefix_ns
                        .saturating_add(elapsed_ns_u64(decode_prefix_start));
                    let remaining = iter.count();
                    profile.consumed_tokens = profile
                        .consumed_tokens
                        .saturating_add(cut.saturating_sub(remaining) as u64);
                    profile.fields_decoded_sum = profile
                        .fields_decoded_sum
                        .saturating_add(result.fields_decoded as u64);
                    if result.partial_field.is_some() {
                        profile.partial_fields_reported =
                            profile.partial_fields_reported.saturating_add(1);
                    }
                    if result.complete {
                        profile.complete_prefixes = profile.complete_prefixes.saturating_add(1);
                        if cut == base_tokens.len() {
                            profile.full_sequences_complete =
                                profile.full_sequences_complete.saturating_add(1);
                        }
                    } else {
                        profile.partial_prefixes = profile.partial_prefixes.saturating_add(1);
                    }
                }
            }

            finish_decode_prefix_metrics(
                "decode-prefix-batch",
                elapsed_ns_u64(profile_start),
                token_input_ns,
                profile,
                false,
            );
        }
"#
    };

    arm.replace("__ROOT_TYPE__", root_type)
}
