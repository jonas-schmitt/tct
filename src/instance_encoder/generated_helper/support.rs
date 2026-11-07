//! Shared support code rendered into instance-encoder helpers.

pub(super) fn render_helper_support_code(allocation_metrics_enabled: bool) -> &'static str {
    if allocation_metrics_enabled {
        r#"
use std::alloc::{GlobalAlloc, Layout, System};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};

struct TctCountingAllocator;

static TCT_ALLOCATION_TRACKING_ENABLED: AtomicBool = AtomicBool::new(false);
static TCT_ALLOCATION_COUNT: AtomicU64 = AtomicU64::new(0);
static TCT_ALLOCATED_BYTES: AtomicU64 = AtomicU64::new(0);

#[global_allocator]
static TCT_GLOBAL_ALLOCATOR: TctCountingAllocator = TctCountingAllocator;

unsafe impl GlobalAlloc for TctCountingAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let ptr = unsafe { System.alloc(layout) };
        track_allocation(ptr, layout.size());
        ptr
    }

    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        let ptr = unsafe { System.alloc_zeroed(layout) };
        track_allocation(ptr, layout.size());
        ptr
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        unsafe { System.dealloc(ptr, layout) };
    }

    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        let new_ptr = unsafe { System.realloc(ptr, layout, new_size) };
        track_allocation(new_ptr, new_size);
        new_ptr
    }
}

fn track_allocation(ptr: *mut u8, size: usize) {
    if !ptr.is_null() && TCT_ALLOCATION_TRACKING_ENABLED.load(Ordering::Relaxed) {
        TCT_ALLOCATION_COUNT.fetch_add(1, Ordering::Relaxed);
        TCT_ALLOCATED_BYTES.fetch_add(size as u64, Ordering::Relaxed);
    }
}

fn start_allocation_tracking() -> bool {
    if std::env::var_os("TCT_MEASURE_ALLOCATIONS").is_some() {
        TCT_ALLOCATION_COUNT.store(0, Ordering::Relaxed);
        TCT_ALLOCATED_BYTES.store(0, Ordering::Relaxed);
        TCT_ALLOCATION_TRACKING_ENABLED.store(true, Ordering::SeqCst);
        true
    } else {
        false
    }
}

fn finish_allocation_tracking(operation: &str, started: bool) {
    if started {
        TCT_ALLOCATION_TRACKING_ENABLED.store(false, Ordering::SeqCst);
        let allocations = TCT_ALLOCATION_COUNT.load(Ordering::Relaxed);
        let allocated_bytes = TCT_ALLOCATED_BYTES.load(Ordering::Relaxed);
        eprintln!(
            "TCT_ALLOCATION_METRICS\t{}\t{}\t{}",
            operation, allocations, allocated_bytes
        );
    }
}

fn finish_batch_timing(operation: &str, wall_time_ms: u128) {
    eprintln!("TCT_BATCH_TIMING\t{}\t{}", operation, wall_time_ms);
}

#[derive(Clone, Copy, Default)]
struct TctEncodeRecordProfile {
    failed: bool,
    file_read_ns: u64,
    json_value_parse_ns: u64,
    serde_from_value_ns: u64,
    json_read_parse_ns: u64,
    base_encode_ns: u64,
    bpe_encode_ns: u64,
    output_serialization_ns: u64,
}

#[derive(Clone, Copy, Default)]
struct TctEncodeBatchProfile {
    records: u64,
    failures: u64,
    file_read_ns: u64,
    json_value_parse_ns: u64,
    serde_from_value_ns: u64,
    json_read_parse_ns: u64,
    base_encode_ns: u64,
    bpe_encode_ns: u64,
    output_serialization_ns: u64,
}

impl TctEncodeBatchProfile {
    fn add_record(&mut self, record: TctEncodeRecordProfile) {
        self.records = self.records.saturating_add(1);
        if record.failed {
            self.failures = self.failures.saturating_add(1);
        }
        self.file_read_ns = self.file_read_ns.saturating_add(record.file_read_ns);
        self.json_value_parse_ns = self
            .json_value_parse_ns
            .saturating_add(record.json_value_parse_ns);
        self.serde_from_value_ns = self
            .serde_from_value_ns
            .saturating_add(record.serde_from_value_ns);
        self.json_read_parse_ns = self
            .json_read_parse_ns
            .saturating_add(record.json_read_parse_ns);
        self.base_encode_ns = self.base_encode_ns.saturating_add(record.base_encode_ns);
        self.bpe_encode_ns = self.bpe_encode_ns.saturating_add(record.bpe_encode_ns);
        self.output_serialization_ns = self
            .output_serialization_ns
            .saturating_add(record.output_serialization_ns);
    }
}

#[derive(Clone, Copy, Default)]
struct TctDecodePrefixBatchProfile {
    records: u64,
    prefix_cuts: u64,
    complete_prefixes: u64,
    partial_prefixes: u64,
    failed_prefixes: u64,
    full_sequences_complete: u64,
    consumed_tokens: u64,
    total_prefix_tokens: u64,
    fields_decoded_sum: u64,
    partial_fields_reported: u64,
    bpe_decode_ns: u64,
    decode_prefix_ns: u64,
}

impl TctDecodePrefixBatchProfile {
    fn record_failure(&mut self) {
        self.records = self.records.saturating_add(1);
        self.failed_prefixes = self.failed_prefixes.saturating_add(1);
    }
}

fn elapsed_ns_u64(start: std::time::Instant) -> u64 {
    start.elapsed().as_nanos().try_into().unwrap_or(u64::MAX)
}

fn finish_encode_phase_metrics(
    operation: &str,
    profile_wall_time_ns: u64,
    path_input_ns: u64,
    profile: TctEncodeBatchProfile,
    stdout_write_ns: u64,
    has_bpe: bool,
) {
    let bpe_encode_ns = if has_bpe {
        serde_json::Value::from(profile.bpe_encode_ns)
    } else {
        serde_json::Value::Null
    };
    let metrics = serde_json::json!({
        "records": profile.records,
        "failures": profile.failures,
        "profile_wall_time_ns": profile_wall_time_ns,
        "path_input_ns": path_input_ns,
        "json_read_parse_ns": profile.json_read_parse_ns,
        "base_encode_ns": profile.base_encode_ns,
        "bpe_encode_ns": bpe_encode_ns,
        "output_serialization_ns": profile.output_serialization_ns,
        "stdout_write_ns": stdout_write_ns,
    });
    eprintln!("TCT_ENCODE_PHASE_METRICS\t{}\t{}", operation, metrics);
}

const TCT_TOKEN_CHECKSUM_OFFSET: u64 = 0xcbf2_9ce4_8422_2325;
const TCT_TOKEN_CHECKSUM_PRIME: u64 = 0x0000_0100_0000_01b3;

fn update_tct_token_checksum(mut checksum: u64, tokens: &[u32]) -> u64 {
    for token in tokens {
        checksum ^= u64::from(*token);
        checksum = checksum.wrapping_mul(TCT_TOKEN_CHECKSUM_PRIME);
    }
    checksum ^= u64::MAX;
    checksum.wrapping_mul(TCT_TOKEN_CHECKSUM_PRIME)
}

fn finish_encode_hot_path_metrics(
    operation: &str,
    profile_wall_time_ns: u64,
    path_input_ns: u64,
    profile: TctEncodeBatchProfile,
    token_count: u64,
    token_checksum: u64,
    has_bpe: bool,
) {
    let bpe_encode_ns = if has_bpe {
        serde_json::Value::from(profile.bpe_encode_ns)
    } else {
        serde_json::Value::Null
    };
    let metrics = serde_json::json!({
        "records": profile.records,
        "failures": profile.failures,
        "token_count": token_count,
        "token_checksum": token_checksum,
        "profile_wall_time_ns": profile_wall_time_ns,
        "path_input_ns": path_input_ns,
        "json_read_parse_ns": profile.json_read_parse_ns,
        "base_encode_ns": profile.base_encode_ns,
        "bpe_encode_ns": bpe_encode_ns,
    });
    eprintln!("TCT_ENCODE_HOT_PATH_METRICS\t{}\t{}", operation, metrics);
}

fn finish_encode_validation_phase_metrics(
    operation: &str,
    profile_wall_time_ns: u64,
    path_input_ns: u64,
    profile: TctEncodeBatchProfile,
    token_count: u64,
    token_checksum: u64,
    has_bpe: bool,
) {
    let bpe_encode_ns = if has_bpe {
        serde_json::Value::from(profile.bpe_encode_ns)
    } else {
        serde_json::Value::Null
    };
    let metrics = serde_json::json!({
        "records": profile.records,
        "failures": profile.failures,
        "token_count": token_count,
        "token_checksum": token_checksum,
        "profile_wall_time_ns": profile_wall_time_ns,
        "path_input_ns": path_input_ns,
        "file_read_ns": profile.file_read_ns,
        "json_value_parse_ns": profile.json_value_parse_ns,
        "serde_from_value_ns": profile.serde_from_value_ns,
        "base_encode_ns": profile.base_encode_ns,
        "bpe_encode_ns": bpe_encode_ns,
    });
    eprintln!(
        "TCT_ENCODE_VALIDATION_PHASE_METRICS\t{}\t{}",
        operation, metrics
    );
}

fn prefix_cut_lengths(len: usize) -> Vec<usize> {
    let mut cuts = vec![0, len];
    if len > 1 {
        cuts.push(len / 2);
    }
    cuts.sort_unstable();
    cuts.dedup();
    cuts
}

fn finish_decode_prefix_metrics(
    operation: &str,
    profile_wall_time_ns: u64,
    token_input_ns: u64,
    profile: TctDecodePrefixBatchProfile,
    has_bpe: bool,
) {
    let bpe_decode_ns = if has_bpe {
        serde_json::Value::from(profile.bpe_decode_ns)
    } else {
        serde_json::Value::Null
    };
    let metrics = serde_json::json!({
        "records": profile.records,
        "prefix_cuts": profile.prefix_cuts,
        "complete_prefixes": profile.complete_prefixes,
        "partial_prefixes": profile.partial_prefixes,
        "failed_prefixes": profile.failed_prefixes,
        "full_sequences_complete": profile.full_sequences_complete,
        "consumed_tokens": profile.consumed_tokens,
        "total_prefix_tokens": profile.total_prefix_tokens,
        "fields_decoded_sum": profile.fields_decoded_sum,
        "partial_fields_reported": profile.partial_fields_reported,
        "profile_wall_time_ns": profile_wall_time_ns,
        "token_input_ns": token_input_ns,
        "bpe_decode_ns": bpe_decode_ns,
        "decode_prefix_ns": profile.decode_prefix_ns,
    });
    eprintln!("TCT_DECODE_PREFIX_METRICS\t{}\t{}", operation, metrics);
}
"#
    } else {
        r#"
fn start_allocation_tracking() -> bool {
    false
}

fn finish_allocation_tracking(_operation: &str, _started: bool) {}

fn finish_batch_timing(operation: &str, wall_time_ms: u128) {
    eprintln!("TCT_BATCH_TIMING\t{}\t{}", operation, wall_time_ms);
}

#[derive(Clone, Copy, Default)]
struct TctEncodeRecordProfile {
    failed: bool,
    file_read_ns: u64,
    json_value_parse_ns: u64,
    serde_from_value_ns: u64,
    json_read_parse_ns: u64,
    base_encode_ns: u64,
    bpe_encode_ns: u64,
    output_serialization_ns: u64,
}

#[derive(Clone, Copy, Default)]
struct TctEncodeBatchProfile {
    records: u64,
    failures: u64,
    file_read_ns: u64,
    json_value_parse_ns: u64,
    serde_from_value_ns: u64,
    json_read_parse_ns: u64,
    base_encode_ns: u64,
    bpe_encode_ns: u64,
    output_serialization_ns: u64,
}

impl TctEncodeBatchProfile {
    fn add_record(&mut self, record: TctEncodeRecordProfile) {
        self.records = self.records.saturating_add(1);
        if record.failed {
            self.failures = self.failures.saturating_add(1);
        }
        self.file_read_ns = self.file_read_ns.saturating_add(record.file_read_ns);
        self.json_value_parse_ns = self
            .json_value_parse_ns
            .saturating_add(record.json_value_parse_ns);
        self.serde_from_value_ns = self
            .serde_from_value_ns
            .saturating_add(record.serde_from_value_ns);
        self.json_read_parse_ns = self
            .json_read_parse_ns
            .saturating_add(record.json_read_parse_ns);
        self.base_encode_ns = self.base_encode_ns.saturating_add(record.base_encode_ns);
        self.bpe_encode_ns = self.bpe_encode_ns.saturating_add(record.bpe_encode_ns);
        self.output_serialization_ns = self
            .output_serialization_ns
            .saturating_add(record.output_serialization_ns);
    }
}

#[derive(Clone, Copy, Default)]
struct TctDecodePrefixBatchProfile {
    records: u64,
    prefix_cuts: u64,
    complete_prefixes: u64,
    partial_prefixes: u64,
    failed_prefixes: u64,
    full_sequences_complete: u64,
    consumed_tokens: u64,
    total_prefix_tokens: u64,
    fields_decoded_sum: u64,
    partial_fields_reported: u64,
    bpe_decode_ns: u64,
    decode_prefix_ns: u64,
}

impl TctDecodePrefixBatchProfile {
    fn record_failure(&mut self) {
        self.records = self.records.saturating_add(1);
        self.failed_prefixes = self.failed_prefixes.saturating_add(1);
    }
}

fn elapsed_ns_u64(start: std::time::Instant) -> u64 {
    start.elapsed().as_nanos().try_into().unwrap_or(u64::MAX)
}

fn finish_encode_phase_metrics(
    operation: &str,
    profile_wall_time_ns: u64,
    path_input_ns: u64,
    profile: TctEncodeBatchProfile,
    stdout_write_ns: u64,
    has_bpe: bool,
) {
    let bpe_encode_ns = if has_bpe {
        serde_json::Value::from(profile.bpe_encode_ns)
    } else {
        serde_json::Value::Null
    };
    let metrics = serde_json::json!({
        "records": profile.records,
        "failures": profile.failures,
        "profile_wall_time_ns": profile_wall_time_ns,
        "path_input_ns": path_input_ns,
        "json_read_parse_ns": profile.json_read_parse_ns,
        "base_encode_ns": profile.base_encode_ns,
        "bpe_encode_ns": bpe_encode_ns,
        "output_serialization_ns": profile.output_serialization_ns,
        "stdout_write_ns": stdout_write_ns,
    });
    eprintln!("TCT_ENCODE_PHASE_METRICS\t{}\t{}", operation, metrics);
}

const TCT_TOKEN_CHECKSUM_OFFSET: u64 = 0xcbf2_9ce4_8422_2325;
const TCT_TOKEN_CHECKSUM_PRIME: u64 = 0x0000_0100_0000_01b3;

fn update_tct_token_checksum(mut checksum: u64, tokens: &[u32]) -> u64 {
    for token in tokens {
        checksum ^= u64::from(*token);
        checksum = checksum.wrapping_mul(TCT_TOKEN_CHECKSUM_PRIME);
    }
    checksum ^= u64::MAX;
    checksum.wrapping_mul(TCT_TOKEN_CHECKSUM_PRIME)
}

fn finish_encode_hot_path_metrics(
    operation: &str,
    profile_wall_time_ns: u64,
    path_input_ns: u64,
    profile: TctEncodeBatchProfile,
    token_count: u64,
    token_checksum: u64,
    has_bpe: bool,
) {
    let bpe_encode_ns = if has_bpe {
        serde_json::Value::from(profile.bpe_encode_ns)
    } else {
        serde_json::Value::Null
    };
    let metrics = serde_json::json!({
        "records": profile.records,
        "failures": profile.failures,
        "token_count": token_count,
        "token_checksum": token_checksum,
        "profile_wall_time_ns": profile_wall_time_ns,
        "path_input_ns": path_input_ns,
        "json_read_parse_ns": profile.json_read_parse_ns,
        "base_encode_ns": profile.base_encode_ns,
        "bpe_encode_ns": bpe_encode_ns,
    });
    eprintln!("TCT_ENCODE_HOT_PATH_METRICS\t{}\t{}", operation, metrics);
}

fn finish_encode_validation_phase_metrics(
    operation: &str,
    profile_wall_time_ns: u64,
    path_input_ns: u64,
    profile: TctEncodeBatchProfile,
    token_count: u64,
    token_checksum: u64,
    has_bpe: bool,
) {
    let bpe_encode_ns = if has_bpe {
        serde_json::Value::from(profile.bpe_encode_ns)
    } else {
        serde_json::Value::Null
    };
    let metrics = serde_json::json!({
        "records": profile.records,
        "failures": profile.failures,
        "token_count": token_count,
        "token_checksum": token_checksum,
        "profile_wall_time_ns": profile_wall_time_ns,
        "path_input_ns": path_input_ns,
        "file_read_ns": profile.file_read_ns,
        "json_value_parse_ns": profile.json_value_parse_ns,
        "serde_from_value_ns": profile.serde_from_value_ns,
        "base_encode_ns": profile.base_encode_ns,
        "bpe_encode_ns": bpe_encode_ns,
    });
    eprintln!(
        "TCT_ENCODE_VALIDATION_PHASE_METRICS\t{}\t{}",
        operation, metrics
    );
}

fn prefix_cut_lengths(len: usize) -> Vec<usize> {
    let mut cuts = vec![0, len];
    if len > 1 {
        cuts.push(len / 2);
    }
    cuts.sort_unstable();
    cuts.dedup();
    cuts
}

fn finish_decode_prefix_metrics(
    operation: &str,
    profile_wall_time_ns: u64,
    token_input_ns: u64,
    profile: TctDecodePrefixBatchProfile,
    has_bpe: bool,
) {
    let bpe_decode_ns = if has_bpe {
        serde_json::Value::from(profile.bpe_decode_ns)
    } else {
        serde_json::Value::Null
    };
    let metrics = serde_json::json!({
        "records": profile.records,
        "prefix_cuts": profile.prefix_cuts,
        "complete_prefixes": profile.complete_prefixes,
        "partial_prefixes": profile.partial_prefixes,
        "failed_prefixes": profile.failed_prefixes,
        "full_sequences_complete": profile.full_sequences_complete,
        "consumed_tokens": profile.consumed_tokens,
        "total_prefix_tokens": profile.total_prefix_tokens,
        "fields_decoded_sum": profile.fields_decoded_sum,
        "partial_fields_reported": profile.partial_fields_reported,
        "profile_wall_time_ns": profile_wall_time_ns,
        "token_input_ns": token_input_ns,
        "bpe_decode_ns": bpe_decode_ns,
        "decode_prefix_ns": profile.decode_prefix_ns,
    });
    eprintln!("TCT_DECODE_PREFIX_METRICS\t{}\t{}", operation, metrics);
}
"#
    }
}
