//! Metrics emitted by generated instance-encoder helpers.

use serde::Deserialize;

const ALLOCATION_METRICS_PREFIX: &str = "TCT_ALLOCATION_METRICS\t";
const BATCH_TIMING_PREFIX: &str = "TCT_BATCH_TIMING\t";
const ENCODE_PHASE_METRICS_PREFIX: &str = "TCT_ENCODE_PHASE_METRICS\t";
const ENCODE_HOT_PATH_METRICS_PREFIX: &str = "TCT_ENCODE_HOT_PATH_METRICS\t";
const ENCODE_VALIDATION_PHASE_METRICS_PREFIX: &str = "TCT_ENCODE_VALIDATION_PHASE_METRICS\t";
const DECODE_PREFIX_METRICS_PREFIX: &str = "TCT_DECODE_PREFIX_METRICS\t";
const TOKEN_CHECKSUM_OFFSET: u64 = 0xcbf2_9ce4_8422_2325;
const TOKEN_CHECKSUM_PRIME: u64 = 0x0000_0100_0000_01b3;

/// Allocation counters emitted by the generated helper binary for one batch mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BatchAllocationMetrics {
    pub allocations: u64,
    pub allocated_bytes: u64,
}

/// Runtime metrics emitted by the generated helper binary for one batch mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BatchRuntimeMetrics {
    pub wall_time_ms: u64,
    pub allocation_metrics: Option<BatchAllocationMetrics>,
}

/// Parent-side transport timings for a generated helper primary batch run.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct BatchParentIoMetrics {
    pub stdin_write_ns: u64,
    pub stdout_read_parse_ns: u64,
    pub stdout_json_parse_ns: u64,
    pub child_wait_ns: u64,
}

/// Runtime metrics for the primary generated encode/decode batch path.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BatchPrimaryRuntimeMetrics {
    pub child_wall_time_ms: u64,
    pub parent_io: BatchParentIoMetrics,
    pub allocation_metrics: Option<BatchAllocationMetrics>,
}

/// Phase metrics emitted by generated helper binaries for one profiled encode batch.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
pub struct BatchEncodePhaseMetrics {
    pub records: u64,
    pub failures: u64,
    pub profile_wall_time_ns: u64,
    pub path_input_ns: u64,
    pub json_read_parse_ns: u64,
    pub base_encode_ns: u64,
    pub bpe_encode_ns: Option<u64>,
    pub output_serialization_ns: u64,
    pub stdout_write_ns: u64,
}

/// Hot-path metrics emitted by generated helper binaries for one profiled encode batch.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
pub struct BatchEncodeHotPathMetrics {
    pub records: u64,
    pub failures: u64,
    pub token_count: u64,
    pub token_checksum: u64,
    pub profile_wall_time_ns: u64,
    pub path_input_ns: u64,
    pub json_read_parse_ns: u64,
    pub base_encode_ns: u64,
    pub bpe_encode_ns: Option<u64>,
}

/// Diagnostic phase metrics emitted by generated helpers for Value -> typed encode attribution.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
pub struct BatchEncodeValidationPhaseMetrics {
    pub records: u64,
    pub failures: u64,
    pub token_count: u64,
    pub token_checksum: u64,
    pub profile_wall_time_ns: u64,
    pub path_input_ns: u64,
    pub file_read_ns: u64,
    pub json_value_parse_ns: u64,
    pub serde_from_value_ns: u64,
    pub base_encode_ns: u64,
    pub bpe_encode_ns: Option<u64>,
}

/// Prefix-decode metrics emitted by generated helpers for measured token batches.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
pub struct BatchDecodePrefixMetrics {
    pub records: u64,
    pub prefix_cuts: u64,
    pub complete_prefixes: u64,
    pub partial_prefixes: u64,
    pub failed_prefixes: u64,
    pub full_sequences_complete: u64,
    pub consumed_tokens: u64,
    pub total_prefix_tokens: u64,
    pub fields_decoded_sum: u64,
    pub partial_fields_reported: u64,
    pub profile_wall_time_ns: u64,
    pub token_input_ns: u64,
    pub bpe_decode_ns: Option<u64>,
    pub decode_prefix_ns: u64,
}

/// Compute the deterministic checksum used by generated encode hot-path summaries.
#[must_use]
pub fn token_sequence_checksum(token_sequences: &[Vec<u32>]) -> u64 {
    token_sequences
        .iter()
        .fold(TOKEN_CHECKSUM_OFFSET, |checksum, tokens| {
            update_token_checksum(checksum, tokens)
        })
}

fn update_token_checksum(mut checksum: u64, tokens: &[u32]) -> u64 {
    for token in tokens {
        checksum ^= u64::from(*token);
        checksum = checksum.wrapping_mul(TOKEN_CHECKSUM_PRIME);
    }
    checksum ^= u64::MAX;
    checksum.wrapping_mul(TOKEN_CHECKSUM_PRIME)
}

pub(super) fn parse_batch_allocation_metrics(
    stderr: &[u8],
    operation: &str,
) -> Option<BatchAllocationMetrics> {
    let stderr = String::from_utf8_lossy(stderr);
    for line in stderr.lines() {
        let Some(rest) = line.strip_prefix(ALLOCATION_METRICS_PREFIX) else {
            continue;
        };
        let mut parts = rest.split('\t');
        let Some(actual_operation) = parts.next() else {
            continue;
        };
        if actual_operation != operation {
            continue;
        }
        let allocations = parts.next()?.parse().ok()?;
        let allocated_bytes = parts.next()?.parse().ok()?;
        return Some(BatchAllocationMetrics {
            allocations,
            allocated_bytes,
        });
    }
    None
}

fn parse_batch_timing_ms(stderr: &[u8], operation: &str) -> Option<u64> {
    let stderr = String::from_utf8_lossy(stderr);
    for line in stderr.lines() {
        let Some(rest) = line.strip_prefix(BATCH_TIMING_PREFIX) else {
            continue;
        };
        let mut parts = rest.split('\t');
        let Some(actual_operation) = parts.next() else {
            continue;
        };
        if actual_operation != operation {
            continue;
        }
        return parts.next()?.parse().ok();
    }
    None
}

pub(super) fn parse_batch_runtime_metrics(
    stderr: &[u8],
    operation: &str,
    expect_allocations: bool,
) -> Option<BatchRuntimeMetrics> {
    let wall_time_ms = parse_batch_timing_ms(stderr, operation)?;
    let allocation_metrics = if expect_allocations {
        parse_batch_allocation_metrics(stderr, operation)
    } else {
        None
    };
    Some(BatchRuntimeMetrics {
        wall_time_ms,
        allocation_metrics,
    })
}

pub(super) fn parse_primary_batch_runtime_metrics(
    stderr: &[u8],
    operation: &str,
    expect_allocations: bool,
    parent_io: BatchParentIoMetrics,
) -> Option<BatchPrimaryRuntimeMetrics> {
    let child_runtime = parse_batch_runtime_metrics(stderr, operation, expect_allocations)?;
    Some(BatchPrimaryRuntimeMetrics {
        child_wall_time_ms: child_runtime.wall_time_ms,
        parent_io,
        allocation_metrics: child_runtime.allocation_metrics,
    })
}

pub(super) fn parse_batch_encode_phase_metrics(
    stderr: &[u8],
    operation: &str,
) -> Option<BatchEncodePhaseMetrics> {
    parse_json_sideband(stderr, operation, ENCODE_PHASE_METRICS_PREFIX)
}

pub(super) fn parse_batch_encode_hot_path_metrics(
    stderr: &[u8],
    operation: &str,
) -> Option<BatchEncodeHotPathMetrics> {
    parse_json_sideband(stderr, operation, ENCODE_HOT_PATH_METRICS_PREFIX)
}

pub(super) fn parse_batch_encode_validation_phase_metrics(
    stderr: &[u8],
    operation: &str,
) -> Option<BatchEncodeValidationPhaseMetrics> {
    parse_json_sideband(stderr, operation, ENCODE_VALIDATION_PHASE_METRICS_PREFIX)
}

pub(super) fn parse_batch_decode_prefix_metrics(
    stderr: &[u8],
    operation: &str,
) -> Option<BatchDecodePrefixMetrics> {
    parse_json_sideband(stderr, operation, DECODE_PREFIX_METRICS_PREFIX)
}

fn parse_json_sideband<T: for<'de> Deserialize<'de>>(
    stderr: &[u8],
    operation: &str,
    prefix: &str,
) -> Option<T> {
    let stderr = String::from_utf8_lossy(stderr);
    for line in stderr.lines() {
        let Some(rest) = line.strip_prefix(prefix) else {
            continue;
        };
        let mut parts = rest.split('\t');
        let Some(actual_operation) = parts.next() else {
            continue;
        };
        if actual_operation != operation {
            continue;
        }
        let payload = parts.next()?;
        return serde_json::from_str(payload).ok();
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_batch_runtime_metrics_combines_timing_and_allocations() {
        let stderr = b"noise\nTCT_BATCH_TIMING\tbpe-encode-batch\t17\nTCT_ALLOCATION_METRICS\tbpe-encode-batch\t3\t99\n";

        assert_eq!(
            parse_batch_runtime_metrics(stderr, "bpe-encode-batch", true),
            Some(BatchRuntimeMetrics {
                wall_time_ms: 17,
                allocation_metrics: Some(BatchAllocationMetrics {
                    allocations: 3,
                    allocated_bytes: 99,
                }),
            })
        );
    }

    #[test]
    fn test_parse_primary_batch_runtime_metrics_adds_parent_io() {
        let stderr = b"TCT_BATCH_TIMING\tencode-batch\t23\n";
        let parent_io = BatchParentIoMetrics {
            stdin_write_ns: 1,
            stdout_read_parse_ns: 2,
            stdout_json_parse_ns: 3,
            child_wait_ns: 4,
        };

        assert_eq!(
            parse_primary_batch_runtime_metrics(stderr, "encode-batch", false, parent_io),
            Some(BatchPrimaryRuntimeMetrics {
                child_wall_time_ms: 23,
                parent_io,
                allocation_metrics: None,
            })
        );
    }

    #[test]
    fn test_parse_batch_encode_phase_metrics_reads_json_sideband() {
        let stderr = br#"noise
TCT_ALLOCATION_METRICS	encode-batch	3	99
TCT_ENCODE_PHASE_METRICS	encode-batch	{"records":2,"failures":1,"profile_wall_time_ns":100,"path_input_ns":5,"json_read_parse_ns":40,"base_encode_ns":30,"bpe_encode_ns":7,"output_serialization_ns":10,"stdout_write_ns":8}
"#;

        assert_eq!(
            parse_batch_encode_phase_metrics(stderr, "encode-batch"),
            Some(BatchEncodePhaseMetrics {
                records: 2,
                failures: 1,
                profile_wall_time_ns: 100,
                path_input_ns: 5,
                json_read_parse_ns: 40,
                base_encode_ns: 30,
                bpe_encode_ns: Some(7),
                output_serialization_ns: 10,
                stdout_write_ns: 8,
            })
        );
    }

    #[test]
    fn test_parse_batch_encode_hot_path_metrics_reads_json_sideband() {
        let stderr = br#"noise
TCT_ENCODE_HOT_PATH_METRICS	encode-batch	{"records":2,"failures":0,"token_count":3,"token_checksum":123,"profile_wall_time_ns":100,"path_input_ns":5,"json_read_parse_ns":40,"base_encode_ns":30,"bpe_encode_ns":7}
"#;

        assert_eq!(
            parse_batch_encode_hot_path_metrics(stderr, "encode-batch"),
            Some(BatchEncodeHotPathMetrics {
                records: 2,
                failures: 0,
                token_count: 3,
                token_checksum: 123,
                profile_wall_time_ns: 100,
                path_input_ns: 5,
                json_read_parse_ns: 40,
                base_encode_ns: 30,
                bpe_encode_ns: Some(7),
            })
        );
    }

    #[test]
    fn test_parse_batch_encode_validation_phase_metrics_reads_json_sideband() {
        let stderr = br#"noise
TCT_ENCODE_VALIDATION_PHASE_METRICS	encode-batch	{"records":2,"failures":0,"token_count":3,"token_checksum":123,"profile_wall_time_ns":100,"path_input_ns":5,"file_read_ns":11,"json_value_parse_ns":22,"serde_from_value_ns":33,"base_encode_ns":44,"bpe_encode_ns":7}
"#;

        assert_eq!(
            parse_batch_encode_validation_phase_metrics(stderr, "encode-batch"),
            Some(BatchEncodeValidationPhaseMetrics {
                records: 2,
                failures: 0,
                token_count: 3,
                token_checksum: 123,
                profile_wall_time_ns: 100,
                path_input_ns: 5,
                file_read_ns: 11,
                json_value_parse_ns: 22,
                serde_from_value_ns: 33,
                base_encode_ns: 44,
                bpe_encode_ns: Some(7),
            })
        );
    }

    #[test]
    fn test_parse_batch_decode_prefix_metrics_reads_json_sideband() {
        let stderr = br#"noise
TCT_DECODE_PREFIX_METRICS	decode-prefix-batch	{"records":2,"prefix_cuts":5,"complete_prefixes":2,"partial_prefixes":3,"failed_prefixes":0,"full_sequences_complete":2,"consumed_tokens":9,"total_prefix_tokens":11,"fields_decoded_sum":7,"partial_fields_reported":3,"profile_wall_time_ns":100,"token_input_ns":5,"bpe_decode_ns":null,"decode_prefix_ns":70}
"#;

        assert_eq!(
            parse_batch_decode_prefix_metrics(stderr, "decode-prefix-batch"),
            Some(BatchDecodePrefixMetrics {
                records: 2,
                prefix_cuts: 5,
                complete_prefixes: 2,
                partial_prefixes: 3,
                failed_prefixes: 0,
                full_sequences_complete: 2,
                consumed_tokens: 9,
                total_prefix_tokens: 11,
                fields_decoded_sum: 7,
                partial_fields_reported: 3,
                profile_wall_time_ns: 100,
                token_input_ns: 5,
                bpe_decode_ns: None,
                decode_prefix_ns: 70,
            })
        );
    }

    #[test]
    fn test_token_sequence_checksum_is_order_sensitive() {
        let first = token_sequence_checksum(&[vec![1, 2], vec![3]]);
        let second = token_sequence_checksum(&[vec![3], vec![1, 2]]);

        assert_ne!(first, second);
        assert_eq!(first, token_sequence_checksum(&[vec![1, 2], vec![3]]));
    }

    #[test]
    fn test_parse_batch_encode_phase_metrics_rejects_missing_or_malformed_fields() {
        let missing = br#"TCT_ENCODE_PHASE_METRICS	encode-batch	{"records":2}"#;
        let malformed = br#"TCT_ENCODE_PHASE_METRICS	encode-batch	not-json"#;
        let wrong_operation = br#"TCT_ENCODE_PHASE_METRICS	decode-batch	{"records":2,"failures":0,"profile_wall_time_ns":1,"path_input_ns":1,"json_read_parse_ns":1,"base_encode_ns":1,"bpe_encode_ns":null,"output_serialization_ns":1,"stdout_write_ns":1}"#;

        assert_eq!(
            parse_batch_encode_phase_metrics(missing, "encode-batch"),
            None
        );
        assert_eq!(
            parse_batch_encode_phase_metrics(malformed, "encode-batch"),
            None
        );
        assert_eq!(
            parse_batch_encode_phase_metrics(wrong_operation, "encode-batch"),
            None
        );
    }
}
