//! Policy checks for schema-evolution compatibility reports.

use super::{
    ArtifactReuseDecision, BpeArtifactCompatibility, DownstreamEmbeddingHandoff,
    StableVocabularyAssessment, StableVocabularyDecision, StructuralCompatibilityAssessment,
    StructuralCompatibilityDecision, TokenIdLayout, TokenIdLayoutCompatibility, TokenIdLayoutDiff,
    TokenIdRange, VocabularyExtensionHandoff, VocabularyExtensionStatus,
};

pub(super) fn assess_stable_vocabulary_policy(
    compatibility: TokenIdLayoutCompatibility,
    structural_compatibility: &StructuralCompatibilityAssessment,
    structural_promotion_allowed: bool,
) -> StableVocabularyAssessment {
    match compatibility {
        TokenIdLayoutCompatibility::Unchanged => StableVocabularyAssessment {
            decision: StableVocabularyDecision::ExactReuse,
            generated_tokenizer_reuse: ArtifactReuseDecision::Reusable,
            downstream_bpe_or_embedding_reuse: ArtifactReuseDecision::Reusable,
            reasons: vec![
                "generated code hash and top-level token layout are unchanged for this compiler configuration"
                    .to_string(),
            ],
        },
        TokenIdLayoutCompatibility::TokenLayoutUnchanged => {
            match structural_compatibility.decision {
                StructuralCompatibilityDecision::AdditiveCompatible
                    if structural_promotion_allowed =>
                {
                    StableVocabularyAssessment {
                        decision: StableVocabularyDecision::AdditiveCompatible,
                        generated_tokenizer_reuse: ArtifactReuseDecision::RegenerateRequired,
                        downstream_bpe_or_embedding_reuse: ArtifactReuseDecision::ReviewRequired,
                        reasons: vec![
                            "generated code changed, so the generated tokenizer artifact must be regenerated"
                                .to_string(),
                            "structured token-map evidence proves existing token meanings are preserved for supported additive changes"
                                .to_string(),
                            "downstream BPE or embedding artifacts require explicit extension review for newly introduced tokens"
                                .to_string(),
                        ],
                    }
                }
                StructuralCompatibilityDecision::AdditiveCompatible => {
                    StableVocabularyAssessment {
                        decision: StableVocabularyDecision::LayoutOnlyUnchanged,
                        generated_tokenizer_reuse: ArtifactReuseDecision::RegenerateRequired,
                        downstream_bpe_or_embedding_reuse: ArtifactReuseDecision::ReviewRequired,
                        reasons: vec![
                            "generated code changed, so the generated tokenizer artifact must be regenerated"
                                .to_string(),
                            "structured token-map evidence is additive, but changed compiler settings or BPE artifacts prevent automatic stable-vocabulary promotion"
                                .to_string(),
                        ],
                    }
                }
                StructuralCompatibilityDecision::BreakingChange => StableVocabularyAssessment {
                    decision: StableVocabularyDecision::BreakingChange,
                    generated_tokenizer_reuse: ArtifactReuseDecision::RegenerateRequired,
                    downstream_bpe_or_embedding_reuse: ArtifactReuseDecision::RegenerateRequired,
                    reasons: vec![
                        "top-level ranges are unchanged, but structured token semantics changed"
                            .to_string(),
                        "downstream BPE artifacts, embeddings, and schema-local token assumptions require regeneration or an explicit migration"
                            .to_string(),
                    ],
                },
                StructuralCompatibilityDecision::Unchanged
                | StructuralCompatibilityDecision::NotProven => StableVocabularyAssessment {
                    decision: StableVocabularyDecision::LayoutOnlyUnchanged,
                    generated_tokenizer_reuse: ArtifactReuseDecision::RegenerateRequired,
                    downstream_bpe_or_embedding_reuse: ArtifactReuseDecision::ReviewRequired,
                    reasons: vec![
                        "generated code changed, so the generated tokenizer artifact must be regenerated"
                            .to_string(),
                        "unchanged top-level ranges do not prove that existing structural token IDs preserve identical meaning"
                            .to_string(),
                        "schema changes outside the current structural token-map proof require review before reusing downstream BPE or embeddings"
                            .to_string(),
                    ],
                },
            }
        }
        TokenIdLayoutCompatibility::TokenLayoutChanged => StableVocabularyAssessment {
            decision: StableVocabularyDecision::BreakingChange,
            generated_tokenizer_reuse: ArtifactReuseDecision::RegenerateRequired,
            downstream_bpe_or_embedding_reuse: ArtifactReuseDecision::RegenerateRequired,
            reasons: vec![
                "one or more top-level token ranges moved".to_string(),
                "downstream BPE artifacts, embeddings, and schema-local token assumptions require regeneration or an explicit migration"
                    .to_string(),
            ],
        },
    }
}

pub(super) fn build_vocabulary_extension_handoff(
    old: &TokenIdLayout,
    new: &TokenIdLayout,
    diff: &TokenIdLayoutDiff,
) -> VocabularyExtensionHandoff {
    let bpe_prefix_extension =
        diff.bpe_artifact.compatibility == BpeArtifactCompatibility::PrefixExtended;
    let has_appended_vocab = new.vocab_size > old.vocab_size;
    let (status, downstream_embedding_handoff, reusable_schema_local_token_range, mut notes) =
        if diff.stable_vocabulary.decision == StableVocabularyDecision::ExactReuse {
            (
                VocabularyExtensionStatus::NoExtensionNeeded,
                DownstreamEmbeddingHandoff::ReuseExistingRows,
                Some(TokenIdRange {
                    start: 0,
                    end_exclusive: old.vocab_size,
                }),
                vec![
                    "old generated tokenizer, token IDs, and downstream rows can be reused for the same compiler configuration"
                        .to_string(),
                ],
            )
        } else if diff.stable_vocabulary.decision == StableVocabularyDecision::AdditiveCompatible {
            (
                VocabularyExtensionStatus::CandidateExtensionArtifact,
                DownstreamEmbeddingHandoff::InitializeOrReviewRows,
                Some(TokenIdRange {
                    start: 0,
                    end_exclusive: old.vocab_size,
                }),
                vec![
                    "existing schema-local token IDs are preserved by compiler-side structural evidence"
                        .to_string(),
                    "new structural paths need downstream embedding-row initialization or explicit reuse review"
                        .to_string(),
                ],
            )
        } else if bpe_prefix_extension && has_appended_vocab {
            (
                VocabularyExtensionStatus::CandidateExtensionArtifact,
                DownstreamEmbeddingHandoff::InitializeOrReviewRows,
                Some(TokenIdRange {
                    start: 0,
                    end_exclusive: old.vocab_size,
                }),
                vec![
                    "new BPE token IDs are appended after the old vocabulary and the old merge table is preserved as a prefix"
                        .to_string(),
                    "stable-vocabulary compatibility remains conservative; use this block only as a downstream migration handoff"
                        .to_string(),
                ],
            )
        } else if diff.stable_vocabulary.decision == StableVocabularyDecision::LayoutOnlyUnchanged
            || has_appended_vocab
        {
            let review_note = if has_appended_vocab {
                "vocabulary size grew without an exact append-only BPE or structural extension proof"
            } else {
                "top-level token ranges are unchanged, but this report does not prove structural token meanings are stable"
            };
            (
                VocabularyExtensionStatus::ReviewOnly,
                DownstreamEmbeddingHandoff::ReviewRequired,
                None,
                vec![review_note.to_string()],
            )
        } else {
            (
                VocabularyExtensionStatus::RegenerationRequired,
                DownstreamEmbeddingHandoff::RegenerateOrMigrate,
                None,
                vec![
                    "compiler-side evidence classifies this transition as breaking for vocabulary extension"
                        .to_string(),
                ],
            )
        };

    if new.vocab_size > old.vocab_size {
        notes.push(
            "new vocabulary size appends top-level token IDs; downstream artifacts must initialize or migrate those rows explicitly"
                .to_string(),
        );
    }
    if bpe_prefix_extension {
        notes.push(
            "BPE merge-table prefix extension is candidate evidence for vocabulary extension, not proof of unchanged encoded sequences or model behavior"
                .to_string(),
        );
    }

    VocabularyExtensionHandoff {
        artifact_kind: "tct_vocabulary_extension_handoff",
        artifact_version: 1,
        status,
        stable_vocabulary_decision: diff.stable_vocabulary.decision,
        generated_tokenizer_reuse: diff.stable_vocabulary.generated_tokenizer_reuse,
        downstream_embedding_handoff,
        old_schema_sha256: old.structural_token_map.schema_sha256.clone(),
        new_schema_sha256: new.structural_token_map.schema_sha256.clone(),
        layout_affecting_inputs_match: layout_affecting_inputs_match(old, new),
        old_vocab_size: old.vocab_size,
        new_vocab_size: new.vocab_size,
        vocab_size_delta: diff.vocab_size_delta,
        old_vocab_range: TokenIdRange {
            start: 0,
            end_exclusive: old.vocab_size,
        },
        new_vocab_range: TokenIdRange {
            start: 0,
            end_exclusive: new.vocab_size,
        },
        reusable_schema_local_token_range,
        appended_token_ranges: appended_token_ranges(old, new),
        new_structural_paths: diff.structural_compatibility.changed_paths.clone(),
        bpe_artifact_compatibility: diff.bpe_artifact.compatibility,
        bpe_prefix_extension: diff.bpe_artifact.old_is_prefix_of_new,
        old_bpe_table_sha256: old
            .bpe_artifact
            .as_ref()
            .map(|artifact| artifact.table_sha256.clone()),
        new_bpe_table_sha256: new
            .bpe_artifact
            .as_ref()
            .map(|artifact| artifact.table_sha256.clone()),
        old_bpe_merge_sequence_sha256: old
            .bpe_artifact
            .as_ref()
            .map(|artifact| artifact.merge_sequence_sha256.clone()),
        new_bpe_merge_sequence_sha256: new
            .bpe_artifact
            .as_ref()
            .map(|artifact| artifact.merge_sequence_sha256.clone()),
        required_downstream_metadata: vec![
            "old_schema_file_and_hash",
            "new_schema_file_and_hash",
            "token_diff_report_hash",
            "generated_tokenizer_artifact_hash",
            "sequence_or_handoff_profile",
            "bpe_artifact_fingerprints",
            "embedding_initialization_policy",
            "downstream_training_or_finetuning_command",
            "downstream_result_file_provenance",
        ],
        notes,
    }
}

fn layout_affecting_inputs_match(old: &TokenIdLayout, new: &TokenIdLayout) -> bool {
    old.root_type_name == new.root_type_name
        && old.optimization_level == new.optimization_level
        && old.optimization_pass_overrides == new.optimization_pass_overrides
        && old.string_encoding == new.string_encoding
        && old.max_base_vocab == new.max_base_vocab
        && old.base_vocab_size == new.base_vocab_size
        && old.encoding_base == new.encoding_base
        && old.bpe_start == new.bpe_start
}

fn appended_token_ranges(old: &TokenIdLayout, new: &TokenIdLayout) -> Vec<TokenIdRange> {
    if new.vocab_size > old.vocab_size {
        vec![TokenIdRange {
            start: old.vocab_size,
            end_exclusive: new.vocab_size,
        }]
    } else {
        Vec::new()
    }
}
