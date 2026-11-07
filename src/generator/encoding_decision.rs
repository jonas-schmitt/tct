//! Encoding decisions for the optimization phase.
//!
//! This module defines pure data structures that represent how discovered types
//! should be encoded. The active four-pass pipeline produces these decisions in
//! `generator::optimization_phase::OptimizationPhase` and consumes them in
//! `generator::code_generator::CodeGenerator`, which then passes individual
//! decisions to the specialized generators.
//!
//! Keep the boundary strict:
//! - discovery and analysis describe schema structure and type properties;
//! - optimization produces `EncodingStrategy` metadata and member refinements;
//! - generation turns that metadata into `TokenStream` output.
//!
//! Do not add generated Rust source or generated-source string classifiers to
//! this module. Add structured fields here when generation needs new
//! optimization metadata.
//!
//! Producer and consumer map for active decisions:
//! - `Standard`: produced as the fallback by `OptimizationPhase` and consumed by
//!   all generators as ordinary schema-driven encoding.
//! - `EnumIndex`: produced for enums when `enum_index_optimization` is active;
//!   consumed by `enum_generator` through `CodeGenerator::generate_enum`.
//! - `BoundedIntegerOffset`: produced for fully bounded `i64` wrappers whose
//!   offset range fits the active structural vocabulary budget; consumed by
//!   `bounded_integer_generator`.
//! - `BoundedIntegerCodec`: produced for fully bounded `i64` wrappers whose
//!   offset range exceeds `max_vocab`; consumed by `bounded_integer_generator`
//!   and the generated structural codec helpers.
//! - `SingleVariantEnum`: produced for one-variant enums when enum optimization
//!   is active; consumed by `enum_generator` and by struct streaming generation
//!   as a zero-token child-type signal.
//! - `ConstantInteger`: produced for bounded integer wrappers with
//!   `minimum == maximum`; consumed by `type_alias_generator`.
//! - `FormatOptimized`: produced at O3 for recognized JSON Schema string
//!   formats; consumed by `type_alias_generator`.
//! - `BitPackedStruct`: produced for structs with enough two-state optional
//!   fields when bit packing is enabled; consumed by `struct_generator`.
//!
//! Reserved or compatibility-shaped decisions:
//! - `BoundedInteger` is a supported data shape for statistics, tests, and
//!   compatibility reporting, but the active optimization pass does not emit it.
//!   (`EnumCodec` is emitted for enums whose variant count exceeds `max_vocab`.)
//! - `PatternOptimized` and `FixedSizeArray` describe future optimization
//!   shapes; do not emit them from `OptimizationPhase` without adding generation
//!   support and generated-crate regressions in the same change.
//!
//! `MemberTypeRefinement` is separate from per-type encoding decisions. It is
//! produced by additionalProperties optimization and consumed by
//! `CodeGenerator::apply_member_type_refinements` before generation, without
//! mutating `DiscoveryPass` output.
//!
//! `schema_evolution` and token-diff reporting read these decisions as
//! structured compatibility evidence. That reporting layer must not become a
//! producer of optimization decisions.
//!
//! `EncodingStrategy::update_stats()` is advisory reporting metadata.
//! `CodeGenerator` recomputes generated structural token bounds from the final
//! decisions and discovery graph before emitting constants.
//!
//! Note that `src/vocabulary_analyzer.rs` has its own report-local
//! `EncodingStrategy` enum. It is not this generation-facing strategy.

use rustc_hash::FxHashMap;

fn inclusive_i64_range_size(min: i64, max: i64) -> Option<u128> {
    if min > max {
        return None;
    }

    Some((i128::from(max) - i128::from(min) + 1) as u128)
}

fn bounded_i64_range_usage(min: i64, max: i64) -> usize {
    inclusive_i64_range_size(min, max)
        .and_then(|range| usize::try_from(range).ok())
        .unwrap_or(usize::MAX)
}

fn u128_to_u32_saturating(value: u128) -> u32 {
    u32::try_from(value).unwrap_or(u32::MAX)
}

fn nonnegative_i64_to_u32_saturating(value: i64) -> u32 {
    if value <= 0 {
        0
    } else {
        u32::try_from(value).unwrap_or(u32::MAX)
    }
}

fn u32_plus_one_to_usize_saturating(value: u32) -> usize {
    usize::try_from(u64::from(value) + 1).unwrap_or(usize::MAX)
}

fn add_saturating(total: &mut usize, value: usize) {
    *total = total.saturating_add(value);
}

/// Well-known string formats for specialized encoding
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StringFormat {
    /// Integer formatted as string (JSON Schema int32/int64)
    Int32,
    Int64,

    /// Floating point formatted as string (JSON Schema double/float)
    Double,
    Float,

    /// Base64-encoded binary data
    Byte,

    /// ISO 8601 date-time string (RFC 3339)
    DateTime,

    /// ISO 8601 date string (YYYY-MM-DD)
    Date,

    /// ISO 8601 time string (HH:MM:SS)
    Time,

    /// ISO 8601 duration string
    Duration,

    /// UUID (RFC 4122) - 128-bit identifier
    Uuid,

    /// Email address (RFC 5322)
    Email,

    /// URI/URL (RFC 3986)
    Uri,
    UriReference,
    UriTemplate,

    /// Hostname (RFC 1034)
    Hostname,

    /// IPv4 address
    Ipv4,

    /// IPv6 address
    Ipv6,

    /// JSON Pointer (RFC 6901)
    JsonPointer,

    /// Regex pattern
    Regex,
}

/// Type identifier (e.g., "Status", "Job", "definitions.Step")
pub type TypePath = String;

/// Identifies a discovered member whose generated type is refined by optimization.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct MemberPath {
    /// Discovered type name.
    pub type_name: TypePath,
    /// Member/property name inside the discovered type.
    pub member_name: String,
}

impl MemberPath {
    /// Create a new member path.
    pub fn new(type_name: impl Into<TypePath>, member_name: impl Into<String>) -> Self {
        Self {
            type_name: type_name.into(),
            member_name: member_name.into(),
        }
    }
}

/// Explicit generation-facing type refinement produced by optimization.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MemberTypeRefinement {
    /// Refine a discovered HashMap member to a more specific key/value type.
    HashMap {
        /// Key Rust type.
        key: String,
        /// Value Rust type.
        value: String,
    },
}

/// Collection of encoding decisions for all types in a schema
#[derive(Debug, Clone, Default)]
pub struct EncodingStrategy {
    /// Encoding decision for each type
    pub decisions: FxHashMap<TypePath, EncodingDecision>,

    /// Explicit member type refinements for generation.
    pub member_type_refinements: FxHashMap<MemberPath, MemberTypeRefinement>,

    /// Maximum vocabulary size constraint (if any)
    pub max_vocabulary_size: Option<usize>,

    /// Statistics about optimizations applied
    pub stats: crate::vocabulary_encoder::statistics::VocabularyOptimizationStats,
}

impl EncodingStrategy {
    /// Create new empty encoding strategy
    pub fn new() -> Self {
        Self::default()
    }

    /// Add encoding decision for a type
    pub fn add_decision(&mut self, type_path: impl Into<String>, decision: EncodingDecision) {
        self.decisions.insert(type_path.into(), decision);
    }

    /// Add a generation-facing member type refinement.
    pub fn add_member_type_refinement(
        &mut self,
        type_name: impl Into<TypePath>,
        member_name: impl Into<String>,
        refinement: MemberTypeRefinement,
    ) {
        self.member_type_refinements
            .insert(MemberPath::new(type_name, member_name), refinement);
    }

    /// Get encoding decision for a type (returns Standard if not found)
    pub fn get_decision(&self, type_path: &str) -> EncodingDecision {
        self.decisions
            .get(type_path)
            .cloned()
            .unwrap_or(EncodingDecision::Standard)
    }

    /// Check if any optimizations were applied
    pub fn has_optimizations(&self) -> bool {
        self.decisions
            .values()
            .any(|d| !matches!(d, EncodingDecision::Standard))
    }

    /// Count how many types use each encoding strategy
    pub fn count_by_strategy(&self) -> FxHashMap<&'static str, usize> {
        let mut counts = FxHashMap::default();
        for decision in self.decisions.values() {
            let strategy_name = decision.strategy_name();
            *counts.entry(strategy_name).or_insert(0) += 1;
        }
        counts
    }

    /// Update statistics based on current encoding decisions
    pub fn update_stats(&mut self) {
        let mut optimized_fields = 0;
        let mut total_constrained_values = 0;
        let mut naive_vocabulary_size = 0;
        let mut distinct_structural_tokens_count = 0;
        let mut max_token_id = 0u32; // Track maximum token ID that can appear
        let mut enum_domains = 0;
        let mut bounded_integer_domains = 0;
        let mut pattern_domains = 0;
        let mut optimization_breakdown = FxHashMap::default();

        for decision in self.decisions.values() {
            match decision {
                EncodingDecision::Standard => {
                    // Standard encoding doesn't use vocabulary tokens
                    continue;
                }
                EncodingDecision::BitPackedStruct { .. } => {
                    // Bit-packed struct encoding doesn't use vocabulary tokens (structural optimization)
                    continue;
                }
                EncodingDecision::EnumIndex { variants } => {
                    optimized_fields += 1;
                    enum_domains += 1;
                    add_saturating(&mut total_constrained_values, variants.len());
                    add_saturating(&mut naive_vocabulary_size, variants.len());
                    add_saturating(&mut distinct_structural_tokens_count, variants.len());
                    // Max token ID is (variants.len() - 1) for 0-indexed enum
                    // Use saturating_sub to handle empty enums gracefully
                    max_token_id = max_token_id.max(u128_to_u32_saturating(
                        variants.len().saturating_sub(1) as u128,
                    ));
                    *optimization_breakdown
                        .entry("enum_index".to_string())
                        .or_insert(0) += 1;
                }
                EncodingDecision::EnumCodec {
                    variants,
                    codec_spec,
                } => {
                    optimized_fields += 1;
                    enum_domains += 1;
                    add_saturating(&mut total_constrained_values, variants.len());
                    add_saturating(&mut naive_vocabulary_size, variants.len());
                    add_saturating(
                        &mut distinct_structural_tokens_count,
                        codec_spec.base as usize,
                    );
                    // Max token ID is (base - 1) for codec digits
                    max_token_id = max_token_id.max(codec_spec.base - 1);
                    *optimization_breakdown
                        .entry("enum_codec".to_string())
                        .or_insert(0) += 1;
                }
                EncodingDecision::BoundedInteger { min, max } => {
                    optimized_fields += 1;
                    bounded_integer_domains += 1;
                    let range = bounded_i64_range_usage(*min, *max);
                    add_saturating(&mut total_constrained_values, range);
                    add_saturating(&mut naive_vocabulary_size, range);
                    add_saturating(&mut distinct_structural_tokens_count, range);
                    // Max token ID is the maximum value itself
                    max_token_id = max_token_id.max(nonnegative_i64_to_u32_saturating(*max));
                    *optimization_breakdown
                        .entry("bounded_integer".to_string())
                        .or_insert(0) += 1;
                }
                EncodingDecision::BoundedIntegerOffset { range, .. } => {
                    optimized_fields += 1;
                    bounded_integer_domains += 1;
                    let range_usize = *range as usize;
                    add_saturating(&mut total_constrained_values, range_usize);
                    add_saturating(&mut naive_vocabulary_size, range_usize);
                    add_saturating(&mut distinct_structural_tokens_count, range_usize);
                    // Max token ID is (range - 1) for 0-indexed offset encoding
                    // Use saturating_sub to handle edge case of range=0
                    max_token_id = max_token_id.max(range.saturating_sub(1));
                    *optimization_breakdown
                        .entry("bounded_integer_offset".to_string())
                        .or_insert(0) += 1;
                }
                EncodingDecision::BoundedIntegerCodec {
                    min,
                    max,
                    codec_spec,
                } => {
                    optimized_fields += 1;
                    bounded_integer_domains += 1;
                    let range = bounded_i64_range_usage(*min, *max);
                    add_saturating(&mut total_constrained_values, range);
                    add_saturating(&mut naive_vocabulary_size, range);
                    add_saturating(
                        &mut distinct_structural_tokens_count,
                        codec_spec.base as usize,
                    );
                    // CRITICAL FIX: Max token ID is the maximum VALUE, not the codec base
                    // Even with multi-token encoding, the value itself may be emitted directly
                    max_token_id = max_token_id.max(nonnegative_i64_to_u32_saturating(*max));
                    *optimization_breakdown
                        .entry("bounded_integer_codec".to_string())
                        .or_insert(0) += 1;
                }
                EncodingDecision::PatternOptimized { strategy, .. } => {
                    optimized_fields += 1;
                    pattern_domains += 1;
                    let usage = strategy.estimated_vocabulary_usage();
                    add_saturating(&mut total_constrained_values, usage);
                    add_saturating(&mut naive_vocabulary_size, usage);
                    add_saturating(&mut distinct_structural_tokens_count, usage);
                    // Max token ID is (usage - 1) for sequential 0-indexed pattern tokens
                    if usage > 0 {
                        max_token_id =
                            max_token_id.max(u128_to_u32_saturating((usage - 1) as u128));
                    }
                    *optimization_breakdown
                        .entry("pattern_optimized".to_string())
                        .or_insert(0) += 1;
                }
                EncodingDecision::SingleVariantEnum { .. } => {
                    optimized_fields += 1;
                    enum_domains += 1;
                    // Naive: would need 1 token for enum value
                    // Optimized: constant, no tokens needed
                    add_saturating(&mut naive_vocabulary_size, 1);
                    add_saturating(&mut total_constrained_values, 1);
                    *optimization_breakdown
                        .entry("single_variant_enum".to_string())
                        .or_insert(0) += 1;
                }
                EncodingDecision::ConstantInteger { .. } => {
                    optimized_fields += 1;
                    bounded_integer_domains += 1;
                    // Naive: would need 1+ tokens for integer
                    // Optimized: constant, no tokens needed
                    add_saturating(&mut naive_vocabulary_size, 1);
                    add_saturating(&mut total_constrained_values, 1);
                    *optimization_breakdown
                        .entry("constant_integer".to_string())
                        .or_insert(0) += 1;
                }
                EncodingDecision::FixedSizeArray { .. } => {
                    optimized_fields += 1;
                    // Naive: would need 1 token for length
                    // Optimized: length is constant, no token needed
                    add_saturating(&mut naive_vocabulary_size, 1);
                    *optimization_breakdown
                        .entry("fixed_size_array".to_string())
                        .or_insert(0) += 1;
                }
                EncodingDecision::FormatOptimized { .. } => {
                    optimized_fields += 1;
                    // Naive: string encoding (many tokens)
                    // Optimized: structured encoding (e.g., int32 → 1 token, uuid → 2 tokens)
                    // Estimated average: 10 tokens naive → 2 tokens optimized
                    add_saturating(&mut naive_vocabulary_size, 10);
                    add_saturating(&mut distinct_structural_tokens_count, 2);
                    *optimization_breakdown
                        .entry("format_optimized".to_string())
                        .or_insert(0) += 1;
                }
            }
        }

        let tokens_saved = naive_vocabulary_size.saturating_sub(distinct_structural_tokens_count);
        let reduction_percentage = if naive_vocabulary_size > 0 {
            (tokens_saved as f64 / naive_vocabulary_size as f64) * 100.0
        } else {
            0.0
        };

        let pattern_tokens_saved = 0; // Will be populated when pattern optimization is fully implemented
        let pattern_tokens_allocated = if pattern_domains > 0 {
            distinct_structural_tokens_count
        } else {
            0
        };

        self.stats = crate::vocabulary_encoder::statistics::VocabularyOptimizationStats {
            optimized_fields,
            total_constrained_values,
            naive_vocabulary_size,
            distinct_structural_tokens_count,
            tokens_saved,
            reduction_percentage,
            pattern_domains,
            pattern_vocabulary_reduction: 0.0,
            pattern_tokens_saved,
            pattern_tokens_allocated,
            max_structural_token_id_plus_one: u32_plus_one_to_usize_saturating(max_token_id),
            estimated_unoptimized_tokens: naive_vocabulary_size,
            total_vocabulary_reduction: reduction_percentage,
            enum_domains,
            bounded_integer_domains,
            optimization_breakdown,
        };
    }
}

/// Explicit encoding decision (pure data)
///
/// This represents a decision about HOW to encode a type, separate from the actual
/// code generation logic.
#[derive(Debug, Clone, PartialEq)]
pub enum EncodingDecision {
    /// Standard encoding (no optimization)
    ///
    /// For enums: one-token variant indices, exactly like `EnumIndex`
    /// For integers and numbers: standard Rust/generated runtime types
    /// For strings: Byte-level encoding
    Standard,

    /// Enum: index-based encoding (0, 1, 2, ...)
    ///
    /// Maps enum variants to sequential indices.
    /// Example: ["active", "inactive", "pending"] → 0, 1, 2
    EnumIndex { variants: Vec<String> },

    /// Enum: codec-based multi-token encoding for structural budget
    ///
    /// When enum exceeds structural vocabulary budget, use multi-token encoding.
    /// Example: 200 variants with max_structural=100 → 2 tokens in base-100
    /// NOTE: This is LOSSLESS - uses multiple tokens to preserve all information
    EnumCodec {
        variants: Vec<String>,
        codec_spec: CodecSpec,
    },

    /// Integer: bounded range encoding
    ///
    /// For integers with known min/max bounds.
    /// Example: priority: 1-10 → BoundedInteger { min: 1, max: 10 }
    BoundedInteger { min: i64, max: i64 },

    /// Integer: bounded range with standard offset encoding
    ///
    /// For bounded integers without vocabulary constraints.
    /// Example: priority: 1-100 → encode as (value - 1), single token 0-99
    BoundedIntegerOffset { min: i64, max: i64, range: u32 },

    /// Integer: bounded range with codec-based multi-token encoding
    ///
    /// When integer range exceeds structural vocabulary budget, use multi-token encoding.
    /// Example: count: 1-1000 with max_structural=256 → 2 tokens in base-256
    /// NOTE: This is LOSSLESS - uses multiple tokens to preserve all information
    BoundedIntegerCodec {
        min: i64,
        max: i64,
        codec_spec: CodecSpec,
    },

    /// String: pattern-optimized encoding
    ///
    /// For strings matching a regex pattern with finite enumeration.
    /// Example: semver: "\\d+\\.\\d+\\.\\d+" → finite set of common versions
    PatternOptimized {
        pattern: String,
        strategy: PatternEncodingStrategy,
    },

    /// Enum: single variant (constant value)
    ///
    /// When an enum has only one variant, it's always that value - no encoding needed.
    /// Example: enum: ["default"] → constant "default", skip encoding entirely
    /// -O3 optimization: Saves 1 token per instance
    SingleVariantEnum { variant: String },

    /// Integer: constant value (min = max)
    ///
    /// When min equals max, the value is always the same - no encoding needed.
    /// Example: version: {minimum: 1, maximum: 1} → constant 1, skip encoding
    /// -O3 optimization: Saves 1+ tokens per instance
    ConstantInteger { value: i64 },

    /// Array: fixed size (minItems = maxItems)
    ///
    /// When minItems equals maxItems, length is constant - skip length encoding.
    /// Example: coordinates: {minItems: 2, maxItems: 2} → always 2 elements
    /// -O3 optimization: Saves 1 token per instance (length token)
    FixedSizeArray { size: usize, item_type: String },

    /// String: format-specific encoding
    ///
    /// Specialized encoding for well-known string formats.
    /// Example: date-time → encode as Unix timestamp (i64) instead of string
    /// Example: uuid → encode as 128-bit binary instead of 36-char string
    /// -O3 optimization: Significant token savings for structured string data
    FormatOptimized {
        format: StringFormat,
        fallback_to_string: bool, // Allow malformed values to fallback to string encoding
    },

    /// Struct: bit-packed optional field presence flags
    ///
    /// Packs presence flags for optional fields into u32 mask groups
    /// (32 flags per mask group).
    ///
    /// Each u32 mask group is then emitted through the generated structural
    /// codec, so it may occupy multiple base tokens depending on the mask value
    /// and encoding base.
    ///
    /// Example: struct with 10 optional fields → 1 presence-mask group (10 bits used)
    /// Example: struct with 40 optional fields → 2 presence-mask groups
    /// O3/Os optimization: Reduces per-field presence overhead for many-optional structs
    ///
    /// Note: Only applies to `Option<T>` fields (2-state: present/absent).
    /// `OptionalField<T>` (3-state: absent/null/present) continues using individual tokens.
    BitPackedStruct {
        /// Number of optional `Option<T>` fields in this struct
        optional_field_count: usize,
        /// Number of u32 presence-mask groups needed: ceil(optional_field_count / 32).
        ///
        /// This is not necessarily the number of emitted base tokens; each mask
        /// group is encoded with the generated structural codec.
        presence_tokens_needed: usize,
    },
}

impl EncodingDecision {
    /// Get human-readable name of this encoding strategy
    pub fn strategy_name(&self) -> &'static str {
        match self {
            Self::Standard => "standard",
            Self::EnumIndex { .. } => "enum_index",
            Self::EnumCodec { .. } => "enum_codec",
            Self::BoundedInteger { .. } => "bounded_integer",
            Self::BoundedIntegerOffset { .. } => "bounded_integer_offset",
            Self::BoundedIntegerCodec { .. } => "bounded_integer_codec",
            Self::PatternOptimized { .. } => "pattern_optimized",
            Self::SingleVariantEnum { .. } => "single_variant_enum",
            Self::ConstantInteger { .. } => "constant_integer",
            Self::FixedSizeArray { .. } => "fixed_size_array",
            Self::FormatOptimized { .. } => "format_optimized",
            Self::BitPackedStruct { .. } => "bit_packed_struct",
        }
    }

    /// Check if this decision respects a vocabulary constraint
    pub fn respects_vocabulary_constraint(&self, max_vocab: Option<usize>) -> bool {
        match (self, max_vocab) {
            // No constraint - always valid
            (_, None) => true,

            // Codec-based encoding uses multiple tokens (respects structural budget)
            (Self::EnumCodec { codec_spec, .. }, Some(max)) => codec_spec.base as usize <= max,
            (Self::BoundedIntegerCodec { codec_spec, .. }, Some(max)) => {
                codec_spec.base as usize <= max
            }

            // Unconstrained encodings must fit within limit
            (Self::EnumIndex { variants }, Some(max)) => variants.len() <= max,
            (Self::BoundedInteger { min, max: max_val }, Some(max)) => {
                bounded_i64_range_usage(*min, *max_val) <= max
            }
            (Self::BoundedIntegerOffset { range, .. }, Some(max)) => *range as usize <= max,

            // Standard and pattern-optimized encodings don't use vocabulary
            (Self::Standard, _) => true,
            (Self::PatternOptimized { .. }, _) => true,

            // O3 constant optimizations use zero vocabulary (always same value)
            (Self::SingleVariantEnum { .. }, _) => true,
            (Self::ConstantInteger { .. }, _) => true,
            (Self::FixedSizeArray { .. }, _) => true,

            // Format optimizations convert to structured encoding (no string vocabulary)
            (Self::FormatOptimized { .. }, _) => true,

            // Bit-packed struct encoding uses structural tokens (no vocabulary)
            (Self::BitPackedStruct { .. }, _) => true,
        }
    }

    /// Estimate vocabulary tokens used by this decision
    pub fn estimated_vocabulary_usage(&self) -> usize {
        match self {
            Self::Standard => 0, // Uses byte-level encoding
            Self::EnumIndex { variants } => variants.len(),
            Self::EnumCodec { codec_spec, .. } => codec_spec.base as usize,
            Self::BoundedInteger { min, max } => bounded_i64_range_usage(*min, *max),
            Self::BoundedIntegerOffset { range, .. } => *range as usize,
            Self::BoundedIntegerCodec { codec_spec, .. } => codec_spec.base as usize,
            Self::PatternOptimized { strategy, .. } => strategy.estimated_vocabulary_usage(),

            // O3 optimizations: zero vocabulary (constants don't need tokens)
            Self::SingleVariantEnum { .. } => 0,
            Self::ConstantInteger { .. } => 0,
            Self::FixedSizeArray { .. } => 0, // Length is constant, only items encoded

            // Format optimizations: structured encoding (no string vocabulary)
            Self::FormatOptimized { .. } => 0,

            // Bit-packed struct: structural encoding (no vocabulary)
            Self::BitPackedStruct { .. } => 0,
        }
    }
}

/// Specification for compile-time codec generation
///
/// Contains all information needed to generate brutally efficient encode/decode code
/// at compile time with zero runtime overhead.
#[derive(Debug, Clone, PartialEq)]
pub struct CodecSpec {
    /// Token width (number of tokens needed to encode max_value)
    /// Calculated as ceil(log_base(max_value + 1))
    pub token_width: usize,

    /// Encoding base (from max_vocab or calculated)
    pub base: u32,

    /// Maximum value to encode (e.g., variant count - 1, max integer value)
    pub max_value: u32,

    /// Whether base is a power of 2 (enables bit-shift optimization)
    pub is_power_of_2: bool,

    /// Bit shift amount (if power of 2)
    pub shift: Option<u32>,

    /// Bit mask (if power of 2)
    pub mask: Option<u32>,

    /// Divisors for each digit position (precomputed for non-power-of-2)
    /// Example: base=10, token_width=3 → [100, 10, 1]
    pub divisors: Vec<u32>,
}

impl CodecSpec {
    /// Calculate codec spec for given max_value and optional base constraint
    ///
    /// # Arguments
    /// * `max_value` - Maximum value to encode (e.g., variant_count - 1)
    /// * `base` - Optional encoding base from structural vocabulary budget
    ///
    /// # Returns
    /// Complete codec specification with precomputed values for code generation
    pub fn calculate(max_value: u32, base: Option<u32>) -> Self {
        // If no base constraint, use single token (base > max_value)
        // CRITICAL: Use saturating_add to avoid overflow when max_value = u32::MAX
        let base = base.unwrap_or_else(|| max_value.saturating_add(1));

        // CRITICAL: Ensure base >= 2 to avoid divide-by-zero and invalid logarithms
        // base=0: log(0) = -∞ → undefined behavior
        // base=1: log(1) = 0 → token_width=0 → division by zero
        // Minimum base=2 enables binary encoding for any value
        let base = base.max(2);

        // Calculate token width: ceil(log_base(max_value + 1))
        // CRITICAL: Use u64 to avoid overflow when max_value = u32::MAX
        let token_width = if base > max_value {
            1 // Single token sufficient
        } else {
            let log_result = ((max_value as u64 + 1) as f64).log(base as f64);
            log_result.ceil() as usize
        };

        // Check if base is power of 2
        let is_power_of_2 = base.is_power_of_two();
        let (shift, mask) = if is_power_of_2 {
            (Some(base.trailing_zeros()), Some(base - 1))
        } else {
            (None, None)
        };

        // Precompute divisors for each digit position (big-endian)
        // CRITICAL: Use u64 to avoid overflow when base^i exceeds u32::MAX
        // Example: base=256, token_width=4 → 256^4 = 2^32 overflows u32 to 0
        // This zero divisor would cause division-by-zero panics in decode
        let divisors: Vec<u32> = (0..token_width)
            .rev()
            .map(|i| {
                let divisor_u64 = (base as u64).pow(i as u32);
                // Safely convert back to u32 (capped at u32::MAX if overflow would occur)
                // For valid CodecSpec, divisor should always fit in u32
                // but we saturate to u32::MAX for safety rather than wrapping to 0
                divisor_u64.min(u32::MAX as u64) as u32
            })
            .collect();

        Self {
            token_width,
            base,
            max_value,
            is_power_of_2,
            shift,
            mask,
            divisors,
        }
    }
}

/// Strategy for pattern-optimized strings
#[derive(Debug, Clone, PartialEq)]
pub enum PatternEncodingStrategy {
    /// Finite enumeration of all possible values
    FiniteEnumeration { values: Vec<String> },

    /// Bounded range encoding (e.g., semver components)
    BoundedComponents { component_ranges: Vec<(i64, i64)> },
}

impl PatternEncodingStrategy {
    /// Estimate vocabulary usage for this pattern strategy
    pub fn estimated_vocabulary_usage(&self) -> usize {
        match self {
            Self::FiniteEnumeration { values } => values.len(),
            Self::BoundedComponents { component_ranges } => {
                // Each component uses a range of tokens
                component_ranges.iter().fold(0usize, |total, (min, max)| {
                    total.saturating_add(bounded_i64_range_usage(*min, *max))
                })
            }
        }
    }
}

/// String encoding mode: ASCII or UTF-8
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum StringEncoding {
    /// ASCII-only restricted-input strings (bytes 0-127, END_STRING_MARKER = 128)
    /// - Compact vocabulary: 129 base tokens
    /// - UTF-8 safe: all bytes are valid single-byte UTF-8
    /// - LLM efficient: smaller vocabulary improves training
    /// - Generated encoders reject non-ASCII input instead of emitting
    ///   token streams that cannot decode in ASCII mode
    Ascii,

    /// Full UTF-8 strings (bytes 0-255, END_STRING_MARKER = 256)
    /// - Complete Unicode support via UTF-8 byte encoding
    /// - Larger vocabulary: 257 base tokens
    /// - Handles all valid UTF-8 multi-byte sequences
    #[default]
    Utf8,
}

impl StringEncoding {
    /// Get the END_STRING_MARKER token for this encoding mode
    pub const fn end_marker(&self) -> u32 {
        match self {
            StringEncoding::Ascii => 128,
            StringEncoding::Utf8 => 256,
        }
    }

    /// Get the maximum valid byte value for string content
    pub const fn max_byte_value(&self) -> u32 {
        match self {
            StringEncoding::Ascii => 127,
            StringEncoding::Utf8 => 255,
        }
    }

    /// Get the base vocabulary size (content bytes + end marker)
    pub const fn base_vocab_size(&self) -> u32 {
        match self {
            StringEncoding::Ascii => 129, // 0-127 + END_MARKER(128)
            StringEncoding::Utf8 => 257,  // 0-255 + END_MARKER(256)
        }
    }
}

/// Compilation constraints (hard limits that MUST be respected)
#[derive(Debug, Clone, PartialEq, Default)]
pub struct CompilationConstraints {
    /// Maximum total vocabulary size (if specified)
    ///
    /// Structural vocabulary cap (`--max-vocab`): an enum or bounded integer whose
    /// variant count or range exceeds it is encoded with a multi-token codec in
    /// this base. It is not a limit on the generated `VOCAB_SIZE`; the string-token
    /// capacity and the encoding base are not capped by it.
    pub max_vocab: Option<usize>,

    /// String encoding mode: UTF-8 (default) or ASCII restricted-input mode
    ///
    /// Controls string byte encoding and END_STRING_MARKER token:
    /// - ASCII: bytes 0-127, END_MARKER = 128 (129 base tokens)
    /// - UTF-8: bytes 0-255, END_MARKER = 256 (257 base tokens)
    ///
    /// Default: UTF-8 so ordinary JSON strings preserve Unicode unless ASCII
    /// restricted-input mode is selected explicitly.
    pub string_encoding: StringEncoding,
    // Note: max_string_length removed in favor of end-marker string encoding
    // Strings now use END_STRING_MARKER to signal end, eliminating length constraints
    // Future constraints:
    // pub max_nesting_depth: Option<usize>,
    // pub require_constant_time_decode: bool,
}

impl CompilationConstraints {
    /// Create new constraints with default values
    pub fn new() -> Self {
        Self::default()
    }

    /// Create constraints with maximum total vocabulary size
    ///
    /// Sets the total vocabulary limit across all layers (base + BPE + special tokens).
    pub fn with_max_vocabulary(max_size: usize) -> Self {
        Self {
            max_vocab: Some(max_size),
            ..Default::default()
        }
    }

    /// Set string encoding mode (ASCII or UTF-8)
    ///
    /// # Arguments
    /// * `encoding` - StringEncoding::Utf8 (default) or StringEncoding::Ascii
    ///
    /// # Examples
    /// ```
    /// use tct::generator::encoding_decision::{CompilationConstraints, StringEncoding};
    ///
    /// // ASCII restricted-input mode: bytes 0-127, END_MARKER = 128
    /// let ascii_constraints = CompilationConstraints::new()
    ///     .with_string_encoding(StringEncoding::Ascii);
    ///
    /// // UTF-8 mode: bytes 0-255, END_MARKER = 256
    /// let utf8_constraints = CompilationConstraints::new()
    ///     .with_string_encoding(StringEncoding::Utf8);
    /// ```
    pub fn with_string_encoding(mut self, encoding: StringEncoding) -> Self {
        self.string_encoding = encoding;
        self
    }

    /// Check if constraints are satisfied by an encoding strategy
    pub fn are_satisfied_by(&self, strategy: &EncodingStrategy) -> bool {
        // Check vocabulary constraint
        // IMPORTANT: Vocabulary is the MAXIMUM usage across all contexts, not the SUM!
        // Tokens are reused across different structural elements (context-dependent).
        if let Some(max_vocab) = self.max_vocab {
            let max_usage: usize = strategy
                .decisions
                .values()
                .map(|d| d.estimated_vocabulary_usage())
                .max()
                .unwrap_or(0);

            if max_usage > max_vocab {
                return false;
            }

            // Also check that each individual decision respects constraint
            for decision in strategy.decisions.values() {
                if !decision.respects_vocabulary_constraint(Some(max_vocab)) {
                    return false;
                }
            }
        }

        // Future constraint checks go here

        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encoding_decision_strategy_name() {
        assert_eq!(EncodingDecision::Standard.strategy_name(), "standard");
        assert_eq!(
            EncodingDecision::EnumIndex { variants: vec![] }.strategy_name(),
            "enum_index"
        );
    }

    #[test]
    fn test_encoding_decision_respects_constraint() {
        let decision = EncodingDecision::EnumIndex {
            variants: vec!["a".to_string(), "b".to_string(), "c".to_string()],
        };

        assert!(decision.respects_vocabulary_constraint(None));
        assert!(decision.respects_vocabulary_constraint(Some(10)));
        assert!(!decision.respects_vocabulary_constraint(Some(2)));
    }

    #[test]
    fn test_codec_spec_single_token() {
        // 5 variants, no constraint → single token
        let spec = CodecSpec::calculate(5, None);
        assert_eq!(spec.token_width, 1);
        assert_eq!(spec.base, 6); // base > max_value
        assert_eq!(spec.max_value, 5);
        assert_eq!(spec.divisors, vec![1]);
    }

    #[test]
    fn test_codec_spec_multi_token() {
        // 200 variants, base=10 → 3 tokens (ceil(log₁₀(200)))
        let spec = CodecSpec::calculate(199, Some(10));
        assert_eq!(spec.token_width, 3);
        assert_eq!(spec.base, 10);
        assert_eq!(spec.max_value, 199);
        assert!(!spec.is_power_of_2);
        assert_eq!(spec.divisors, vec![100, 10, 1]);
    }

    #[test]
    fn test_codec_spec_power_of_2() {
        // 1000 variants, base=256 → 2 tokens with bit shifts
        let spec = CodecSpec::calculate(999, Some(256));
        assert_eq!(spec.token_width, 2);
        assert_eq!(spec.base, 256);
        assert!(spec.is_power_of_2);
        assert_eq!(spec.shift, Some(8)); // 256 = 2^8
        assert_eq!(spec.mask, Some(255)); // 0xFF
        assert_eq!(spec.divisors, vec![256, 1]);
    }

    #[test]
    fn test_codec_spec_exact_power() {
        // Exactly base^n values
        let spec = CodecSpec::calculate(99, Some(10)); // 100 values (10^2)
        assert_eq!(spec.token_width, 2);
        assert_eq!(spec.divisors, vec![10, 1]);
    }

    #[test]
    fn test_codec_spec_base_zero() {
        // base=0 should be clamped to base=2 (binary encoding)
        let spec = CodecSpec::calculate(15, Some(0));
        assert_eq!(spec.base, 2); // Clamped to minimum
        assert_eq!(spec.token_width, 4); // ceil(log₂(16)) = 4
        assert_eq!(spec.divisors, vec![8, 4, 2, 1]); // Powers of 2
    }

    #[test]
    fn test_codec_spec_base_one() {
        // base=1 should be clamped to base=2 (binary encoding)
        let spec = CodecSpec::calculate(15, Some(1));
        assert_eq!(spec.base, 2); // Clamped to minimum
        assert_eq!(spec.token_width, 4); // ceil(log₂(16)) = 4
        assert_eq!(spec.divisors, vec![8, 4, 2, 1]); // Powers of 2
    }

    #[test]
    fn test_encoding_strategy_has_optimizations() {
        let mut strategy = EncodingStrategy::new();
        assert!(!strategy.has_optimizations());

        strategy.add_decision("Status", EncodingDecision::Standard);
        assert!(!strategy.has_optimizations());

        strategy.add_decision(
            "Priority",
            EncodingDecision::EnumIndex {
                variants: vec!["low".to_string(), "high".to_string()],
            },
        );
        assert!(strategy.has_optimizations());
    }

    #[test]
    fn test_compilation_constraints_satisfied() {
        let constraints = CompilationConstraints::with_max_vocabulary(100);

        let mut strategy = EncodingStrategy::new();
        strategy.add_decision(
            "Status",
            EncodingDecision::EnumIndex {
                variants: vec!["a".to_string(); 50],
            },
        );

        assert!(constraints.are_satisfied_by(&strategy));

        strategy.add_decision(
            "Priority",
            EncodingDecision::EnumIndex {
                variants: vec!["b".to_string(); 60],
            },
        );

        // Both enums fit within constraint (context-dependent tokens: max(50, 60) = 60 < 100)
        assert!(constraints.are_satisfied_by(&strategy));

        // Add enum that exceeds constraint
        strategy.add_decision(
            "Category",
            EncodingDecision::EnumIndex {
                variants: vec!["c".to_string(); 120],
            },
        );

        // max(50, 60, 120) = 120 > 100
        assert!(!constraints.are_satisfied_by(&strategy));
    }

    // ============================================================================
    // Additional comprehensive tests for Phase 4.8
    // ============================================================================

    // Batch 1 - EncodingStrategy Tests (7 tests)
    #[test]
    fn test_encoding_strategy_new() {
        let strategy = EncodingStrategy::new();
        assert!(strategy.decisions.is_empty());
        assert_eq!(strategy.max_vocabulary_size, None);
        assert_eq!(strategy.stats.optimized_fields, 0);
    }

    #[test]
    fn test_encoding_strategy_default() {
        let strategy = EncodingStrategy::default();
        assert!(strategy.decisions.is_empty());
        assert!(strategy.member_type_refinements.is_empty());
        assert_eq!(strategy.max_vocabulary_size, None);
    }

    #[test]
    fn test_encoding_strategy_get_decision() {
        let mut strategy = EncodingStrategy::new();

        // Non-existent type returns Standard
        assert_eq!(
            strategy.get_decision("NonExistent"),
            EncodingDecision::Standard
        );

        // Add decision and retrieve it
        strategy.add_decision(
            "Status",
            EncodingDecision::EnumIndex {
                variants: vec!["active".to_string(), "inactive".to_string()],
            },
        );

        assert_eq!(
            strategy.get_decision("Status"),
            EncodingDecision::EnumIndex {
                variants: vec!["active".to_string(), "inactive".to_string()],
            }
        );
    }

    #[test]
    fn test_encoding_strategy_member_type_refinement() {
        let mut strategy = EncodingStrategy::new();

        strategy.add_member_type_refinement(
            "Config",
            "additional_properties",
            MemberTypeRefinement::HashMap {
                key: "String".to_string(),
                value: "String".to_string(),
            },
        );

        let path = MemberPath::new("Config", "additional_properties");
        assert_eq!(
            strategy.member_type_refinements.get(&path),
            Some(&MemberTypeRefinement::HashMap {
                key: "String".to_string(),
                value: "String".to_string(),
            })
        );
    }

    #[test]
    fn test_encoding_strategy_count_by_strategy() {
        let mut strategy = EncodingStrategy::new();

        strategy.add_decision(
            "Status",
            EncodingDecision::EnumIndex {
                variants: vec!["a".to_string()],
            },
        );
        strategy.add_decision(
            "Priority",
            EncodingDecision::EnumIndex {
                variants: vec!["low".to_string()],
            },
        );
        strategy.add_decision(
            "Count",
            EncodingDecision::BoundedInteger { min: 0, max: 100 },
        );
        strategy.add_decision("Name", EncodingDecision::Standard);

        let counts = strategy.count_by_strategy();
        assert_eq!(counts.get("enum_index"), Some(&2));
        assert_eq!(counts.get("bounded_integer"), Some(&1));
        assert_eq!(counts.get("standard"), Some(&1));
    }

    #[test]
    fn test_encoding_strategy_update_stats_enum_index() {
        let mut strategy = EncodingStrategy::new();
        strategy.add_decision(
            "Status",
            EncodingDecision::EnumIndex {
                variants: vec![
                    "active".to_string(),
                    "inactive".to_string(),
                    "pending".to_string(),
                ],
            },
        );

        strategy.update_stats();

        assert_eq!(strategy.stats.optimized_fields, 1);
        assert_eq!(strategy.stats.enum_domains, 1);
        assert_eq!(strategy.stats.total_constrained_values, 3);
        assert_eq!(strategy.stats.naive_vocabulary_size, 3);
        assert_eq!(strategy.stats.distinct_structural_tokens_count, 3);
    }

    #[test]
    fn test_encoding_strategy_update_stats_enum_codec() {
        let mut strategy = EncodingStrategy::new();
        strategy.add_decision(
            "Status",
            EncodingDecision::EnumCodec {
                variants: vec!["v".to_string(); 200],
                codec_spec: CodecSpec::calculate(199, Some(10)),
            },
        );

        strategy.update_stats();

        assert_eq!(strategy.stats.optimized_fields, 1);
        assert_eq!(strategy.stats.enum_domains, 1);
        assert_eq!(strategy.stats.total_constrained_values, 200);
        assert_eq!(strategy.stats.naive_vocabulary_size, 200);
        assert_eq!(strategy.stats.distinct_structural_tokens_count, 10); // base=10
        assert!(strategy.stats.tokens_saved > 0);
    }

    #[test]
    fn test_encoding_strategy_update_stats_bounded_integer() {
        let mut strategy = EncodingStrategy::new();
        strategy.add_decision(
            "Priority",
            EncodingDecision::BoundedInteger { min: 1, max: 10 },
        );

        strategy.update_stats();

        assert_eq!(strategy.stats.optimized_fields, 1);
        assert_eq!(strategy.stats.bounded_integer_domains, 1);
        assert_eq!(strategy.stats.total_constrained_values, 10); // 1-10 = 10 values
    }

    #[test]
    fn test_encoding_strategy_update_stats_extreme_bounded_integer_does_not_overflow() {
        let mut strategy = EncodingStrategy::new();
        strategy.add_decision(
            "Extreme",
            EncodingDecision::BoundedInteger {
                min: i64::MIN,
                max: i64::MAX,
            },
        );

        strategy.update_stats();

        assert_eq!(strategy.stats.optimized_fields, 1);
        assert_eq!(strategy.stats.bounded_integer_domains, 1);
        assert_eq!(strategy.stats.total_constrained_values, usize::MAX);
        assert_eq!(strategy.stats.naive_vocabulary_size, usize::MAX);
        assert_eq!(strategy.stats.distinct_structural_tokens_count, usize::MAX);
        assert_eq!(
            strategy.stats.max_structural_token_id_plus_one,
            u32_plus_one_to_usize_saturating(u32::MAX)
        );
        assert!(strategy.stats.reduction_percentage.is_finite());
    }

    // Batch 2 - EncodingDecision Tests (10 tests)
    #[test]
    fn test_encoding_decision_all_strategy_names() {
        assert_eq!(EncodingDecision::Standard.strategy_name(), "standard");
        assert_eq!(
            EncodingDecision::EnumIndex { variants: vec![] }.strategy_name(),
            "enum_index"
        );
        assert_eq!(
            EncodingDecision::EnumCodec {
                variants: vec![],
                codec_spec: CodecSpec::calculate(0, Some(2))
            }
            .strategy_name(),
            "enum_codec"
        );
        assert_eq!(
            EncodingDecision::BoundedInteger { min: 0, max: 10 }.strategy_name(),
            "bounded_integer"
        );
        assert_eq!(
            EncodingDecision::BoundedIntegerOffset {
                min: 0,
                max: 10,
                range: 11
            }
            .strategy_name(),
            "bounded_integer_offset"
        );
        assert_eq!(
            EncodingDecision::BoundedIntegerCodec {
                min: 0,
                max: 1000,
                codec_spec: CodecSpec::calculate(1000, Some(10))
            }
            .strategy_name(),
            "bounded_integer_codec"
        );
        assert_eq!(
            EncodingDecision::PatternOptimized {
                pattern: "\\d+".to_string(),
                strategy: PatternEncodingStrategy::FiniteEnumeration { values: vec![] }
            }
            .strategy_name(),
            "pattern_optimized"
        );
    }

    #[test]
    fn test_encoding_decision_estimated_vocabulary_usage() {
        // Standard: 0 (byte-level encoding)
        assert_eq!(EncodingDecision::Standard.estimated_vocabulary_usage(), 0);

        // EnumIndex: variant count
        assert_eq!(
            EncodingDecision::EnumIndex {
                variants: vec!["a".to_string(), "b".to_string(), "c".to_string()]
            }
            .estimated_vocabulary_usage(),
            3
        );

        // EnumCodec: base (multi-token)
        assert_eq!(
            EncodingDecision::EnumCodec {
                variants: vec!["v".to_string(); 200],
                codec_spec: CodecSpec::calculate(199, Some(10))
            }
            .estimated_vocabulary_usage(),
            10
        );

        // BoundedInteger: range size
        assert_eq!(
            EncodingDecision::BoundedInteger { min: 5, max: 10 }.estimated_vocabulary_usage(),
            6 // 5,6,7,8,9,10
        );

        // BoundedIntegerOffset: range
        assert_eq!(
            EncodingDecision::BoundedIntegerOffset {
                min: 0,
                max: 99,
                range: 100
            }
            .estimated_vocabulary_usage(),
            100
        );

        // BoundedIntegerCodec: base
        assert_eq!(
            EncodingDecision::BoundedIntegerCodec {
                min: 0,
                max: 1000,
                codec_spec: CodecSpec::calculate(1000, Some(256))
            }
            .estimated_vocabulary_usage(),
            256
        );
    }

    #[test]
    fn test_encoding_decision_extreme_and_invalid_bounded_integer_usage_saturates() {
        assert_eq!(
            EncodingDecision::BoundedInteger {
                min: i64::MIN,
                max: i64::MAX
            }
            .estimated_vocabulary_usage(),
            usize::MAX
        );
        assert_eq!(
            EncodingDecision::BoundedInteger { min: 10, max: 1 }.estimated_vocabulary_usage(),
            usize::MAX
        );
    }

    #[test]
    fn test_encoding_decision_respects_constraint_no_constraint() {
        // All decisions should pass with no constraint
        assert!(EncodingDecision::Standard.respects_vocabulary_constraint(None));
        assert!(
            EncodingDecision::EnumIndex {
                variants: vec!["a".to_string(); 1000]
            }
            .respects_vocabulary_constraint(None)
        );
    }

    #[test]
    fn test_encoding_decision_respects_constraint_enum_codec() {
        let decision = EncodingDecision::EnumCodec {
            variants: vec!["v".to_string(); 200],
            codec_spec: CodecSpec::calculate(199, Some(10)),
        };

        assert!(decision.respects_vocabulary_constraint(None));
        assert!(decision.respects_vocabulary_constraint(Some(10)));
        assert!(decision.respects_vocabulary_constraint(Some(20)));
        assert!(!decision.respects_vocabulary_constraint(Some(5)));
    }

    #[test]
    fn test_encoding_decision_respects_constraint_bounded_integer() {
        let decision = EncodingDecision::BoundedInteger { min: 0, max: 99 };

        assert!(decision.respects_vocabulary_constraint(None));
        assert!(decision.respects_vocabulary_constraint(Some(100)));
        assert!(decision.respects_vocabulary_constraint(Some(200)));
        assert!(!decision.respects_vocabulary_constraint(Some(50)));
    }

    #[test]
    fn test_encoding_decision_invalid_bounded_integer_does_not_fit_constraint() {
        let decision = EncodingDecision::BoundedInteger { min: 10, max: 1 };

        assert!(!decision.respects_vocabulary_constraint(Some(1)));
        assert!(!decision.respects_vocabulary_constraint(Some(1024)));
    }

    #[test]
    fn test_encoding_decision_respects_constraint_bounded_integer_offset() {
        let decision = EncodingDecision::BoundedIntegerOffset {
            min: 0,
            max: 99,
            range: 100,
        };

        assert!(decision.respects_vocabulary_constraint(Some(100)));
        assert!(!decision.respects_vocabulary_constraint(Some(50)));
    }

    #[test]
    fn test_encoding_decision_respects_constraint_bounded_integer_codec() {
        let decision = EncodingDecision::BoundedIntegerCodec {
            min: 0,
            max: 1000,
            codec_spec: CodecSpec::calculate(1000, Some(256)),
        };

        assert!(decision.respects_vocabulary_constraint(Some(256)));
        assert!(!decision.respects_vocabulary_constraint(Some(100)));
    }

    #[test]
    fn test_encoding_decision_respects_constraint_standard() {
        let decision = EncodingDecision::Standard;
        assert!(decision.respects_vocabulary_constraint(None));
        assert!(decision.respects_vocabulary_constraint(Some(1)));
        assert!(decision.respects_vocabulary_constraint(Some(1000)));
    }

    #[test]
    fn test_encoding_decision_respects_constraint_pattern_optimized() {
        let decision = EncodingDecision::PatternOptimized {
            pattern: "\\d+\\.\\d+\\.\\d+".to_string(),
            strategy: PatternEncodingStrategy::FiniteEnumeration {
                values: vec!["1.0.0".to_string(), "2.0.0".to_string()],
            },
        };

        // Pattern-optimized doesn't use vocabulary (byte-level encoding)
        assert!(decision.respects_vocabulary_constraint(None));
        assert!(decision.respects_vocabulary_constraint(Some(1)));
    }

    #[test]
    fn test_encoding_decision_equality() {
        let d1 = EncodingDecision::EnumIndex {
            variants: vec!["a".to_string(), "b".to_string()],
        };
        let d2 = EncodingDecision::EnumIndex {
            variants: vec!["a".to_string(), "b".to_string()],
        };
        let d3 = EncodingDecision::EnumIndex {
            variants: vec!["a".to_string(), "c".to_string()],
        };

        assert_eq!(d1, d2);
        assert_ne!(d1, d3);
    }

    // Batch 3 - CodecSpec Tests (5 tests)
    #[test]
    fn test_codec_spec_large_value_small_base() {
        // Very large value with small base requires many tokens
        let spec = CodecSpec::calculate(999999, Some(10));
        assert_eq!(spec.token_width, 6); // ceil(log₁₀(1000000))
        assert_eq!(spec.base, 10);
        assert_eq!(spec.divisors, vec![100000, 10000, 1000, 100, 10, 1]);
    }

    #[test]
    fn test_codec_spec_various_power_of_2() {
        // Base 4 (2^2)
        let spec = CodecSpec::calculate(15, Some(4));
        assert!(spec.is_power_of_2);
        assert_eq!(spec.shift, Some(2));
        assert_eq!(spec.mask, Some(3));

        // Base 64 (2^6)
        let spec = CodecSpec::calculate(4095, Some(64));
        assert!(spec.is_power_of_2);
        assert_eq!(spec.shift, Some(6));
        assert_eq!(spec.mask, Some(63));
    }

    #[test]
    fn test_codec_spec_non_power_of_2_various_bases() {
        // Base 3
        let spec = CodecSpec::calculate(26, Some(3)); // 27 values (3^3)
        assert!(!spec.is_power_of_2);
        assert_eq!(spec.token_width, 3);
        assert_eq!(spec.divisors, vec![9, 3, 1]);

        // Base 5 - use value clearly < 5^3 to avoid floating point edge case
        let spec = CodecSpec::calculate(99, Some(5)); // 100 values (< 5^3 = 125)
        assert!(!spec.is_power_of_2);
        assert_eq!(spec.token_width, 3); // ceil(log_5(100)) = 3
        assert_eq!(spec.divisors, vec![25, 5, 1]);
    }

    #[test]
    fn test_codec_spec_edge_case_max_value_zero() {
        // Single value (0) should use single token
        let spec = CodecSpec::calculate(0, None);
        assert_eq!(spec.token_width, 1);
        assert_eq!(spec.base, 2); // Clamped from 1 to 2
        assert_eq!(spec.max_value, 0);
    }

    #[test]
    fn test_codec_spec_base_equals_max_value_plus_one() {
        // When base exactly equals max_value + 1, should use single token
        let spec = CodecSpec::calculate(9, Some(10));
        assert_eq!(spec.token_width, 1);
        assert_eq!(spec.base, 10);
        assert_eq!(spec.divisors, vec![1]);
    }

    // Batch 4 - PatternEncodingStrategy Tests (2 tests)
    #[test]
    fn test_pattern_strategy_finite_enumeration() {
        let strategy = PatternEncodingStrategy::FiniteEnumeration {
            values: vec![
                "1.0.0".to_string(),
                "2.0.0".to_string(),
                "3.0.0".to_string(),
            ],
        };

        assert_eq!(strategy.estimated_vocabulary_usage(), 3);
    }

    #[test]
    fn test_pattern_strategy_bounded_components() {
        let strategy = PatternEncodingStrategy::BoundedComponents {
            component_ranges: vec![(0, 9), (0, 9), (0, 99)], // 10 + 10 + 100 = 120
        };

        assert_eq!(strategy.estimated_vocabulary_usage(), 120);
    }

    #[test]
    fn test_pattern_strategy_invalid_bounded_component_saturates_usage() {
        let strategy = PatternEncodingStrategy::BoundedComponents {
            component_ranges: vec![(0, 9), (10, 1)],
        };

        assert_eq!(strategy.estimated_vocabulary_usage(), usize::MAX);
    }

    // Batch 5 - CompilationConstraints Tests (9 tests)
    #[test]
    fn test_compilation_constraints_new() {
        let constraints = CompilationConstraints::new();
        assert_eq!(constraints.max_vocab, None);
    }

    #[test]
    fn test_compilation_constraints_default() {
        let constraints = CompilationConstraints::default();
        assert_eq!(constraints.max_vocab, None);
    }

    #[test]
    fn test_compilation_constraints_with_max_vocabulary() {
        let constraints = CompilationConstraints::with_max_vocabulary(1000);
        assert_eq!(constraints.max_vocab, Some(1000));
    }

    #[test]
    fn test_compilation_constraints_are_satisfied_no_constraint() {
        let constraints = CompilationConstraints::new();
        let mut strategy = EncodingStrategy::new();
        strategy.add_decision(
            "Status",
            EncodingDecision::EnumIndex {
                variants: vec!["a".to_string(); 10000],
            },
        );

        // No constraint - always satisfied
        assert!(constraints.are_satisfied_by(&strategy));
    }
}

// Note: Tests for max_string_length removed - end-marker encoding eliminates length constraints
