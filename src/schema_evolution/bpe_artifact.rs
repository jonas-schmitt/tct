//! BPE artifact comparison records for schema-evolution reports.

use super::{TokenIdLayout, TokenIdRange, checked_add, checked_usize_to_u32, sha256_json};
use crate::bpe::{BpeMerge, BpeMergeTable, bpe_start_token};
use crate::error::{TctError, TctResult};
use crate::provenance::{FileProvenance, file_provenance};
use serde::Serialize;
use std::path::Path;

/// BPE merge-table provenance and compatibility evidence for one compiled layout.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct BpeArtifactLayout {
    pub file: FileProvenance,
    pub schema: String,
    pub base_vocab_size: u32,
    pub bpe_start: u32,
    pub merges: u32,
    pub first_new_token: Option<u32>,
    pub last_new_token: Option<u32>,
    pub bpe_range: Option<TokenIdRange>,
    pub table_sha256: String,
    pub merge_sequence_sha256: String,
    pub metadata_trained_on_instances: usize,
    pub metadata_training_date: String,
    pub metadata_max_merges: usize,
    pub metadata_dict_size: Option<usize>,
    pub matches_compiled_base_vocab: bool,
    pub matches_compiled_bpe_start: bool,
    pub metadata_merge_count_matches_table: bool,
    #[serde(skip)]
    merge_evidence: Vec<BpeMerge>,
}

/// BPE artifact comparison evidence for an old/new schema transition.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct BpeArtifactDiff {
    pub compatibility: BpeArtifactCompatibility,
    pub old_present: bool,
    pub new_present: bool,
    pub file_sha256_changed: bool,
    pub table_sha256_changed: bool,
    pub merge_sequence_sha256_changed: bool,
    pub base_vocab_size_changed: bool,
    pub bpe_start_changed: bool,
    pub merge_count_delta: i64,
    pub old_is_prefix_of_new: Option<bool>,
    pub first_differing_merge_index: Option<usize>,
    pub reasons: Vec<String>,
}

/// Conservative BPE artifact compatibility classification.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum BpeArtifactCompatibility {
    /// Neither side uses a BPE artifact.
    NoArtifact,
    /// Old and new BPE tables are semantically identical.
    Unchanged,
    /// A new transition adds BPE where the old one had none.
    Added,
    /// A new transition removes BPE where the old one had BPE.
    Removed,
    /// The new table preserves the old table as an exact prefix and appends merges.
    PrefixExtended,
    /// The artifact changed in a way that needs retraining or explicit review.
    Changed,
}

/// Summarize a BPE artifact against a compiled base vocabulary.
///
/// This is used by complete manifests and by partial token-diff reports for
/// stale BPE files that fail compiler base-vocabulary validation.
pub fn summarize_bpe_artifact_layout(
    path: &Path,
    table: &BpeMergeTable,
    compiled_base_vocab: u32,
) -> TctResult<BpeArtifactLayout> {
    let file = file_provenance(path).map_err(|source| TctError::Io {
        operation: "compute BPE merge-table provenance".to_string(),
        path: path.display().to_string(),
        source,
        suggestion: "Ensure the BPE merge file is readable while producing the token-ID manifest"
            .to_string(),
    })?;
    let merges = checked_usize_to_u32(table.merges.len(), "bpe_artifact_merges")?;
    let bpe_start = table.bpe_start();
    let bpe_range = if merges > 0 {
        Some(TokenIdRange {
            start: bpe_start,
            end_exclusive: checked_add(bpe_start, merges, "bpe_artifact_range")?,
        })
    } else {
        None
    };

    Ok(BpeArtifactLayout {
        file,
        schema: table.schema.clone(),
        base_vocab_size: table.base_vocab_size,
        bpe_start,
        merges,
        first_new_token: table.merges.first().map(|merge| merge.new_token),
        last_new_token: table.merges.last().map(|merge| merge.new_token),
        bpe_range,
        table_sha256: sha256_json(table, "bpe_artifact_table_sha256")?,
        merge_sequence_sha256: sha256_json(&table.merges, "bpe_artifact_merge_sequence_sha256")?,
        metadata_trained_on_instances: table.metadata.trained_on_instances,
        metadata_training_date: table.metadata.training_date.clone(),
        metadata_max_merges: table.metadata.max_merges,
        metadata_dict_size: table.metadata.dict_size,
        matches_compiled_base_vocab: table.base_vocab_size == compiled_base_vocab,
        matches_compiled_bpe_start: bpe_start == bpe_start_token(compiled_base_vocab),
        metadata_merge_count_matches_table: table.metadata.max_merges == table.merges.len(),
        merge_evidence: table.merges.clone(),
    })
}

pub(super) fn diff_bpe_artifacts(old: &TokenIdLayout, new: &TokenIdLayout) -> BpeArtifactDiff {
    let old_artifact = old.bpe_artifact.as_ref();
    let new_artifact = new.bpe_artifact.as_ref();
    let old_present = old_artifact.is_some();
    let new_present = new_artifact.is_some();
    let file_sha256_changed = bpe_artifact_sha256(old) != bpe_artifact_sha256(new);
    let table_sha256_changed = bpe_artifact_table_sha256(old) != bpe_artifact_table_sha256(new);
    let merge_sequence_sha256_changed =
        bpe_artifact_merge_sequence_sha256(old) != bpe_artifact_merge_sequence_sha256(new);
    let base_vocab_size_changed =
        bpe_artifact_base_vocab_size(old) != bpe_artifact_base_vocab_size(new);
    let bpe_start_changed = bpe_artifact_bpe_start(old) != bpe_artifact_bpe_start(new);
    let merge_count_delta = i64::from(new.bpe_merges) - i64::from(old.bpe_merges);

    let (old_is_prefix_of_new, first_differing_merge_index) =
        if let (Some(old), Some(new)) = (old_artifact, new_artifact) {
            (
                Some(is_merge_prefix(&old.merge_evidence, &new.merge_evidence)),
                first_differing_merge_index(&old.merge_evidence, &new.merge_evidence),
            )
        } else {
            (None, None)
        };

    let mut reasons = Vec::new();
    let compatibility = match (old_artifact, new_artifact) {
        (None, None) => {
            reasons.push("neither schema layout uses a BPE artifact".to_string());
            BpeArtifactCompatibility::NoArtifact
        }
        (None, Some(_)) => {
            reasons.push("new schema layout adds a BPE artifact".to_string());
            BpeArtifactCompatibility::Added
        }
        (Some(_), None) => {
            reasons.push("new schema layout removes the old BPE artifact".to_string());
            BpeArtifactCompatibility::Removed
        }
        (Some(old), Some(new)) if old.table_sha256 == new.table_sha256 => {
            reasons.push("BPE merge tables are identical".to_string());
            BpeArtifactCompatibility::Unchanged
        }
        (Some(_), Some(_)) if old_is_prefix_of_new == Some(true) && merge_count_delta > 0 => {
            reasons
                .push("new BPE merge table preserves the old table as an exact prefix".to_string());
            BpeArtifactCompatibility::PrefixExtended
        }
        (Some(_), Some(_)) => {
            reasons.push(
                "BPE merge table changed and is not an append-only prefix extension".to_string(),
            );
            BpeArtifactCompatibility::Changed
        }
    };

    BpeArtifactDiff {
        compatibility,
        old_present,
        new_present,
        file_sha256_changed,
        table_sha256_changed,
        merge_sequence_sha256_changed,
        base_vocab_size_changed,
        bpe_start_changed,
        merge_count_delta,
        old_is_prefix_of_new,
        first_differing_merge_index,
        reasons,
    }
}

fn bpe_artifact_sha256(layout: &TokenIdLayout) -> Option<&str> {
    layout
        .bpe_artifact
        .as_ref()
        .map(|artifact| artifact.file.sha256.as_str())
}

fn bpe_artifact_table_sha256(layout: &TokenIdLayout) -> Option<&str> {
    layout
        .bpe_artifact
        .as_ref()
        .map(|artifact| artifact.table_sha256.as_str())
}

fn bpe_artifact_merge_sequence_sha256(layout: &TokenIdLayout) -> Option<&str> {
    layout
        .bpe_artifact
        .as_ref()
        .map(|artifact| artifact.merge_sequence_sha256.as_str())
}

pub(super) fn bpe_artifact_schema(layout: &TokenIdLayout) -> Option<&str> {
    layout
        .bpe_artifact
        .as_ref()
        .map(|artifact| artifact.schema.as_str())
}

pub(super) fn bpe_artifact_base_vocab_size(layout: &TokenIdLayout) -> Option<u32> {
    layout
        .bpe_artifact
        .as_ref()
        .map(|artifact| artifact.base_vocab_size)
}

fn bpe_artifact_bpe_start(layout: &TokenIdLayout) -> Option<u32> {
    layout
        .bpe_artifact
        .as_ref()
        .map(|artifact| artifact.bpe_start)
}

fn is_merge_prefix(prefix: &[BpeMerge], full: &[BpeMerge]) -> bool {
    prefix.len() <= full.len()
        && prefix
            .iter()
            .zip(full.iter())
            .all(|(left, right)| left == right)
}

fn first_differing_merge_index(old: &[BpeMerge], new: &[BpeMerge]) -> Option<usize> {
    old.iter()
        .zip(new.iter())
        .position(|(old, new)| old != new)
        .or_else(|| (old.len() != new.len()).then_some(old.len().min(new.len())))
}
