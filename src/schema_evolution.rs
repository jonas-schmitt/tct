//! Schema evolution helpers for token-ID layout comparison.
//!
//! These helpers produce reportable token-ID layout manifests for old/new
//! schema pairs. They intentionally compare generated tokenizer layout metadata
//! without claiming semantic compatibility between schemas.

use crate::bpe::bpe_start_token;
use crate::error::{TctError, TctResult};
use crate::generator::encoding_decision::StringEncoding;
use crate::provenance::hex_lower;
use crate::{OptimizationLevel, TctCompiler};
use serde::Serialize;
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::path::Path;
use std::time::Duration;
use std::time::Instant;

mod bpe_artifact;
mod policy;
mod structural;

pub use bpe_artifact::{
    BpeArtifactCompatibility, BpeArtifactDiff, BpeArtifactLayout, summarize_bpe_artifact_layout,
};
use bpe_artifact::{bpe_artifact_base_vocab_size, bpe_artifact_schema, diff_bpe_artifacts};
use policy::{assess_stable_vocabulary_policy, build_vocabulary_extension_handoff};
use structural::{
    assess_structural_compatibility, structural_promotion_allowed, summarize_structural_token_map,
};

/// Half-open token ID range `[start, end_exclusive)`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct TokenIdRange {
    pub start: u32,
    pub end_exclusive: u32,
}

/// Generated tokenizer token-ID layout for one schema/configuration.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct TokenIdLayout {
    pub root_type_name: String,
    pub optimization_level: String,
    pub optimization_pass_overrides: OptimizationPassOverrides,
    pub string_encoding: String,
    pub max_base_vocab: Option<u32>,
    pub base_vocab_size: u32,
    pub encoding_base: u32,
    pub bpe_start: u32,
    pub bpe_merges: u32,
    pub bpe_range: Option<TokenIdRange>,
    pub bpe_artifact: Option<BpeArtifactLayout>,
    pub structural_token_map: StructuralTokenMap,
    pub pad_token: u32,
    pub vocab_size: u32,
    pub compile_wall_time_us: u64,
    pub generate_code_wall_time_us: u64,
    pub generated_code_bytes: u64,
    pub generated_code_sha256: String,
}

/// Structured token-semantic summary for conservative schema-evolution checks.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct StructuralTokenMap {
    pub root_kind: StructuralRootKind,
    pub schema_sha256: String,
    pub entries: Vec<StructuralTokenEntry>,
}

/// Root schema shape used by structural compatibility checks.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum StructuralRootKind {
    Object,
    Enum,
    Const,
    Opaque,
}

/// One token-relevant structural entry.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct StructuralTokenEntry {
    pub path: String,
    pub generated_order: Option<usize>,
    pub role: StructuralTokenRole,
    pub required: Option<bool>,
    pub member_type_path: Option<String>,
    pub encoding_decision: Option<String>,
    pub emission: StructuralEmissionMetadata,
    pub schema_sha256: String,
    pub value_sha256: Option<String>,
}

/// Per-member evidence for how a structural entry contributes to token streams.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct StructuralEmissionMetadata {
    pub parent_encoding_decision: Option<String>,
    pub selector: StructuralSelectorEmission,
    pub value: StructuralValueEmission,
    pub strict_stream_shape: StructuralStrictStreamShape,
    pub catch_all_map: bool,
    pub nested_object: bool,
}

/// Structural selector, presence, or state emission for one entry.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct StructuralSelectorEmission {
    pub kind: StructuralSelectorKind,
    pub fixed_token_count: Option<usize>,
    pub bit_packed_presence: Option<BitPackedPresenceMetadata>,
}

/// Bit-packed presence-bit location for optional `Option<T>` fields.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct BitPackedPresenceMetadata {
    pub optional_field_index: usize,
    pub presence_mask_index: usize,
    pub bit_index: usize,
    pub presence_mask_count: usize,
}

/// How an entry's selector or presence state is emitted.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum StructuralSelectorKind {
    None,
    EnumVariantToken,
    OptionPresenceFlag,
    OptionalFieldStateToken,
    BitPackedPresenceBit,
    Opaque,
}

/// Value-token emission for one structural entry.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct StructuralValueEmission {
    pub emits_when_present: bool,
    pub width: StructuralValueWidth,
    pub fixed_token_count: Option<usize>,
}

/// Conservative value width class for per-member stream review.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum StructuralValueWidth {
    Zero,
    Fixed,
    Variable,
    SchemaDependent,
    Opaque,
}

/// Strict stream contribution shape for compatibility and migration review.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum StructuralStrictStreamShape {
    ZeroTokens,
    EnumSelector,
    RequiredValueTokens,
    PresenceFlagThenValueIfPresent,
    OptionalStateThenValueIfPresent,
    SharedPresenceMaskThenValueIfPresent,
    Opaque,
}

/// Conservative role for how a schema member contributes to token streams.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum StructuralTokenRole {
    EnumVariant,
    RequiredValue,
    OptionPresenceAndValue,
    OptionalFieldStateAndValue,
    ZeroTokenConst,
    Opaque,
}

/// Explicit optimization pass overrides applied on top of an optimization profile.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize)]
pub struct OptimizationPassOverrides {
    pub enum_optimization: bool,
    pub bounded_int_optimization: bool,
    /// Reserved for historical manifests; compile helpers reject it until
    /// direct finite-regex encoding has a generated-tokenizer contract.
    pub pattern_optimization: bool,
    pub bit_packing: bool,
}

impl OptimizationPassOverrides {
    pub fn with_enum_optimization(mut self) -> Self {
        self.enum_optimization = true;
        self
    }

    pub fn with_bounded_int_optimization(mut self) -> Self {
        self.bounded_int_optimization = true;
        self
    }

    /// Mark the reserved pattern optimization override.
    ///
    /// `compile_token_id_layout_with_options` rejects this until direct
    /// finite-regex encoding is implemented.
    pub fn with_pattern_optimization(mut self) -> Self {
        self.pattern_optimization = true;
        self
    }

    pub fn with_bit_packing(mut self) -> Self {
        self.bit_packing = true;
        self
    }

    pub fn union(self, other: Self) -> Self {
        Self {
            enum_optimization: self.enum_optimization || other.enum_optimization,
            bounded_int_optimization: self.bounded_int_optimization
                || other.bounded_int_optimization,
            pattern_optimization: self.pattern_optimization || other.pattern_optimization,
            bit_packing: self.bit_packing || other.bit_packing,
        }
    }
}

/// Token-ID layout compatibility classification.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum TokenIdLayoutCompatibility {
    /// Token layout and generated source hash are identical.
    Unchanged,
    /// Token layout constants are identical, but generated code changed.
    ///
    /// This is not a semantic-compatibility guarantee. It only means the
    /// top-level ID ranges did not move.
    TokenLayoutUnchanged,
    /// One or more top-level token-ID ranges moved.
    TokenLayoutChanged,
}

/// Stable-vocabulary policy decision for a schema transition.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum StableVocabularyDecision {
    /// The generated tokenizer and top-level token layout are unchanged.
    ExactReuse,
    /// Existing schema-local token IDs and stream shapes are proven stable.
    AdditiveCompatible,
    /// The top-level layout is unchanged, but generated code changed.
    ///
    /// Additive schemas usually land here today: the manifest cannot yet prove
    /// that existing structural token IDs still have identical meaning.
    LayoutOnlyUnchanged,
    /// Top-level token ranges moved.
    BreakingChange,
}

/// Reuse decision for a generated or downstream artifact.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ArtifactReuseDecision {
    /// Existing artifact can be reused for this schema/configuration.
    Reusable,
    /// Existing artifact may be reusable, but requires a structural policy review.
    ReviewRequired,
    /// Existing artifact must be regenerated or migrated explicitly.
    RegenerateRequired,
}

/// Explicit evidence boundary for token-diff reports.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct TokenDiffExperimentGate {
    pub evidence_scope: &'static str,
    pub downstream_claims_allowed: bool,
    pub required_downstream_provenance: Vec<&'static str>,
    pub notes: Vec<&'static str>,
}

pub fn token_diff_experiment_gate() -> TokenDiffExperimentGate {
    TokenDiffExperimentGate {
        evidence_scope: "compiler_artifact_evidence_only",
        downstream_claims_allowed: false,
        required_downstream_provenance: vec![
            "old_schema_file_and_hash",
            "new_schema_file_and_hash",
            "token_diff_report",
            "bpe_artifact_fingerprints",
            "corpus_or_source_manifest",
            "random_seed_list",
            "training_command",
            "evaluation_command",
            "checkpoint_identifier",
            "generation_settings",
            "constrained_decoding_settings",
            "result_files",
            "tct_commit",
            "downstream_repo_commits",
        ],
        notes: vec![
            "Token-diff reports are compiler-local compatibility evidence, not downstream model evidence.",
            "Stable-vocabulary decisions do not by themselves justify paper, training, or embedding-reuse claims.",
            "Downstream experiments must preserve the required provenance before updating result claims.",
        ],
    }
}

/// Conservative stable-vocabulary assessment for a schema transition.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct StableVocabularyAssessment {
    pub decision: StableVocabularyDecision,
    pub generated_tokenizer_reuse: ArtifactReuseDecision,
    pub downstream_bpe_or_embedding_reuse: ArtifactReuseDecision,
    pub reasons: Vec<String>,
}

/// Versioned compiler-side handoff for downstream vocabulary-extension review.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct VocabularyExtensionHandoff {
    pub artifact_kind: &'static str,
    pub artifact_version: u32,
    pub status: VocabularyExtensionStatus,
    pub stable_vocabulary_decision: StableVocabularyDecision,
    pub generated_tokenizer_reuse: ArtifactReuseDecision,
    pub downstream_embedding_handoff: DownstreamEmbeddingHandoff,
    pub old_schema_sha256: String,
    pub new_schema_sha256: String,
    pub layout_affecting_inputs_match: bool,
    pub old_vocab_size: u32,
    pub new_vocab_size: u32,
    pub vocab_size_delta: i64,
    pub old_vocab_range: TokenIdRange,
    pub new_vocab_range: TokenIdRange,
    pub reusable_schema_local_token_range: Option<TokenIdRange>,
    pub appended_token_ranges: Vec<TokenIdRange>,
    pub new_structural_paths: Vec<String>,
    pub bpe_artifact_compatibility: BpeArtifactCompatibility,
    pub bpe_prefix_extension: Option<bool>,
    pub old_bpe_table_sha256: Option<String>,
    pub new_bpe_table_sha256: Option<String>,
    pub old_bpe_merge_sequence_sha256: Option<String>,
    pub new_bpe_merge_sequence_sha256: Option<String>,
    pub required_downstream_metadata: Vec<&'static str>,
    pub notes: Vec<String>,
}

/// Handoff status for downstream vocabulary extension.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum VocabularyExtensionStatus {
    NoExtensionNeeded,
    CandidateExtensionArtifact,
    ReviewOnly,
    RegenerationRequired,
}

/// Downstream embedding-row policy implied by a compiler-side handoff.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum DownstreamEmbeddingHandoff {
    ReuseExistingRows,
    InitializeOrReviewRows,
    ReviewRequired,
    RegenerateOrMigrate,
}

/// Structural-token compatibility assessment for old/new schemas.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct StructuralCompatibilityAssessment {
    pub decision: StructuralCompatibilityDecision,
    pub changed_paths: Vec<String>,
    pub reasons: Vec<String>,
}

/// Conservative structural compatibility classification.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum StructuralCompatibilityDecision {
    Unchanged,
    AdditiveCompatible,
    BreakingChange,
    NotProven,
}

/// Difference between old and new token-ID layouts.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct TokenIdLayoutDiff {
    pub compatibility: TokenIdLayoutCompatibility,
    pub stable_vocabulary: StableVocabularyAssessment,
    pub structural_compatibility: StructuralCompatibilityAssessment,
    pub bpe_artifact: BpeArtifactDiff,
    pub token_id_layout_changed: bool,
    pub generated_code_changed: bool,
    pub changed_fields: Vec<String>,
    pub reasons: Vec<String>,
    pub base_vocab_delta: i64,
    pub vocab_size_delta: i64,
}

/// Complete old/new layout manifest.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct TokenIdDiffManifest {
    pub old: TokenIdLayout,
    pub new: TokenIdLayout,
    pub diff: TokenIdLayoutDiff,
    pub vocabulary_extension_handoff: VocabularyExtensionHandoff,
}

/// Compiler options that can affect token-ID layout manifests.
#[derive(Debug, Clone)]
pub struct TokenIdLayoutCompileOptions<'a> {
    pub optimization_level: OptimizationLevel,
    pub optimization_pass_overrides: OptimizationPassOverrides,
    pub bpe_merges_path: Option<&'a Path>,
    pub max_base_vocab: Option<u32>,
    pub string_encoding: StringEncoding,
}

impl<'a> TokenIdLayoutCompileOptions<'a> {
    pub fn new(optimization_level: OptimizationLevel) -> Self {
        Self {
            optimization_level,
            optimization_pass_overrides: OptimizationPassOverrides::default(),
            bpe_merges_path: None,
            max_base_vocab: None,
            string_encoding: StringEncoding::Utf8,
        }
    }

    pub fn with_bpe_merges_path(mut self, path: Option<&'a Path>) -> Self {
        self.bpe_merges_path = path;
        self
    }

    pub fn with_optimization_pass_overrides(
        mut self,
        overrides: OptimizationPassOverrides,
    ) -> Self {
        self.optimization_pass_overrides = overrides;
        self
    }

    pub fn with_max_base_vocab(mut self, max_base_vocab: Option<u32>) -> Self {
        self.max_base_vocab = max_base_vocab;
        self
    }

    pub fn with_string_encoding(mut self, string_encoding: StringEncoding) -> Self {
        self.string_encoding = string_encoding;
        self
    }
}

/// Compile a schema and summarize its top-level token-ID layout.
pub fn compile_token_id_layout(
    schema: Value,
    root_type_name: &str,
    optimization_level: OptimizationLevel,
    bpe_merges_path: Option<&Path>,
) -> TctResult<TokenIdLayout> {
    compile_token_id_layout_with_options(
        schema,
        root_type_name,
        TokenIdLayoutCompileOptions::new(optimization_level).with_bpe_merges_path(bpe_merges_path),
    )
}

/// Compile a schema and summarize its top-level token-ID layout with explicit options.
pub fn compile_token_id_layout_with_options(
    schema: Value,
    root_type_name: &str,
    options: TokenIdLayoutCompileOptions<'_>,
) -> TctResult<TokenIdLayout> {
    let mut compiler = TctCompiler::new(schema, root_type_name)?
        .with_optimization_level(options.optimization_level);
    compiler = compiler.with_string_encoding(options.string_encoding);
    compiler = apply_optimization_pass_overrides(compiler, options.optimization_pass_overrides)?;
    if let Some(max_base_vocab) = options.max_base_vocab {
        compiler = compiler.with_max_base_vocab(max_base_vocab)?;
    }
    if let Some(path) = options.bpe_merges_path {
        compiler = compiler.with_bpe_merges(path)?;
    }

    let compile_start = Instant::now();
    let compiled = compiler.compile_with_artifacts()?;
    let compile_wall_time_us = duration_micros_u64(
        compile_start.elapsed(),
        "compile_wall_time_us",
        "Compile a smaller schema before producing a token-ID manifest",
    )?;
    let structural_token_map = summarize_structural_token_map(&compiled.artifacts)?;
    let grammar = compiled.grammar;
    let generate_code_start = Instant::now();
    let generated_code = grammar.generate_code()?;
    let generate_code_wall_time_us = duration_micros_u64(
        generate_code_start.elapsed(),
        "generate_code_wall_time_us",
        "Reduce generated-code size before producing a token-ID manifest",
    )?;
    let generated_code_bytes = checked_usize_to_u64(
        generated_code.len(),
        "generated_code_bytes",
        "Reduce generated-code size before producing a token-ID manifest",
    )?;
    let bpe_merges = checked_usize_to_u32(
        grammar
            .bpe_merge_table
            .as_ref()
            .map_or(0, |table| table.merges.len()),
        "bpe_merges",
    )?;
    let bpe_start = bpe_start_token(grammar.base_vocab);
    let pad_token = checked_add(bpe_start, bpe_merges, "pad_token")?;
    let vocab_size = checked_add(pad_token, 1, "vocab_size")?;
    let bpe_artifact = match (options.bpe_merges_path, grammar.bpe_merge_table.as_ref()) {
        (Some(path), Some(table)) => Some(summarize_bpe_artifact_layout(
            path,
            table,
            grammar.base_vocab,
        )?),
        _ => None,
    };
    let bpe_range = if bpe_merges > 0 {
        Some(TokenIdRange {
            start: bpe_start,
            end_exclusive: pad_token,
        })
    } else {
        None
    };

    Ok(TokenIdLayout {
        root_type_name: grammar.root_type_name,
        optimization_level: options.optimization_level.to_string(),
        optimization_pass_overrides: options.optimization_pass_overrides,
        string_encoding: string_encoding_label(&options.string_encoding).to_string(),
        max_base_vocab: options.max_base_vocab,
        base_vocab_size: grammar.base_vocab,
        encoding_base: grammar.encoding_base,
        bpe_start,
        bpe_merges,
        bpe_range,
        bpe_artifact,
        structural_token_map,
        pad_token,
        vocab_size,
        compile_wall_time_us,
        generate_code_wall_time_us,
        generated_code_bytes,
        generated_code_sha256: sha256_hex(generated_code.as_bytes()),
    })
}

/// Build a token-ID diff manifest from two compiled layouts.
pub fn diff_token_id_layouts(old: TokenIdLayout, new: TokenIdLayout) -> TokenIdDiffManifest {
    let mut changed_fields = Vec::new();
    let bpe_artifact_diff = diff_bpe_artifacts(&old, &new);
    let structural_compatibility =
        assess_structural_compatibility(&old.structural_token_map, &new.structural_token_map);
    let structural_promotion_allowed = structural_promotion_allowed(&old, &new, &bpe_artifact_diff);
    push_changed(
        &mut changed_fields,
        "root_type_name",
        old.root_type_name != new.root_type_name,
    );
    push_changed(
        &mut changed_fields,
        "optimization_level",
        old.optimization_level != new.optimization_level,
    );
    push_changed(
        &mut changed_fields,
        "optimization_pass_overrides",
        old.optimization_pass_overrides != new.optimization_pass_overrides,
    );
    push_changed(
        &mut changed_fields,
        "string_encoding",
        old.string_encoding != new.string_encoding,
    );
    push_changed(
        &mut changed_fields,
        "max_base_vocab",
        old.max_base_vocab != new.max_base_vocab,
    );
    push_changed(
        &mut changed_fields,
        "base_vocab_size",
        old.base_vocab_size != new.base_vocab_size,
    );
    push_changed(
        &mut changed_fields,
        "encoding_base",
        old.encoding_base != new.encoding_base,
    );
    push_changed(
        &mut changed_fields,
        "bpe_start",
        old.bpe_start != new.bpe_start,
    );
    push_changed(
        &mut changed_fields,
        "bpe_merges",
        old.bpe_merges != new.bpe_merges,
    );
    push_changed(
        &mut changed_fields,
        "bpe_range",
        old.bpe_range != new.bpe_range,
    );
    push_changed(
        &mut changed_fields,
        "bpe_artifact_presence",
        old.bpe_artifact.is_some() != new.bpe_artifact.is_some(),
    );
    push_changed(
        &mut changed_fields,
        "bpe_artifact_sha256",
        bpe_artifact_diff.file_sha256_changed,
    );
    push_changed(
        &mut changed_fields,
        "bpe_artifact_table_sha256",
        bpe_artifact_diff.table_sha256_changed,
    );
    push_changed(
        &mut changed_fields,
        "bpe_artifact_merge_sequence_sha256",
        bpe_artifact_diff.merge_sequence_sha256_changed,
    );
    push_changed(
        &mut changed_fields,
        "bpe_artifact_schema",
        bpe_artifact_schema(&old) != bpe_artifact_schema(&new),
    );
    push_changed(
        &mut changed_fields,
        "bpe_artifact_base_vocab_size",
        bpe_artifact_base_vocab_size(&old) != bpe_artifact_base_vocab_size(&new),
    );
    push_changed(
        &mut changed_fields,
        "structural_token_map",
        old.structural_token_map != new.structural_token_map,
    );
    push_changed(
        &mut changed_fields,
        "pad_token",
        old.pad_token != new.pad_token,
    );
    push_changed(
        &mut changed_fields,
        "vocab_size",
        old.vocab_size != new.vocab_size,
    );
    push_changed(
        &mut changed_fields,
        "generated_code_sha256",
        old.generated_code_sha256 != new.generated_code_sha256,
    );

    let token_id_layout_changed = old.base_vocab_size != new.base_vocab_size
        || old.encoding_base != new.encoding_base
        || old.bpe_start != new.bpe_start
        || old.bpe_merges != new.bpe_merges
        || old.bpe_range != new.bpe_range
        || old.pad_token != new.pad_token
        || old.vocab_size != new.vocab_size;
    let generated_code_changed = old.generated_code_sha256 != new.generated_code_sha256;

    let compatibility = if token_id_layout_changed {
        TokenIdLayoutCompatibility::TokenLayoutChanged
    } else if generated_code_changed {
        TokenIdLayoutCompatibility::TokenLayoutUnchanged
    } else {
        TokenIdLayoutCompatibility::Unchanged
    };

    let mut reasons = Vec::new();
    match compatibility {
        TokenIdLayoutCompatibility::Unchanged => {
            reasons.push("token layout and generated code hash are unchanged".to_string());
        }
        TokenIdLayoutCompatibility::TokenLayoutUnchanged => {
            reasons.push(
                "top-level token ID ranges are unchanged; semantic compatibility still requires schema-policy review"
                    .to_string(),
            );
        }
        TokenIdLayoutCompatibility::TokenLayoutChanged => {
            reasons.push(
                "one or more top-level token ID ranges changed; downstream embeddings or BPE artifacts may need migration"
                    .to_string(),
            );
        }
    }
    if old.root_type_name != new.root_type_name {
        reasons.push("root type name changed".to_string());
    }
    if old.optimization_level != new.optimization_level {
        reasons.push("optimization level changed".to_string());
    }
    if old.optimization_pass_overrides != new.optimization_pass_overrides {
        reasons.push("optimization pass overrides changed".to_string());
    }
    if old.string_encoding != new.string_encoding {
        reasons.push("string encoding changed".to_string());
    }
    if old.max_base_vocab != new.max_base_vocab {
        reasons.push("max_base_vocab setting changed".to_string());
    }
    if bpe_artifact_diff.file_sha256_changed {
        reasons.push("BPE artifact fingerprint changed".to_string());
    }
    if bpe_artifact_diff.compatibility != BpeArtifactCompatibility::NoArtifact {
        reasons.extend(bpe_artifact_diff.reasons.iter().cloned());
    }
    if structural_compatibility.decision != StructuralCompatibilityDecision::Unchanged {
        reasons.extend(structural_compatibility.reasons.iter().cloned());
    }

    let stable_vocabulary = assess_stable_vocabulary_policy(
        compatibility,
        &structural_compatibility,
        structural_promotion_allowed,
    );
    let diff = TokenIdLayoutDiff {
        compatibility,
        stable_vocabulary,
        structural_compatibility,
        bpe_artifact: bpe_artifact_diff,
        token_id_layout_changed,
        generated_code_changed,
        changed_fields,
        reasons,
        base_vocab_delta: i64::from(new.base_vocab_size) - i64::from(old.base_vocab_size),
        vocab_size_delta: i64::from(new.vocab_size) - i64::from(old.vocab_size),
    };
    let vocabulary_extension_handoff = build_vocabulary_extension_handoff(&old, &new, &diff);

    TokenIdDiffManifest {
        old,
        new,
        diff,
        vocabulary_extension_handoff,
    }
}

fn push_changed(fields: &mut Vec<String>, field: &str, changed: bool) {
    if changed {
        fields.push(field.to_string());
    }
}

fn string_encoding_label(encoding: &StringEncoding) -> &'static str {
    match encoding {
        StringEncoding::Ascii => "ascii",
        StringEncoding::Utf8 => "utf8",
    }
}

fn apply_optimization_pass_overrides(
    mut compiler: TctCompiler,
    overrides: OptimizationPassOverrides,
) -> TctResult<TctCompiler> {
    if overrides.enum_optimization {
        compiler = compiler.with_enum_optimization();
    }
    if overrides.bounded_int_optimization {
        compiler = compiler.with_bounded_int_optimization();
    }
    if overrides.pattern_optimization {
        return Err(TctError::InvalidConfiguration {
            field: "optimization_pass".to_string(),
            reason: "pattern_optimization is reserved but not implemented as a direct optimization pass".to_string(),
            suggestion: "Do not enable pattern_optimization for token-diff manifests. patternProperties validation is schema-driven.".to_string(),
        });
    }
    if overrides.bit_packing {
        compiler = compiler.with_bit_packing_optimization();
    }
    Ok(compiler)
}

fn checked_usize_to_u32(value: usize, field: &str) -> TctResult<u32> {
    u32::try_from(value).map_err(|_| TctError::InvalidConfiguration {
        field: field.to_string(),
        reason: format!("{field} exceeds u32 token ID range"),
        suggestion: "Reduce the generated token layout size before producing a token-ID manifest"
            .to_string(),
    })
}

fn checked_usize_to_u64(value: usize, field: &str, suggestion: &str) -> TctResult<u64> {
    u64::try_from(value).map_err(|_| TctError::InvalidConfiguration {
        field: field.to_string(),
        reason: format!("{field} exceeds u64 report range"),
        suggestion: suggestion.to_string(),
    })
}

fn duration_micros_u64(duration: Duration, field: &str, suggestion: &str) -> TctResult<u64> {
    u64::try_from(duration.as_micros()).map_err(|_| TctError::InvalidConfiguration {
        field: field.to_string(),
        reason: format!("{field} exceeds u64 report range"),
        suggestion: suggestion.to_string(),
    })
}

fn checked_add(left: u32, right: u32, field: &str) -> TctResult<u32> {
    left.checked_add(right)
        .ok_or_else(|| TctError::InvalidConfiguration {
            field: field.to_string(),
            reason: format!("{field} exceeds u32 token ID range"),
            suggestion:
                "Reduce base vocabulary size or BPE merge count before producing a token-ID manifest"
                    .to_string(),
        })
}

fn sha256_hex(bytes: &[u8]) -> String {
    hex_lower(Sha256::digest(bytes).as_ref())
}

fn sha256_json<T: Serialize>(value: &T, field: &str) -> TctResult<String> {
    let bytes = serde_json::to_vec(value).map_err(|source| TctError::InvalidConfiguration {
        field: field.to_string(),
        reason: format!("failed to serialize {field}: {source}"),
        suggestion: "Ensure the BPE merge table can be serialized for provenance".to_string(),
    })?;
    Ok(sha256_hex(&bytes))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bpe::{BpeMerge, BpeMergeTable, BpeMetadata};
    use serde_json::json;
    use std::path::{Path, PathBuf};

    fn write_bpe_merge_file(path: &Path, merges: Vec<BpeMerge>) -> PathBuf {
        let table = BpeMergeTable {
            schema: "test-bpe".to_string(),
            base_vocab_size: 257,
            metadata: BpeMetadata {
                trained_on_instances: 2,
                training_date: "2026-04-27".to_string(),
                max_merges: merges.len(),
                dict_size: None,
            },
            merges,
        };
        std::fs::write(path, serde_json::to_string_pretty(&table).unwrap()).unwrap();
        path.to_path_buf()
    }

    #[test]
    fn additive_schema_can_leave_top_level_token_layout_unchanged() {
        let old = compile_token_id_layout(
            json!({
                "type": "object",
                "properties": {"name": {"type": "string"}},
                "required": ["name"]
            }),
            "Example",
            OptimizationLevel::O3,
            None,
        )
        .unwrap();
        let new = compile_token_id_layout(
            json!({
                "type": "object",
                "properties": {
                    "name": {"type": "string"},
                    "enabled": {"type": "boolean"}
                },
                "required": ["name"]
            }),
            "Example",
            OptimizationLevel::O3,
            None,
        )
        .unwrap();

        let manifest = diff_token_id_layouts(old, new);
        assert!(manifest.old.generated_code_bytes > 0);
        assert!(manifest.new.generated_code_bytes >= manifest.old.generated_code_bytes);
        assert_eq!(
            manifest.diff.compatibility,
            TokenIdLayoutCompatibility::TokenLayoutUnchanged
        );
        assert!(!manifest.diff.token_id_layout_changed);
        assert!(manifest.diff.generated_code_changed);
        assert_eq!(
            manifest.diff.stable_vocabulary.decision,
            StableVocabularyDecision::BreakingChange
        );
        assert_eq!(
            manifest.diff.structural_compatibility.decision,
            StructuralCompatibilityDecision::BreakingChange
        );
        assert_eq!(
            manifest.diff.stable_vocabulary.generated_tokenizer_reuse,
            ArtifactReuseDecision::RegenerateRequired
        );
        assert_eq!(
            manifest
                .diff
                .stable_vocabulary
                .downstream_bpe_or_embedding_reuse,
            ArtifactReuseDecision::RegenerateRequired
        );
    }

    #[test]
    fn unchanged_schema_is_safe_reuse_under_same_configuration() {
        let old = compile_token_id_layout(
            json!({
                "type": "object",
                "properties": {"name": {"type": "string"}},
                "required": ["name"]
            }),
            "Example",
            OptimizationLevel::O3,
            None,
        )
        .unwrap();
        let new = compile_token_id_layout(
            json!({
                "type": "object",
                "properties": {"name": {"type": "string"}},
                "required": ["name"]
            }),
            "Example",
            OptimizationLevel::O3,
            None,
        )
        .unwrap();

        let manifest = diff_token_id_layouts(old, new);
        assert_eq!(
            manifest.diff.compatibility,
            TokenIdLayoutCompatibility::Unchanged
        );
        assert_eq!(
            manifest.diff.stable_vocabulary.decision,
            StableVocabularyDecision::ExactReuse
        );
        assert_eq!(
            manifest.diff.stable_vocabulary.generated_tokenizer_reuse,
            ArtifactReuseDecision::Reusable
        );
        assert_eq!(
            manifest
                .diff
                .stable_vocabulary
                .downstream_bpe_or_embedding_reuse,
            ArtifactReuseDecision::Reusable
        );
    }

    #[test]
    fn enum_append_with_same_top_level_layout_is_additive_compatible() {
        let old = compile_token_id_layout(
            json!({"enum": ["alpha", "beta"]}),
            "Choice",
            OptimizationLevel::O3,
            None,
        )
        .unwrap();
        let new = compile_token_id_layout(
            json!({"enum": ["alpha", "beta", "gamma"]}),
            "Choice",
            OptimizationLevel::O3,
            None,
        )
        .unwrap();

        let manifest = diff_token_id_layouts(old, new);
        assert_eq!(
            manifest.diff.compatibility,
            TokenIdLayoutCompatibility::TokenLayoutUnchanged
        );
        assert_eq!(
            manifest.diff.structural_compatibility.decision,
            StructuralCompatibilityDecision::AdditiveCompatible
        );
        assert_eq!(
            manifest.diff.structural_compatibility.changed_paths,
            vec!["#/enum/2".to_string()]
        );
        assert_eq!(
            manifest.old.structural_token_map.entries[0].generated_order,
            Some(0)
        );
        assert_eq!(
            manifest.old.structural_token_map.entries[0]
                .member_type_path
                .as_deref(),
            Some("Choice")
        );
        assert_eq!(
            manifest.old.structural_token_map.entries[0]
                .encoding_decision
                .as_deref(),
            Some("enum_index")
        );
        assert_eq!(
            manifest.diff.stable_vocabulary.decision,
            StableVocabularyDecision::AdditiveCompatible
        );
        assert_eq!(
            manifest.diff.stable_vocabulary.generated_tokenizer_reuse,
            ArtifactReuseDecision::RegenerateRequired
        );
        assert_eq!(
            manifest
                .diff
                .stable_vocabulary
                .downstream_bpe_or_embedding_reuse,
            ArtifactReuseDecision::ReviewRequired
        );
        assert_eq!(
            manifest.vocabulary_extension_handoff.status,
            VocabularyExtensionStatus::CandidateExtensionArtifact
        );
        assert_eq!(
            manifest
                .vocabulary_extension_handoff
                .downstream_embedding_handoff,
            DownstreamEmbeddingHandoff::InitializeOrReviewRows
        );
        assert_eq!(
            manifest
                .vocabulary_extension_handoff
                .reusable_schema_local_token_range,
            Some(TokenIdRange {
                start: 0,
                end_exclusive: manifest.old.vocab_size,
            })
        );
        assert!(
            manifest
                .vocabulary_extension_handoff
                .appended_token_ranges
                .is_empty()
        );
        assert_eq!(
            manifest.vocabulary_extension_handoff.new_structural_paths,
            vec!["#/enum/2".to_string()]
        );
    }

    #[test]
    fn enum_reorder_with_same_top_level_layout_is_structural_breaking() {
        let old = compile_token_id_layout(
            json!({"enum": ["alpha", "beta", "gamma"]}),
            "Choice",
            OptimizationLevel::O3,
            None,
        )
        .unwrap();
        let new = compile_token_id_layout(
            json!({"enum": ["gamma", "beta", "alpha"]}),
            "Choice",
            OptimizationLevel::O3,
            None,
        )
        .unwrap();

        let manifest = diff_token_id_layouts(old, new);
        assert_eq!(
            manifest.diff.compatibility,
            TokenIdLayoutCompatibility::TokenLayoutUnchanged
        );
        assert_eq!(
            manifest.diff.structural_compatibility.decision,
            StructuralCompatibilityDecision::BreakingChange
        );
        assert_eq!(
            manifest.diff.stable_vocabulary.decision,
            StableVocabularyDecision::BreakingChange
        );
        assert_ne!(
            manifest.diff.stable_vocabulary.decision,
            StableVocabularyDecision::AdditiveCompatible
        );
    }

    #[test]
    fn required_object_property_append_with_same_layout_is_structural_breaking() {
        let old = compile_token_id_layout(
            json!({
                "type": "object",
                "properties": {"name": {"type": "string"}},
                "required": ["name"]
            }),
            "Example",
            OptimizationLevel::O3,
            None,
        )
        .unwrap();
        let new = compile_token_id_layout(
            json!({
                "type": "object",
                "properties": {
                    "name": {"type": "string"},
                    "enabled": {"type": "boolean"}
                },
                "required": ["name", "enabled"]
            }),
            "Example",
            OptimizationLevel::O3,
            None,
        )
        .unwrap();

        let manifest = diff_token_id_layouts(old, new);
        assert_eq!(
            manifest.diff.compatibility,
            TokenIdLayoutCompatibility::TokenLayoutUnchanged
        );
        assert_eq!(
            manifest.diff.structural_compatibility.decision,
            StructuralCompatibilityDecision::BreakingChange
        );
        assert_eq!(
            manifest.diff.stable_vocabulary.decision,
            StableVocabularyDecision::BreakingChange
        );
        assert!(
            manifest
                .diff
                .structural_compatibility
                .changed_paths
                .contains(&"#/properties/enabled".to_string())
        );
    }

    #[test]
    fn required_zero_token_const_append_is_additive_compatible() {
        let old = compile_token_id_layout(
            json!({
                "type": "object",
                "properties": {"name": {"type": "string"}},
                "required": ["name"],
                "additionalProperties": false
            }),
            "Example",
            OptimizationLevel::O3,
            None,
        )
        .unwrap();
        let new = compile_token_id_layout(
            json!({
                "type": "object",
                "properties": {
                    "name": {"type": "string"},
                    "kind": {"type": "string", "const": "Example"}
                },
                "required": ["name", "kind"],
                "additionalProperties": false
            }),
            "Example",
            OptimizationLevel::O3,
            None,
        )
        .unwrap();

        let manifest = diff_token_id_layouts(old, new);
        assert_eq!(
            manifest.diff.compatibility,
            TokenIdLayoutCompatibility::TokenLayoutUnchanged
        );
        assert_eq!(
            manifest.diff.structural_compatibility.decision,
            StructuralCompatibilityDecision::AdditiveCompatible
        );
        assert_eq!(
            manifest.diff.structural_compatibility.changed_paths,
            vec!["#/properties/kind".to_string()]
        );
        let kind_entry = manifest
            .new
            .structural_token_map
            .entries
            .iter()
            .find(|entry| entry.path == "#/properties/kind")
            .expect("required const addition should be reported");
        assert_eq!(kind_entry.role, StructuralTokenRole::ZeroTokenConst);
        assert_eq!(kind_entry.required, Some(true));
        assert_eq!(kind_entry.generated_order, Some(1));
        assert_eq!(
            kind_entry.emission.strict_stream_shape,
            StructuralStrictStreamShape::ZeroTokens
        );
        assert_eq!(
            kind_entry.emission.value,
            StructuralValueEmission {
                emits_when_present: false,
                width: StructuralValueWidth::Zero,
                fixed_token_count: Some(0),
            }
        );
        assert_eq!(
            manifest.diff.stable_vocabulary.decision,
            StableVocabularyDecision::AdditiveCompatible
        );
        assert_eq!(
            manifest.diff.stable_vocabulary.generated_tokenizer_reuse,
            ArtifactReuseDecision::RegenerateRequired
        );
        assert_eq!(
            manifest
                .diff
                .stable_vocabulary
                .downstream_bpe_or_embedding_reuse,
            ArtifactReuseDecision::ReviewRequired
        );
    }

    #[test]
    fn o0_string_const_append_is_not_promoted_as_zero_token() {
        let old = compile_token_id_layout(
            json!({
                "type": "object",
                "properties": {"name": {"type": "string"}},
                "required": ["name"],
                "additionalProperties": false
            }),
            "Example",
            OptimizationLevel::O0,
            None,
        )
        .unwrap();
        let new = compile_token_id_layout(
            json!({
                "type": "object",
                "properties": {
                    "name": {"type": "string"},
                    "kind": {"type": "string", "const": "Example"}
                },
                "required": ["name", "kind"],
                "additionalProperties": false
            }),
            "Example",
            OptimizationLevel::O0,
            None,
        )
        .unwrap();

        let manifest = diff_token_id_layouts(old, new);
        let kind_entry = manifest
            .new
            .structural_token_map
            .entries
            .iter()
            .find(|entry| entry.path == "#/properties/kind")
            .expect("required string const addition should be reported");
        assert_eq!(kind_entry.encoding_decision.as_deref(), Some("standard"));
        assert_eq!(kind_entry.role, StructuralTokenRole::RequiredValue);
        assert_eq!(
            manifest.diff.structural_compatibility.decision,
            StructuralCompatibilityDecision::BreakingChange
        );
        assert_eq!(
            manifest.diff.stable_vocabulary.decision,
            StableVocabularyDecision::BreakingChange
        );
    }

    #[test]
    fn non_string_const_append_is_zero_token_even_without_o3_enum_optimization() {
        let old = compile_token_id_layout(
            json!({
                "type": "object",
                "properties": {"name": {"type": "string"}},
                "required": ["name"],
                "additionalProperties": false
            }),
            "Example",
            OptimizationLevel::O0,
            None,
        )
        .unwrap();
        let new = compile_token_id_layout(
            json!({
                "type": "object",
                "properties": {
                    "name": {"type": "string"},
                    "enabled": {"type": "boolean", "const": true}
                },
                "required": ["name", "enabled"],
                "additionalProperties": false
            }),
            "Example",
            OptimizationLevel::O0,
            None,
        )
        .unwrap();

        let manifest = diff_token_id_layouts(old, new);
        let enabled_entry = manifest
            .new
            .structural_token_map
            .entries
            .iter()
            .find(|entry| entry.path == "#/properties/enabled")
            .expect("required boolean const addition should be reported");
        assert_eq!(enabled_entry.role, StructuralTokenRole::ZeroTokenConst);
        assert_eq!(enabled_entry.encoding_decision.as_deref(), Some("standard"));
        assert_eq!(
            manifest.diff.structural_compatibility.decision,
            StructuralCompatibilityDecision::AdditiveCompatible
        );
        assert_eq!(
            manifest.diff.stable_vocabulary.decision,
            StableVocabularyDecision::AdditiveCompatible
        );
    }

    #[test]
    fn optional_zero_token_const_append_is_not_promoted() {
        let old = compile_token_id_layout(
            json!({
                "type": "object",
                "properties": {"name": {"type": "string"}},
                "required": ["name"],
                "additionalProperties": false
            }),
            "Example",
            OptimizationLevel::O3,
            None,
        )
        .unwrap();
        let new = compile_token_id_layout(
            json!({
                "type": "object",
                "properties": {
                    "name": {"type": "string"},
                    "kind": {"type": "string", "const": "Example"}
                },
                "required": ["name"],
                "additionalProperties": false
            }),
            "Example",
            OptimizationLevel::O3,
            None,
        )
        .unwrap();

        let manifest = diff_token_id_layouts(old, new);
        assert_eq!(
            manifest.diff.compatibility,
            TokenIdLayoutCompatibility::TokenLayoutUnchanged
        );
        assert_eq!(
            manifest.diff.structural_compatibility.decision,
            StructuralCompatibilityDecision::NotProven
        );
        let kind_entry = manifest
            .new
            .structural_token_map
            .entries
            .iter()
            .find(|entry| entry.path == "#/properties/kind")
            .expect("optional const addition should be reported");
        assert_eq!(kind_entry.role, StructuralTokenRole::OptionPresenceAndValue);
        assert_eq!(kind_entry.required, Some(false));
        assert_eq!(
            manifest.diff.stable_vocabulary.decision,
            StableVocabularyDecision::LayoutOnlyUnchanged
        );
    }

    #[test]
    fn optional_bitpacked_property_append_is_not_promoted_without_migration_contract() {
        let overrides = OptimizationPassOverrides::default().with_bit_packing();
        let old = compile_token_id_layout_with_options(
            json!({
                "type": "object",
                "properties": {
                    "name": {"type": "string"},
                    "optional0": {"type": "boolean"},
                    "optional1": {"type": "boolean"},
                    "optional2": {"type": "boolean"},
                    "optional3": {"type": "boolean"}
                },
                "required": ["name"],
                "additionalProperties": false
            }),
            "Example",
            TokenIdLayoutCompileOptions::new(OptimizationLevel::O3)
                .with_optimization_pass_overrides(overrides),
        )
        .unwrap();
        let new = compile_token_id_layout_with_options(
            json!({
                "type": "object",
                "properties": {
                    "name": {"type": "string"},
                    "optional0": {"type": "boolean"},
                    "optional1": {"type": "boolean"},
                    "optional2": {"type": "boolean"},
                    "optional3": {"type": "boolean"},
                    "enabled": {"type": "boolean"}
                },
                "required": ["name"],
                "additionalProperties": false
            }),
            "Example",
            TokenIdLayoutCompileOptions::new(OptimizationLevel::O3)
                .with_optimization_pass_overrides(overrides),
        )
        .unwrap();

        let manifest = diff_token_id_layouts(old, new);
        let enabled_entry = manifest
            .new
            .structural_token_map
            .entries
            .iter()
            .find(|entry| entry.path == "#/properties/enabled")
            .expect("optional property addition should be reported");
        assert!(manifest.new.optimization_pass_overrides.bit_packing);
        assert_eq!(
            enabled_entry.role,
            StructuralTokenRole::OptionPresenceAndValue
        );
        assert_eq!(
            enabled_entry.emission.selector.kind,
            StructuralSelectorKind::BitPackedPresenceBit
        );
        assert_eq!(
            enabled_entry.emission.selector.bit_packed_presence,
            Some(BitPackedPresenceMetadata {
                optional_field_index: 4,
                presence_mask_index: 0,
                bit_index: 4,
                presence_mask_count: 1,
            })
        );
        assert_eq!(
            enabled_entry.emission.strict_stream_shape,
            StructuralStrictStreamShape::SharedPresenceMaskThenValueIfPresent
        );
        assert_eq!(
            enabled_entry.emission.value,
            StructuralValueEmission {
                emits_when_present: true,
                width: StructuralValueWidth::Fixed,
                fixed_token_count: Some(1),
            }
        );
        assert_eq!(
            manifest.diff.structural_compatibility.decision,
            StructuralCompatibilityDecision::NotProven
        );
        assert_eq!(
            manifest.diff.stable_vocabulary.decision,
            StableVocabularyDecision::LayoutOnlyUnchanged
        );
    }

    #[test]
    fn optional_value_property_append_is_not_promoted_without_migration_contract() {
        let old = compile_token_id_layout(
            json!({
                "type": "object",
                "properties": {"name": {"type": "string"}},
                "required": ["name"],
                "additionalProperties": false
            }),
            "Example",
            OptimizationLevel::O3,
            None,
        )
        .unwrap();
        let new = compile_token_id_layout(
            json!({
                "type": "object",
                "properties": {
                    "name": {"type": "string"},
                    "metadata": {}
                },
                "required": ["name"],
                "additionalProperties": false
            }),
            "Example",
            OptimizationLevel::O3,
            None,
        )
        .unwrap();

        let manifest = diff_token_id_layouts(old, new);
        let metadata_entry = manifest
            .new
            .structural_token_map
            .entries
            .iter()
            .find(|entry| entry.path == "#/properties/metadata")
            .expect("optional Value property addition should be reported");
        assert_eq!(
            metadata_entry.role,
            StructuralTokenRole::OptionalFieldStateAndValue
        );
        assert_eq!(
            metadata_entry.emission.selector.kind,
            StructuralSelectorKind::OptionalFieldStateToken
        );
        assert_eq!(
            metadata_entry.emission.strict_stream_shape,
            StructuralStrictStreamShape::OptionalStateThenValueIfPresent
        );
        assert_eq!(
            metadata_entry.emission.value.width,
            StructuralValueWidth::Variable
        );
        assert_eq!(
            manifest.diff.structural_compatibility.decision,
            StructuralCompatibilityDecision::NotProven
        );
        assert_ne!(
            manifest.diff.stable_vocabulary.decision,
            StableVocabularyDecision::AdditiveCompatible
        );
    }

    #[test]
    fn catch_all_map_addition_is_not_promoted_without_migration_contract() {
        let old = compile_token_id_layout(
            json!({
                "type": "object",
                "properties": {"name": {"type": "string"}},
                "required": ["name"],
                "additionalProperties": false
            }),
            "Example",
            OptimizationLevel::O3,
            None,
        )
        .unwrap();
        let new = compile_token_id_layout(
            json!({
                "type": "object",
                "properties": {"name": {"type": "string"}},
                "required": ["name"],
                "additionalProperties": {"type": "string"}
            }),
            "Example",
            OptimizationLevel::O3,
            None,
        )
        .unwrap();

        let manifest = diff_token_id_layouts(old, new);
        let catch_all_entry = manifest
            .new
            .structural_token_map
            .entries
            .iter()
            .find(|entry| entry.emission.catch_all_map)
            .expect("catch-all map addition should be reported");
        assert_eq!(catch_all_entry.path, "#/properties/additional_properties");
        assert_eq!(
            catch_all_entry.emission.value.width,
            StructuralValueWidth::Variable
        );
        assert_ne!(
            manifest.diff.structural_compatibility.decision,
            StructuralCompatibilityDecision::AdditiveCompatible
        );
        assert_ne!(
            manifest.diff.stable_vocabulary.decision,
            StableVocabularyDecision::AdditiveCompatible
        );
    }

    #[test]
    fn nested_object_addition_reports_emission_metadata() {
        let old = compile_token_id_layout(
            json!({
                "type": "object",
                "properties": {"name": {"type": "string"}},
                "required": ["name"],
                "additionalProperties": false
            }),
            "Example",
            OptimizationLevel::O3,
            None,
        )
        .unwrap();
        let new = compile_token_id_layout(
            json!({
                "type": "object",
                "properties": {
                    "name": {"type": "string"},
                    "child": {
                        "type": "object",
                        "properties": {"id": {"type": "string"}},
                        "required": ["id"],
                        "additionalProperties": false
                    }
                },
                "required": ["name"],
                "additionalProperties": false
            }),
            "Example",
            OptimizationLevel::O3,
            None,
        )
        .unwrap();

        let manifest = diff_token_id_layouts(old, new);
        let child_entry = manifest
            .new
            .structural_token_map
            .entries
            .iter()
            .find(|entry| entry.path == "#/properties/child")
            .expect("nested object addition should be reported");
        assert!(child_entry.emission.nested_object);
        assert_eq!(
            child_entry.emission.value.width,
            StructuralValueWidth::SchemaDependent
        );
        assert_eq!(
            child_entry.emission.strict_stream_shape,
            StructuralStrictStreamShape::PresenceFlagThenValueIfPresent
        );
        assert_eq!(
            manifest.diff.structural_compatibility.decision,
            StructuralCompatibilityDecision::NotProven
        );
    }

    #[test]
    fn object_schema_metadata_change_without_new_entries_is_not_promoted() {
        let old = compile_token_id_layout(
            json!({
                "type": "object",
                "properties": {"name": {"type": "string"}},
                "required": ["name"],
                "additionalProperties": false
            }),
            "Example",
            OptimizationLevel::O3,
            None,
        )
        .unwrap();
        let new = compile_token_id_layout(
            json!({
                "type": "object",
                "description": "metadata-only change",
                "properties": {"name": {"type": "string"}},
                "required": ["name"],
                "additionalProperties": false
            }),
            "Example",
            OptimizationLevel::O3,
            None,
        )
        .unwrap();

        let manifest = diff_token_id_layouts(old, new);
        assert_eq!(
            manifest.diff.structural_compatibility.decision,
            StructuralCompatibilityDecision::NotProven
        );
        assert_eq!(
            manifest.diff.stable_vocabulary.decision,
            StableVocabularyDecision::ExactReuse
        );
    }

    #[test]
    fn structural_map_uses_pass_owned_discovered_members() {
        let old = compile_token_id_layout(
            json!({
                "type": "object",
                "properties": {"name": {"type": "string"}},
                "required": ["name"]
            }),
            "Example",
            OptimizationLevel::O3,
            None,
        )
        .unwrap();
        let new = compile_token_id_layout(
            json!({
                "type": "object",
                "properties": {"name": {"type": "string"}},
                "required": ["name"],
                "allOf": [
                    {
                        "type": "object",
                        "properties": {"enabled": {"type": "boolean"}}
                    }
                ]
            }),
            "Example",
            OptimizationLevel::O3,
            None,
        )
        .unwrap();

        let manifest = diff_token_id_layouts(old, new);
        assert_eq!(
            manifest.diff.structural_compatibility.decision,
            StructuralCompatibilityDecision::BreakingChange
        );
        assert!(
            manifest
                .diff
                .structural_compatibility
                .changed_paths
                .contains(&"#/properties/enabled".to_string())
        );
        let enabled_entry = manifest
            .new
            .structural_token_map
            .entries
            .iter()
            .find(|entry| entry.path == "#/properties/enabled")
            .expect("pass-owned discovery should include allOf-added property");
        assert_eq!(
            enabled_entry.role,
            StructuralTokenRole::OptionPresenceAndValue
        );
        assert!(enabled_entry.generated_order.is_some());
        assert_eq!(enabled_entry.member_type_path.as_deref(), Some("bool"));
        assert_eq!(enabled_entry.encoding_decision.as_deref(), Some("standard"));
    }

    #[test]
    fn manifest_reports_top_level_token_layout_changes() {
        let old_values = (0..260)
            .map(|value| format!("v{value}"))
            .collect::<Vec<_>>();
        let new_values = (0..300)
            .map(|value| format!("v{value}"))
            .collect::<Vec<_>>();

        let old = compile_token_id_layout(
            json!({"enum": old_values}),
            "LargeEnum",
            OptimizationLevel::O0,
            None,
        )
        .unwrap();
        let new = compile_token_id_layout(
            json!({"enum": new_values}),
            "LargeEnum",
            OptimizationLevel::O0,
            None,
        )
        .unwrap();

        let manifest = diff_token_id_layouts(old, new);
        assert_eq!(
            manifest.diff.compatibility,
            TokenIdLayoutCompatibility::TokenLayoutChanged
        );
        assert!(manifest.diff.token_id_layout_changed);
        assert_eq!(
            manifest.diff.stable_vocabulary.decision,
            StableVocabularyDecision::BreakingChange
        );
        assert_eq!(
            manifest
                .diff
                .stable_vocabulary
                .downstream_bpe_or_embedding_reuse,
            ArtifactReuseDecision::RegenerateRequired
        );
        assert!(
            manifest
                .diff
                .changed_fields
                .contains(&"base_vocab_size".to_string())
        );
        assert!(manifest.diff.base_vocab_delta > 0);
    }

    #[test]
    fn manifest_reports_string_encoding_configuration_changes() {
        let old = compile_token_id_layout(
            json!({"type": "string"}),
            "Text",
            OptimizationLevel::O3,
            None,
        )
        .unwrap();
        let new = compile_token_id_layout_with_options(
            json!({"type": "string"}),
            "Text",
            TokenIdLayoutCompileOptions::new(OptimizationLevel::O3)
                .with_string_encoding(StringEncoding::Ascii),
        )
        .unwrap();

        let manifest = diff_token_id_layouts(old, new);
        assert_eq!(manifest.old.string_encoding, "utf8");
        assert_eq!(manifest.new.string_encoding, "ascii");
        assert!(
            manifest
                .diff
                .changed_fields
                .contains(&"string_encoding".to_string())
        );
        assert!(
            manifest
                .diff
                .changed_fields
                .contains(&"base_vocab_size".to_string())
        );
        assert_eq!(
            manifest.diff.stable_vocabulary.decision,
            StableVocabularyDecision::BreakingChange
        );
    }

    #[test]
    fn manifest_reports_max_base_vocab_configuration_changes() {
        let old = compile_token_id_layout(
            json!({"type": "integer", "minimum": 0, "maximum": 1000}),
            "BoundedValue",
            OptimizationLevel::O3,
            None,
        )
        .unwrap();
        let new = compile_token_id_layout_with_options(
            json!({"type": "integer", "minimum": 0, "maximum": 1000}),
            "BoundedValue",
            TokenIdLayoutCompileOptions::new(OptimizationLevel::O3).with_max_base_vocab(Some(512)),
        )
        .unwrap();

        let manifest = diff_token_id_layouts(old, new);
        assert_eq!(manifest.old.max_base_vocab, None);
        assert_eq!(manifest.new.max_base_vocab, Some(512));
        assert_eq!(manifest.new.encoding_base, 512);
        assert!(
            manifest
                .diff
                .changed_fields
                .contains(&"max_base_vocab".to_string())
        );
        assert!(
            manifest
                .diff
                .changed_fields
                .contains(&"encoding_base".to_string())
        );
        assert_eq!(
            manifest.diff.stable_vocabulary.decision,
            StableVocabularyDecision::BreakingChange
        );
    }

    #[test]
    fn manifest_reports_optimization_pass_override_changes() {
        let schema = json!({"enum": ["alpha", "beta", "gamma"]});
        let old =
            compile_token_id_layout(schema.clone(), "Choice", OptimizationLevel::O0, None).unwrap();
        let new = compile_token_id_layout_with_options(
            schema,
            "Choice",
            TokenIdLayoutCompileOptions::new(OptimizationLevel::O0)
                .with_optimization_pass_overrides(
                    OptimizationPassOverrides::default().with_enum_optimization(),
                ),
        )
        .unwrap();

        let manifest = diff_token_id_layouts(old, new);
        assert!(!manifest.old.optimization_pass_overrides.enum_optimization);
        assert!(manifest.new.optimization_pass_overrides.enum_optimization);
        assert!(
            manifest
                .diff
                .changed_fields
                .contains(&"optimization_pass_overrides".to_string())
        );
        assert!(
            manifest
                .diff
                .reasons
                .contains(&"optimization pass overrides changed".to_string())
        );
    }

    #[test]
    fn pattern_optimization_override_is_reserved_until_implemented() {
        let result = compile_token_id_layout_with_options(
            json!({"type": "string", "pattern": "^[a-z]{3}$"}),
            "PatternText",
            TokenIdLayoutCompileOptions::new(OptimizationLevel::O3)
                .with_optimization_pass_overrides(
                    OptimizationPassOverrides::default().with_pattern_optimization(),
                ),
        );

        match result {
            Err(TctError::InvalidConfiguration {
                field,
                reason,
                suggestion,
            }) => {
                assert_eq!(field, "optimization_pass");
                assert!(reason.contains("pattern_optimization is reserved"));
                assert!(suggestion.contains("patternProperties validation is schema-driven"));
            }
            Ok(_) => panic!("expected reserved pattern optimization override error, got success"),
            Err(error) => {
                panic!("expected reserved pattern optimization override error, got {error}")
            }
        }
    }

    #[test]
    fn bpe_artifact_summary_reports_stale_base_vocab_evidence() {
        let dir = tempfile::TempDir::new().unwrap();
        let bpe_path = write_bpe_merge_file(
            &dir.path().join("stale-bpe.json"),
            vec![BpeMerge {
                pair: [1, 2],
                new_token: 257,
                frequency: 10,
            }],
        );
        let table = BpeMergeTable::from_file(&bpe_path).unwrap();

        let artifact = summarize_bpe_artifact_layout(&bpe_path, &table, 512).unwrap();
        assert_eq!(artifact.base_vocab_size, 257);
        assert_eq!(artifact.bpe_start, 257);
        assert_eq!(artifact.first_new_token, Some(257));
        assert_eq!(artifact.last_new_token, Some(257));
        assert!(!artifact.matches_compiled_base_vocab);
        assert!(!artifact.matches_compiled_bpe_start);
        assert!(artifact.metadata_merge_count_matches_table);
        assert_eq!(artifact.metadata_dict_size, None);
        assert_eq!(
            artifact.bpe_range,
            Some(TokenIdRange {
                start: 257,
                end_exclusive: 258,
            })
        );
    }

    #[test]
    fn manifest_reports_bpe_artifact_prefix_extension_evidence() {
        let dir = tempfile::TempDir::new().unwrap();
        let old_bpe = write_bpe_merge_file(
            &dir.path().join("old-bpe.json"),
            vec![BpeMerge {
                pair: [1, 2],
                new_token: 257,
                frequency: 10,
            }],
        );
        let new_bpe = write_bpe_merge_file(
            &dir.path().join("new-bpe.json"),
            vec![
                BpeMerge {
                    pair: [1, 2],
                    new_token: 257,
                    frequency: 10,
                },
                BpeMerge {
                    pair: [257, 3],
                    new_token: 258,
                    frequency: 8,
                },
            ],
        );

        let old = compile_token_id_layout(
            json!({"type": "string"}),
            "Text",
            OptimizationLevel::O3,
            Some(&old_bpe),
        )
        .unwrap();
        let new = compile_token_id_layout(
            json!({"type": "string"}),
            "Text",
            OptimizationLevel::O3,
            Some(&new_bpe),
        )
        .unwrap();

        let manifest = diff_token_id_layouts(old, new);
        let old_artifact = manifest.old.bpe_artifact.as_ref().unwrap();
        let new_artifact = manifest.new.bpe_artifact.as_ref().unwrap();
        assert_eq!(old_artifact.base_vocab_size, 257);
        assert_eq!(old_artifact.first_new_token, Some(257));
        assert_eq!(old_artifact.last_new_token, Some(257));
        assert_eq!(new_artifact.last_new_token, Some(258));
        assert!(old_artifact.matches_compiled_base_vocab);
        assert!(new_artifact.matches_compiled_bpe_start);
        assert!(old_artifact.metadata_merge_count_matches_table);
        assert_ne!(old_artifact.file.sha256, new_artifact.file.sha256);
        assert_ne!(
            old_artifact.merge_sequence_sha256,
            new_artifact.merge_sequence_sha256
        );
        assert_eq!(
            manifest.diff.bpe_artifact.compatibility,
            BpeArtifactCompatibility::PrefixExtended
        );
        assert_eq!(manifest.diff.bpe_artifact.old_is_prefix_of_new, Some(true));
        assert_eq!(
            manifest.diff.bpe_artifact.first_differing_merge_index,
            Some(1)
        );
        assert_eq!(manifest.diff.bpe_artifact.merge_count_delta, 1);
        assert_eq!(
            manifest.diff.compatibility,
            TokenIdLayoutCompatibility::TokenLayoutChanged
        );
        assert_eq!(
            manifest.diff.stable_vocabulary.decision,
            StableVocabularyDecision::BreakingChange
        );
        assert_eq!(
            manifest.vocabulary_extension_handoff.status,
            VocabularyExtensionStatus::CandidateExtensionArtifact
        );
        assert_eq!(
            manifest.vocabulary_extension_handoff.appended_token_ranges,
            vec![TokenIdRange {
                start: manifest.old.vocab_size,
                end_exclusive: manifest.new.vocab_size,
            }]
        );
        assert_eq!(
            manifest.vocabulary_extension_handoff.bpe_prefix_extension,
            Some(true)
        );
        assert_eq!(
            manifest.vocabulary_extension_handoff.old_bpe_table_sha256,
            Some(old_artifact.table_sha256.clone())
        );
        assert_eq!(
            manifest
                .vocabulary_extension_handoff
                .new_bpe_merge_sequence_sha256,
            Some(new_artifact.merge_sequence_sha256.clone())
        );
    }

    #[test]
    fn manifest_reports_same_size_bpe_artifact_semantic_changes() {
        let dir = tempfile::TempDir::new().unwrap();
        let old_bpe = write_bpe_merge_file(
            &dir.path().join("old-bpe.json"),
            vec![BpeMerge {
                pair: [1, 2],
                new_token: 257,
                frequency: 10,
            }],
        );
        let new_bpe = write_bpe_merge_file(
            &dir.path().join("new-bpe.json"),
            vec![BpeMerge {
                pair: [1, 3],
                new_token: 257,
                frequency: 9,
            }],
        );

        let old = compile_token_id_layout(
            json!({"type": "string"}),
            "Text",
            OptimizationLevel::O3,
            Some(&old_bpe),
        )
        .unwrap();
        let new = compile_token_id_layout(
            json!({"type": "string"}),
            "Text",
            OptimizationLevel::O3,
            Some(&new_bpe),
        )
        .unwrap();

        let manifest = diff_token_id_layouts(old, new);
        assert_eq!(
            manifest.diff.compatibility,
            TokenIdLayoutCompatibility::TokenLayoutUnchanged
        );
        assert_eq!(
            manifest.diff.bpe_artifact.compatibility,
            BpeArtifactCompatibility::Changed
        );
        assert_eq!(
            manifest.diff.bpe_artifact.first_differing_merge_index,
            Some(0)
        );
        assert_eq!(manifest.diff.bpe_artifact.old_is_prefix_of_new, Some(false));
        assert_eq!(manifest.diff.bpe_artifact.merge_count_delta, 0);
        assert!(manifest.diff.bpe_artifact.merge_sequence_sha256_changed);
        assert_eq!(
            manifest.vocabulary_extension_handoff.status,
            VocabularyExtensionStatus::ReviewOnly
        );
        assert!(
            manifest
                .vocabulary_extension_handoff
                .appended_token_ranges
                .is_empty()
        );
        assert_eq!(
            manifest.vocabulary_extension_handoff.bpe_prefix_extension,
            Some(false)
        );
    }
}
