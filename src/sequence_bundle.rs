//! Sequence-boundary bundle manifest contracts.
//!
//! Generated single-schema tokenizers expose sequence-boundary metadata in their
//! generated code. This module defines the repo-level bundle shape that future
//! multi-schema wrappers can serialize, validate, and hand to downstream
//! training or serving code.

use std::collections::BTreeSet;
use std::fmt;

use serde::{Deserialize, Serialize};

/// Supported multi-schema bundle manifest version.
pub const SEQUENCE_BUNDLE_MANIFEST_VERSION: u32 = 1;

/// Supported generated sequence-boundary version.
pub const CURRENT_SEQUENCE_BOUNDARY_VERSION: u32 = 1;

/// Current generation-time logit mask policy.
pub const SELECTED_SCHEMA_PAYLOAD_PLUS_EOS: &str = "selected_schema_payload_plus_eos";

/// Supported hybrid prompt/TCT response boundary manifest version.
pub const PROMPT_RESPONSE_BOUNDARY_MANIFEST_VERSION: u32 = 1;

/// Boundary kind for pretrained-prompt tokens followed by a TCT response span.
pub const PROMPT_TCT_RESPONSE_BOUNDARY_KIND: &str = "prompt_bpe_to_tct_response";

/// Scope of validity-by-construction guarantees for hybrid prompt/TCT use.
pub const TCT_RESPONSE_SPAN_VALIDITY_SCOPE: &str = "tct_response_span_only";

/// Current downstream direct HF/TCT profile name used before bundle migration.
pub const DIRECT_LOCAL_PAYLOAD_PROFILE_NAME: &str = "nanochat_direct_local_payload";

/// Current downstream direct HF/TCT profile version.
pub const DIRECT_LOCAL_PAYLOAD_PROFILE_VERSION: u32 = 1;

/// Shape of the current downstream direct HF/TCT token sequence.
pub const DIRECT_LOCAL_PAYLOAD_HANDOFF_SHAPE: &str =
    "prompt_ids_schema_sentinel_tct_start_local_payload_tct_end";

/// Current direct profile treats `<|tct_end|>` as a global HF special token.
pub const GLOBAL_HF_SPECIAL_TCT_END_ROLE: &str = "global_hf_special_token";

/// Half-open token range `[start, end_exclusive)`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TokenRange {
    pub start: u32,
    pub end_exclusive: u32,
}

impl TokenRange {
    pub fn new(start: u32, end_exclusive: u32) -> Self {
        Self {
            start,
            end_exclusive,
        }
    }

    pub fn len(&self) -> u32 {
        self.end_exclusive.saturating_sub(self.start)
    }

    pub fn is_empty(&self) -> bool {
        self.start >= self.end_exclusive
    }

    pub fn contains(&self, token: u32) -> bool {
        self.start <= token && token < self.end_exclusive
    }
}

/// Bundle-level special-token policy shared by all bundled schemas.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SequenceBundleSpecialTokens {
    pub pad_token: u32,
    pub bos_token: u32,
    pub eos_token: u32,
    pub start_token: u32,
    pub end_token: u32,
}

impl Default for SequenceBundleSpecialTokens {
    fn default() -> Self {
        Self::new(0, 1, 2)
    }
}

impl SequenceBundleSpecialTokens {
    /// Build special tokens from their canonical PAD/BOS/EOS values.
    ///
    /// `start_token` is an alias for `bos_token`, and `end_token` is an alias
    /// for `eos_token`. Use this constructor instead of struct literals when
    /// creating manifests in Rust so those aliases cannot drift at construction
    /// time. Deserialized manifests are still validated explicitly.
    pub const fn new(pad_token: u32, bos_token: u32, eos_token: u32) -> Self {
        Self {
            pad_token,
            bos_token,
            eos_token,
            start_token: bos_token,
            end_token: eos_token,
        }
    }

    fn unique_tokens(&self) -> BTreeSet<u32> {
        BTreeSet::from([self.pad_token, self.bos_token, self.eos_token])
    }

    fn contains(&self, token: u32) -> bool {
        self.pad_token == token
            || self.bos_token == token
            || self.eos_token == token
            || self.start_token == token
            || self.end_token == token
    }
}

/// Label and generation masking policy for bundle-aware training.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SequenceBundleMaskingPolicy {
    pub mask_prompt_tokens_from_labels: bool,
    pub mask_schema_token_from_labels: bool,
    pub mask_bos_token_from_labels: bool,
    pub mask_pad_token_from_labels: bool,
    pub mask_eos_token_from_labels: bool,
    pub generation_requires_schema_selection: bool,
    pub generation_allowed_tokens: String,
}

impl Default for SequenceBundleMaskingPolicy {
    fn default() -> Self {
        Self {
            mask_prompt_tokens_from_labels: true,
            mask_schema_token_from_labels: true,
            mask_bos_token_from_labels: true,
            mask_pad_token_from_labels: true,
            mask_eos_token_from_labels: false,
            generation_requires_schema_selection: true,
            generation_allowed_tokens: SELECTED_SCHEMA_PAYLOAD_PLUS_EOS.to_string(),
        }
    }
}

/// Metadata for one schema tokenizer inside a bundle.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SequenceBundleSchema {
    pub schema_id: String,
    pub schema_token: u32,
    pub schema_sentinel: Option<String>,
    pub generated_artifact: Option<String>,
    pub raw_vocab_size: u32,
    pub raw_pad_token: u32,
    pub raw_payload: TokenRange,
    pub global_payload: TokenRange,
    pub bpe_enabled: bool,
}

impl SequenceBundleSchema {
    pub fn new(
        schema_id: impl Into<String>,
        schema_token: u32,
        raw_vocab_size: u32,
        raw_pad_token: u32,
        global_payload_start: u32,
        bpe_enabled: bool,
    ) -> Self {
        let raw_payload = TokenRange::new(0, raw_pad_token);
        let global_payload = TokenRange::new(
            global_payload_start,
            global_payload_start.saturating_add(raw_payload.len()),
        );
        Self {
            schema_id: schema_id.into(),
            schema_token,
            schema_sentinel: None,
            generated_artifact: None,
            raw_vocab_size,
            raw_pad_token,
            raw_payload,
            global_payload,
            bpe_enabled,
        }
    }

    pub fn with_schema_sentinel(mut self, sentinel: impl Into<String>) -> Self {
        self.schema_sentinel = Some(sentinel.into());
        self
    }

    pub fn with_generated_artifact(mut self, artifact: impl Into<String>) -> Self {
        self.generated_artifact = Some(artifact.into());
        self
    }

    pub fn with_global_payload_start(mut self, global_payload_start: u32) -> Self {
        self.global_payload = TokenRange::new(
            global_payload_start,
            global_payload_start.saturating_add(self.raw_payload.len()),
        );
        self
    }

    pub fn local_to_global(&self, local_token: u32) -> Option<u32> {
        self.raw_payload
            .contains(local_token)
            .then(|| self.global_payload.start + local_token.saturating_sub(self.raw_payload.start))
    }

    pub fn global_to_local(&self, global_token: u32) -> Option<u32> {
        self.global_payload.contains(global_token).then(|| {
            self.raw_payload.start + global_token.saturating_sub(self.global_payload.start)
        })
    }
}

/// Serializable multi-schema sequence bundle manifest.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SequenceBundleManifest {
    pub manifest_version: u32,
    pub sequence_boundary_version: u32,
    pub vocab_size: u32,
    pub special_tokens: SequenceBundleSpecialTokens,
    pub masking: SequenceBundleMaskingPolicy,
    pub schemas: Vec<SequenceBundleSchema>,
}

impl SequenceBundleManifest {
    pub fn new(vocab_size: u32, schemas: Vec<SequenceBundleSchema>) -> Self {
        Self {
            manifest_version: SEQUENCE_BUNDLE_MANIFEST_VERSION,
            sequence_boundary_version: CURRENT_SEQUENCE_BOUNDARY_VERSION,
            vocab_size,
            special_tokens: SequenceBundleSpecialTokens::default(),
            masking: SequenceBundleMaskingPolicy::default(),
            schemas,
        }
    }

    /// SHA-256 digest of the serialized manifest metadata.
    pub fn manifest_sha256(&self) -> Result<String, serde_json::Error> {
        crate::provenance::json_sha256(self)
    }

    pub fn validate(&self) -> Result<(), SequenceBundleManifestError> {
        if self.manifest_version != SEQUENCE_BUNDLE_MANIFEST_VERSION {
            return Err(SequenceBundleManifestError::UnsupportedManifestVersion {
                actual: self.manifest_version,
                expected: SEQUENCE_BUNDLE_MANIFEST_VERSION,
            });
        }
        if self.sequence_boundary_version != CURRENT_SEQUENCE_BOUNDARY_VERSION {
            return Err(SequenceBundleManifestError::UnsupportedBoundaryVersion {
                actual: self.sequence_boundary_version,
                expected: CURRENT_SEQUENCE_BOUNDARY_VERSION,
            });
        }
        if self.schemas.is_empty() {
            return Err(SequenceBundleManifestError::NoSchemas);
        }

        self.validate_special_tokens()?;
        self.validate_masking_policy()?;
        self.validate_schemas()
    }

    pub fn schema_for_token(&self, schema_token: u32) -> Option<&SequenceBundleSchema> {
        self.schemas
            .iter()
            .find(|schema| schema.schema_token == schema_token)
    }

    pub fn schema_for_id(&self, schema_id: &str) -> Option<&SequenceBundleSchema> {
        self.schemas
            .iter()
            .find(|schema| schema.schema_id == schema_id)
    }

    pub fn generation_mask_for_schema_token(
        &self,
        schema_token: u32,
    ) -> Result<SequenceGenerationMask, SequenceBundleManifestError> {
        self.validate()?;
        let schema = self
            .schema_for_token(schema_token)
            .ok_or(SequenceBundleManifestError::UnknownSchemaToken { schema_token })?;
        Ok(SequenceGenerationMask {
            schema_id: schema.schema_id.clone(),
            payload: schema.global_payload.clone(),
            eos_token: self.special_tokens.eos_token,
        })
    }

    pub fn wrap_payload_for_schema_id(
        &self,
        schema_id: &str,
        local_payload: &[u32],
    ) -> Result<Vec<u32>, SequenceBundleManifestError> {
        self.validate()?;
        let schema = self.schema_for_id(schema_id).ok_or_else(|| {
            SequenceBundleManifestError::UnknownSchemaId {
                schema_id: schema_id.to_string(),
            }
        })?;
        self.wrap_payload_for_schema_token(schema.schema_token, local_payload)
    }

    pub fn wrap_payload_for_schema_token(
        &self,
        schema_token: u32,
        local_payload: &[u32],
    ) -> Result<Vec<u32>, SequenceBundleManifestError> {
        self.validate()?;
        let schema = self
            .schema_for_token(schema_token)
            .ok_or(SequenceBundleManifestError::UnknownSchemaToken { schema_token })?;

        let mut sequence = Vec::with_capacity(local_payload.len() + 3);
        sequence.push(self.special_tokens.bos_token);
        sequence.push(schema.schema_token);
        for (offset, local_token) in local_payload.iter().copied().enumerate() {
            let global_token = schema.local_to_global(local_token).ok_or_else(|| {
                SequenceBundleManifestError::InvalidLocalPayloadToken {
                    schema_id: schema.schema_id.clone(),
                    token: local_token,
                    offset,
                }
            })?;
            sequence.push(global_token);
        }
        sequence.push(self.special_tokens.eos_token);
        Ok(sequence)
    }

    pub fn unwrap_payload(
        &self,
        sequence: &[u32],
    ) -> Result<SequenceBundlePayload, SequenceBundleManifestError> {
        self.validate()?;
        if sequence.first().copied() != Some(self.special_tokens.bos_token) {
            return Err(SequenceBundleManifestError::InvalidSequenceStart {
                expected: self.special_tokens.bos_token,
                actual: sequence.first().copied(),
            });
        }

        let schema_token =
            sequence
                .get(1)
                .copied()
                .ok_or(SequenceBundleManifestError::MissingSchemaToken {
                    expected_position: 1,
                })?;
        let schema = self
            .schema_for_token(schema_token)
            .ok_or(SequenceBundleManifestError::UnknownSchemaToken { schema_token })?;

        let eos_index = sequence
            .iter()
            .enumerate()
            .skip(2)
            .find_map(|(index, token)| (*token == self.special_tokens.eos_token).then_some(index))
            .ok_or(SequenceBundleManifestError::MissingEosToken { schema_token })?;

        for (offset, token) in sequence[eos_index + 1..].iter().copied().enumerate() {
            if token != self.special_tokens.pad_token {
                return Err(SequenceBundleManifestError::NonPaddingAfterEos {
                    token,
                    offset: eos_index + 1 + offset,
                    expected_pad: self.special_tokens.pad_token,
                });
            }
        }

        let mut local_payload = Vec::with_capacity(eos_index.saturating_sub(2));
        let mut global_payload = Vec::with_capacity(eos_index.saturating_sub(2));
        for (offset, global_token) in sequence[2..eos_index].iter().copied().enumerate() {
            let local_token = schema.global_to_local(global_token).ok_or_else(|| {
                SequenceBundleManifestError::InvalidGlobalPayloadToken {
                    schema_id: schema.schema_id.clone(),
                    token: global_token,
                    offset,
                }
            })?;
            global_payload.push(global_token);
            local_payload.push(local_token);
        }

        Ok(SequenceBundlePayload {
            schema_id: schema.schema_id.clone(),
            schema_token,
            local_payload,
            global_payload,
            consumed_tokens: eos_index + 1,
            total_tokens: sequence.len(),
        })
    }

    fn validate_special_tokens(&self) -> Result<(), SequenceBundleManifestError> {
        let tokens = &self.special_tokens;
        if tokens.start_token != tokens.bos_token {
            return Err(SequenceBundleManifestError::AliasMismatch {
                alias: "start_token",
                alias_value: tokens.start_token,
                canonical: "bos_token",
                canonical_value: tokens.bos_token,
            });
        }
        if tokens.end_token != tokens.eos_token {
            return Err(SequenceBundleManifestError::AliasMismatch {
                alias: "end_token",
                alias_value: tokens.end_token,
                canonical: "eos_token",
                canonical_value: tokens.eos_token,
            });
        }
        if tokens.unique_tokens().len() != 3 {
            return Err(SequenceBundleManifestError::DuplicateSpecialTokens);
        }
        for token in [tokens.pad_token, tokens.bos_token, tokens.eos_token] {
            if token >= self.vocab_size {
                return Err(SequenceBundleManifestError::TokenOutsideVocab {
                    token,
                    vocab_size: self.vocab_size,
                });
            }
        }
        Ok(())
    }

    fn validate_masking_policy(&self) -> Result<(), SequenceBundleManifestError> {
        let masking = &self.masking;
        if !masking.mask_prompt_tokens_from_labels
            || !masking.mask_schema_token_from_labels
            || !masking.mask_bos_token_from_labels
            || !masking.mask_pad_token_from_labels
            || masking.mask_eos_token_from_labels
            || !masking.generation_requires_schema_selection
            || masking.generation_allowed_tokens != SELECTED_SCHEMA_PAYLOAD_PLUS_EOS
        {
            return Err(SequenceBundleManifestError::UnsupportedMaskingPolicy);
        }
        Ok(())
    }

    fn validate_schemas(&self) -> Result<(), SequenceBundleManifestError> {
        let mut schema_ids = BTreeSet::new();
        let mut schema_tokens = BTreeSet::new();
        let mut schema_sentinels = BTreeSet::new();

        for schema in &self.schemas {
            if schema.schema_id.is_empty() {
                return Err(SequenceBundleManifestError::EmptySchemaId);
            }
            if !schema_ids.insert(schema.schema_id.as_str()) {
                return Err(SequenceBundleManifestError::DuplicateSchemaId {
                    schema_id: schema.schema_id.clone(),
                });
            }
            if !schema_tokens.insert(schema.schema_token) {
                return Err(SequenceBundleManifestError::DuplicateSchemaToken {
                    schema_token: schema.schema_token,
                });
            }
            if self.special_tokens.contains(schema.schema_token) {
                return Err(
                    SequenceBundleManifestError::SchemaTokenCollidesWithSpecial {
                        schema_id: schema.schema_id.clone(),
                        schema_token: schema.schema_token,
                    },
                );
            }
            if schema.schema_token >= self.vocab_size {
                return Err(SequenceBundleManifestError::TokenOutsideVocab {
                    token: schema.schema_token,
                    vocab_size: self.vocab_size,
                });
            }
            if let Some(sentinel) = &schema.schema_sentinel
                && sentinel.is_empty()
            {
                return Err(SequenceBundleManifestError::EmptySchemaSentinel {
                    schema_id: schema.schema_id.clone(),
                });
            }
            if let Some(sentinel) = &schema.schema_sentinel
                && !schema_sentinels.insert(sentinel.as_str())
            {
                return Err(SequenceBundleManifestError::DuplicateSchemaSentinel {
                    sentinel: sentinel.clone(),
                });
            }
            self.validate_schema_ranges(schema)?;
        }
        Ok(())
    }

    fn validate_schema_ranges(
        &self,
        schema: &SequenceBundleSchema,
    ) -> Result<(), SequenceBundleManifestError> {
        if schema.raw_vocab_size == 0 || schema.raw_pad_token >= schema.raw_vocab_size {
            return Err(SequenceBundleManifestError::InvalidRawVocab {
                schema_id: schema.schema_id.clone(),
                raw_vocab_size: schema.raw_vocab_size,
                raw_pad_token: schema.raw_pad_token,
            });
        }
        if schema.raw_payload.is_empty()
            || schema.raw_payload.end_exclusive > schema.raw_pad_token
            || schema.raw_payload.start > schema.raw_payload.end_exclusive
        {
            return Err(SequenceBundleManifestError::InvalidPayloadRange {
                schema_id: schema.schema_id.clone(),
                range_name: "raw_payload",
                start: schema.raw_payload.start,
                end_exclusive: schema.raw_payload.end_exclusive,
            });
        }
        if schema.global_payload.is_empty()
            || schema.global_payload.end_exclusive > self.vocab_size
            || schema.global_payload.len() != schema.raw_payload.len()
        {
            return Err(SequenceBundleManifestError::InvalidPayloadRange {
                schema_id: schema.schema_id.clone(),
                range_name: "global_payload",
                start: schema.global_payload.start,
                end_exclusive: schema.global_payload.end_exclusive,
            });
        }
        for token in [
            self.special_tokens.pad_token,
            self.special_tokens.bos_token,
            self.special_tokens.eos_token,
            schema.schema_token,
        ] {
            if schema.global_payload.contains(token) {
                return Err(
                    SequenceBundleManifestError::PayloadRangeCollidesWithReservedToken {
                        schema_id: schema.schema_id.clone(),
                        token,
                    },
                );
            }
        }
        Ok(())
    }
}

/// Payload extracted from a bundle sequence after boundary validation.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SequenceBundlePayload {
    pub schema_id: String,
    pub schema_token: u32,
    pub local_payload: Vec<u32>,
    pub global_payload: Vec<u32>,
    pub consumed_tokens: usize,
    pub total_tokens: usize,
}

/// Generation mask implied after selecting a schema.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SequenceGenerationMask {
    pub schema_id: String,
    pub payload: TokenRange,
    pub eos_token: u32,
}

impl SequenceGenerationMask {
    pub fn allows(&self, token: u32) -> bool {
        token == self.eos_token || self.payload.contains(token)
    }
}

/// Label and generation masking contract for the direct HF/TCT compatibility profile.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct DirectHfTctMaskingPolicy {
    pub mask_prompt_tokens_from_labels: bool,
    pub mask_schema_sentinel_from_labels: bool,
    pub mask_tct_start_from_labels: bool,
    pub mask_padding_from_labels: bool,
    pub tct_end_is_unmasked_training_target: bool,
    pub generation_allowed_tokens: String,
}

impl Default for DirectHfTctMaskingPolicy {
    fn default() -> Self {
        Self {
            mask_prompt_tokens_from_labels: true,
            mask_schema_sentinel_from_labels: true,
            mask_tct_start_from_labels: true,
            mask_padding_from_labels: true,
            tct_end_is_unmasked_training_target: true,
            generation_allowed_tokens: SELECTED_SCHEMA_PAYLOAD_PLUS_EOS.to_string(),
        }
    }
}

/// Serializable compatibility record for the current downstream direct HF/TCT profile.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct DirectHfTctCompatibilityProfile {
    pub profile_name: String,
    pub profile_version: u32,
    pub handoff_shape: String,
    pub schema_id: String,
    pub schema_sentinel: String,
    pub schema_sentinel_token: u32,
    pub tct_start_sentinel: String,
    pub tct_start_token: u32,
    pub tct_end_sentinel: String,
    pub tct_end_token: u32,
    pub tct_end_role: String,
    pub local_payload: TokenRange,
    pub hf_payload: TokenRange,
    pub raw_pad_token: u32,
    pub raw_pad_hf_token: u32,
    pub raw_pad_is_payload: bool,
    pub masking: DirectHfTctMaskingPolicy,
    pub generation_mask: SequenceGenerationMask,
}

impl DirectHfTctCompatibilityProfile {
    /// SHA-256 digest of the serialized direct HF/TCT profile metadata.
    pub fn profile_sha256(&self) -> Result<String, serde_json::Error> {
        crate::provenance::json_sha256(self)
    }

    pub fn validate(&self) -> Result<(), SequenceBundleManifestError> {
        if self.profile_name != DIRECT_LOCAL_PAYLOAD_PROFILE_NAME {
            return Err(
                SequenceBundleManifestError::UnsupportedDirectHfTctProfileName {
                    actual: self.profile_name.clone(),
                    expected: DIRECT_LOCAL_PAYLOAD_PROFILE_NAME,
                },
            );
        }
        if self.profile_version != DIRECT_LOCAL_PAYLOAD_PROFILE_VERSION {
            return Err(
                SequenceBundleManifestError::UnsupportedDirectHfTctProfileVersion {
                    actual: self.profile_version,
                    expected: DIRECT_LOCAL_PAYLOAD_PROFILE_VERSION,
                },
            );
        }
        if self.handoff_shape != DIRECT_LOCAL_PAYLOAD_HANDOFF_SHAPE {
            return Err(
                SequenceBundleManifestError::UnsupportedDirectHfTctHandoffShape {
                    actual: self.handoff_shape.clone(),
                    expected: DIRECT_LOCAL_PAYLOAD_HANDOFF_SHAPE,
                },
            );
        }
        if self.tct_end_role != GLOBAL_HF_SPECIAL_TCT_END_ROLE {
            return Err(SequenceBundleManifestError::UnsupportedDirectHfTctEndRole {
                actual: self.tct_end_role.clone(),
                expected: GLOBAL_HF_SPECIAL_TCT_END_ROLE,
            });
        }
        if self.schema_id.is_empty() {
            return Err(SequenceBundleManifestError::EmptySchemaId);
        }
        if self.schema_sentinel.is_empty() {
            return Err(SequenceBundleManifestError::EmptyDirectHfTctProfileField {
                field: "schema_sentinel",
            });
        }
        if self.tct_start_sentinel.is_empty() {
            return Err(SequenceBundleManifestError::EmptyDirectHfTctProfileField {
                field: "tct_start_sentinel",
            });
        }
        if self.tct_end_sentinel.is_empty() {
            return Err(SequenceBundleManifestError::EmptyDirectHfTctProfileField {
                field: "tct_end_sentinel",
            });
        }
        if self.local_payload.is_empty()
            || self.hf_payload.is_empty()
            || self.hf_payload.len() != self.local_payload.len()
        {
            return Err(
                SequenceBundleManifestError::InvalidDirectHfTctPayloadRange {
                    schema_id: self.schema_id.clone(),
                    local_start: self.local_payload.start,
                    local_end_exclusive: self.local_payload.end_exclusive,
                    hf_start: self.hf_payload.start,
                    hf_end_exclusive: self.hf_payload.end_exclusive,
                },
            );
        }

        let raw_pad_is_payload = self.local_payload.contains(self.raw_pad_token);
        if self.raw_pad_is_payload != raw_pad_is_payload || raw_pad_is_payload {
            return Err(
                SequenceBundleManifestError::InvalidDirectHfTctRawPadPolicy {
                    schema_id: self.schema_id.clone(),
                    raw_pad_token: self.raw_pad_token,
                    raw_pad_is_payload: self.raw_pad_is_payload,
                },
            );
        }
        let raw_pad_offset = self
            .raw_pad_token
            .checked_sub(self.local_payload.start)
            .ok_or_else(
                || SequenceBundleManifestError::InvalidDirectHfTctRawPadOffset {
                    schema_id: self.schema_id.clone(),
                    raw_pad_token: self.raw_pad_token,
                    local_payload_start: self.local_payload.start,
                    hf_payload_start: self.hf_payload.start,
                },
            )?;
        let expected_raw_pad_hf_token = self
            .hf_payload
            .start
            .checked_add(raw_pad_offset)
            .ok_or_else(
                || SequenceBundleManifestError::InvalidDirectHfTctRawPadOffset {
                    schema_id: self.schema_id.clone(),
                    raw_pad_token: self.raw_pad_token,
                    local_payload_start: self.local_payload.start,
                    hf_payload_start: self.hf_payload.start,
                },
            )?;
        if self.raw_pad_hf_token != expected_raw_pad_hf_token {
            return Err(
                SequenceBundleManifestError::InvalidDirectHfTctRawPadHfToken {
                    schema_id: self.schema_id.clone(),
                    actual: self.raw_pad_hf_token,
                    expected: expected_raw_pad_hf_token,
                },
            );
        }
        if self.masking != DirectHfTctMaskingPolicy::default() {
            return Err(SequenceBundleManifestError::UnsupportedDirectHfTctMaskingPolicy);
        }
        if self.generation_mask.schema_id != self.schema_id
            || self.generation_mask.payload != self.hf_payload
            || self.generation_mask.eos_token != self.tct_end_token
        {
            return Err(
                SequenceBundleManifestError::InvalidDirectHfTctGenerationMask {
                    schema_id: self.schema_id.clone(),
                },
            );
        }
        Ok(())
    }
}

/// Metadata for the pretrained tokenizer used on the natural-language prompt.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PromptTokenizerMetadata {
    pub tokenizer_id: Option<String>,
    pub base_vocab_size: u32,
    pub first_available_token_id: u32,
}

impl PromptTokenizerMetadata {
    pub fn new(base_vocab_size: u32) -> Self {
        Self {
            tokenizer_id: None,
            base_vocab_size,
            first_available_token_id: base_vocab_size,
        }
    }

    pub fn with_tokenizer_id(mut self, tokenizer_id: impl Into<String>) -> Self {
        self.tokenizer_id = Some(tokenizer_id.into());
        self
    }

    pub fn with_first_available_token_id(mut self, first_available_token_id: u32) -> Self {
        self.first_available_token_id = first_available_token_id;
        self
    }
}

/// Text and numeric sentinels used to enter and exit a TCT response span.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PromptResponseBoundaryTokens {
    pub tct_start_sentinel: String,
    pub tct_start_token: u32,
    pub tct_end_sentinel: String,
    pub tct_end_token: u32,
}

impl PromptResponseBoundaryTokens {
    pub fn new(tct_start_token: u32, tct_end_token: u32) -> Self {
        Self {
            tct_start_sentinel: "<|tct_start|>".to_string(),
            tct_start_token,
            tct_end_sentinel: "<|tct_end|>".to_string(),
            tct_end_token,
        }
    }

    pub fn with_sentinels(
        mut self,
        tct_start_sentinel: impl Into<String>,
        tct_end_sentinel: impl Into<String>,
    ) -> Self {
        self.tct_start_sentinel = tct_start_sentinel.into();
        self.tct_end_sentinel = tct_end_sentinel.into();
        self
    }
}

/// Hybrid prompt/TCT response metadata for downstream tokenizer wrappers.
///
/// Prompt tokens keep the pretrained tokenizer's native IDs. TCT response
/// tokens are mapped from the validated bundle vocabulary into a downstream
/// contiguous added-token block with `hf_added_token_block.start + bundle_id`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PromptResponseBoundaryManifest {
    pub manifest_version: u32,
    pub boundary_kind: String,
    pub validity_scope: String,
    pub prompt_tokenizer: PromptTokenizerMetadata,
    pub hf_added_token_block: TokenRange,
    pub tct_bundle: SequenceBundleManifest,
    pub text_tokens: PromptResponseBoundaryTokens,
}

impl PromptResponseBoundaryManifest {
    pub fn new(
        prompt_tokenizer: PromptTokenizerMetadata,
        text_tokens: PromptResponseBoundaryTokens,
        hf_added_token_start: u32,
        tct_bundle: SequenceBundleManifest,
    ) -> Self {
        let hf_added_token_block = TokenRange::new(
            hf_added_token_start,
            hf_added_token_start.saturating_add(tct_bundle.vocab_size),
        );
        Self {
            manifest_version: PROMPT_RESPONSE_BOUNDARY_MANIFEST_VERSION,
            boundary_kind: PROMPT_TCT_RESPONSE_BOUNDARY_KIND.to_string(),
            validity_scope: TCT_RESPONSE_SPAN_VALIDITY_SCOPE.to_string(),
            prompt_tokenizer,
            hf_added_token_block,
            tct_bundle,
            text_tokens,
        }
    }

    /// SHA-256 digest of the serialized prompt/TCT boundary metadata.
    pub fn manifest_sha256(&self) -> Result<String, serde_json::Error> {
        crate::provenance::json_sha256(self)
    }

    pub fn with_text_tokens(mut self, text_tokens: PromptResponseBoundaryTokens) -> Self {
        self.text_tokens = text_tokens;
        self
    }

    pub fn validate(&self) -> Result<(), SequenceBundleManifestError> {
        if self.manifest_version != PROMPT_RESPONSE_BOUNDARY_MANIFEST_VERSION {
            return Err(
                SequenceBundleManifestError::UnsupportedPromptResponseBoundaryVersion {
                    actual: self.manifest_version,
                    expected: PROMPT_RESPONSE_BOUNDARY_MANIFEST_VERSION,
                },
            );
        }
        if self.boundary_kind != PROMPT_TCT_RESPONSE_BOUNDARY_KIND {
            return Err(
                SequenceBundleManifestError::UnsupportedPromptResponseBoundaryKind {
                    actual: self.boundary_kind.clone(),
                    expected: PROMPT_TCT_RESPONSE_BOUNDARY_KIND,
                },
            );
        }
        if self.validity_scope != TCT_RESPONSE_SPAN_VALIDITY_SCOPE {
            return Err(
                SequenceBundleManifestError::UnsupportedPromptResponseValidityScope {
                    actual: self.validity_scope.clone(),
                    expected: TCT_RESPONSE_SPAN_VALIDITY_SCOPE,
                },
            );
        }
        if self.prompt_tokenizer.base_vocab_size == 0 {
            return Err(SequenceBundleManifestError::InvalidPromptTokenizerVocab {
                base_vocab_size: self.prompt_tokenizer.base_vocab_size,
            });
        }
        if self.prompt_tokenizer.first_available_token_id < self.prompt_tokenizer.base_vocab_size {
            return Err(
                SequenceBundleManifestError::InvalidPromptTokenizerFirstAvailableToken {
                    base_vocab_size: self.prompt_tokenizer.base_vocab_size,
                    first_available_token_id: self.prompt_tokenizer.first_available_token_id,
                },
            );
        }

        self.tct_bundle.validate()?;
        self.validate_hf_added_token_block()?;
        self.validate_text_tokens()
    }

    pub fn bundle_token_to_hf(
        &self,
        bundle_token: u32,
    ) -> Result<u32, SequenceBundleManifestError> {
        self.validate()?;
        self.checked_bundle_token_to_hf(bundle_token)
    }

    pub fn hf_token_to_bundle(&self, hf_token: u32) -> Result<u32, SequenceBundleManifestError> {
        self.validate()?;
        self.checked_hf_token_to_bundle(hf_token)
    }

    pub fn wrap_response_payload_for_schema_id(
        &self,
        schema_id: &str,
        local_payload: &[u32],
    ) -> Result<Vec<u32>, SequenceBundleManifestError> {
        self.validate()?;
        let bundle_tokens = self
            .tct_bundle
            .wrap_payload_for_schema_id(schema_id, local_payload)?;
        bundle_tokens
            .into_iter()
            .map(|token| self.checked_bundle_token_to_hf(token))
            .collect()
    }

    pub fn unwrap_response_tokens(
        &self,
        hf_response_tokens: &[u32],
    ) -> Result<SequenceBundlePayload, SequenceBundleManifestError> {
        self.validate()?;
        let bundle_tokens: Result<Vec<u32>, _> = hf_response_tokens
            .iter()
            .copied()
            .map(|token| self.checked_hf_token_to_bundle(token))
            .collect();
        self.tct_bundle.unwrap_payload(&bundle_tokens?)
    }

    pub fn wrap_handoff_response_for_schema_id(
        &self,
        schema_id: &str,
        local_payload: &[u32],
    ) -> Result<Vec<u32>, SequenceBundleManifestError> {
        self.validate()?;
        let response_tokens = self.wrap_response_payload_for_schema_id(schema_id, local_payload)?;
        let mut handoff_tokens = Vec::with_capacity(response_tokens.len() + 2);
        handoff_tokens.push(self.text_tokens.tct_start_token);
        handoff_tokens.extend(response_tokens);
        handoff_tokens.push(self.text_tokens.tct_end_token);
        Ok(handoff_tokens)
    }

    pub fn unwrap_handoff_response_tokens(
        &self,
        handoff_tokens: &[u32],
    ) -> Result<SequenceBundlePayload, SequenceBundleManifestError> {
        self.validate()?;
        if handoff_tokens.first().copied() != Some(self.text_tokens.tct_start_token) {
            return Err(SequenceBundleManifestError::InvalidTctModeStart {
                expected: self.text_tokens.tct_start_token,
                actual: handoff_tokens.first().copied(),
            });
        }
        if handoff_tokens.last().copied() != Some(self.text_tokens.tct_end_token) {
            return Err(SequenceBundleManifestError::InvalidTctModeEnd {
                expected: self.text_tokens.tct_end_token,
                actual: handoff_tokens.last().copied(),
            });
        }
        if handoff_tokens.len() < 3 {
            return Err(SequenceBundleManifestError::MissingTctResponseTokens);
        }
        self.unwrap_response_tokens(&handoff_tokens[1..handoff_tokens.len() - 1])
    }

    pub fn hf_generation_mask_for_schema_token(
        &self,
        schema_token: u32,
    ) -> Result<SequenceGenerationMask, SequenceBundleManifestError> {
        self.validate()?;
        let mask = self
            .tct_bundle
            .generation_mask_for_schema_token(schema_token)?;
        Ok(SequenceGenerationMask {
            schema_id: mask.schema_id,
            payload: TokenRange::new(
                self.hf_added_token_block.start + mask.payload.start,
                self.hf_added_token_block.start + mask.payload.end_exclusive,
            ),
            eos_token: self.checked_bundle_token_to_hf(mask.eos_token)?,
        })
    }

    pub fn direct_local_payload_profile_for_schema(
        &self,
        schema_id: &str,
        schema_sentinel_token: u32,
        local_payload_hf_start: u32,
    ) -> Result<DirectHfTctCompatibilityProfile, SequenceBundleManifestError> {
        self.validate()?;
        let schema = self.tct_bundle.schema_for_id(schema_id).ok_or_else(|| {
            SequenceBundleManifestError::UnknownSchemaId {
                schema_id: schema_id.to_string(),
            }
        })?;
        let schema_sentinel = schema.schema_sentinel.clone().ok_or_else(|| {
            SequenceBundleManifestError::SchemaSentinelRequiredForDirectProfile {
                schema_id: schema.schema_id.clone(),
            }
        })?;
        let hf_payload = TokenRange::new(
            local_payload_hf_start + schema.raw_payload.start,
            local_payload_hf_start + schema.raw_payload.end_exclusive,
        );
        let raw_pad_hf_token = local_payload_hf_start + schema.raw_pad_token;

        Ok(DirectHfTctCompatibilityProfile {
            profile_name: DIRECT_LOCAL_PAYLOAD_PROFILE_NAME.to_string(),
            profile_version: DIRECT_LOCAL_PAYLOAD_PROFILE_VERSION,
            handoff_shape: DIRECT_LOCAL_PAYLOAD_HANDOFF_SHAPE.to_string(),
            schema_id: schema.schema_id.clone(),
            schema_sentinel,
            schema_sentinel_token,
            tct_start_sentinel: self.text_tokens.tct_start_sentinel.clone(),
            tct_start_token: self.text_tokens.tct_start_token,
            tct_end_sentinel: self.text_tokens.tct_end_sentinel.clone(),
            tct_end_token: self.text_tokens.tct_end_token,
            tct_end_role: GLOBAL_HF_SPECIAL_TCT_END_ROLE.to_string(),
            local_payload: schema.raw_payload.clone(),
            hf_payload: hf_payload.clone(),
            raw_pad_token: schema.raw_pad_token,
            raw_pad_hf_token,
            raw_pad_is_payload: schema.raw_payload.contains(schema.raw_pad_token),
            masking: DirectHfTctMaskingPolicy::default(),
            generation_mask: SequenceGenerationMask {
                schema_id: schema.schema_id.clone(),
                payload: hf_payload,
                eos_token: self.text_tokens.tct_end_token,
            },
        })
    }

    fn validate_hf_added_token_block(&self) -> Result<(), SequenceBundleManifestError> {
        if self.hf_added_token_block.is_empty()
            || self.hf_added_token_block.start < self.prompt_tokenizer.first_available_token_id
            || self.hf_added_token_block.len() != self.tct_bundle.vocab_size
        {
            return Err(SequenceBundleManifestError::InvalidHfAddedTokenBlock {
                start: self.hf_added_token_block.start,
                end_exclusive: self.hf_added_token_block.end_exclusive,
                prompt_base_vocab_size: self.prompt_tokenizer.base_vocab_size,
                prompt_first_available_token_id: self.prompt_tokenizer.first_available_token_id,
                expected_len: self.tct_bundle.vocab_size,
            });
        }
        Ok(())
    }

    fn validate_text_tokens(&self) -> Result<(), SequenceBundleManifestError> {
        let mut text_tokens = BTreeSet::new();
        for (name, sentinel) in [
            (
                "tct_start_sentinel",
                self.text_tokens.tct_start_sentinel.as_str(),
            ),
            (
                "tct_end_sentinel",
                self.text_tokens.tct_end_sentinel.as_str(),
            ),
        ] {
            if sentinel.is_empty() {
                return Err(SequenceBundleManifestError::EmptyBoundarySentinel { name });
            }
            if !text_tokens.insert(sentinel) {
                return Err(SequenceBundleManifestError::DuplicateBoundarySentinel {
                    sentinel: sentinel.to_string(),
                });
            }
        }

        let mut token_ids = BTreeSet::new();
        for (name, token) in [
            ("tct_start_token", self.text_tokens.tct_start_token),
            ("tct_end_token", self.text_tokens.tct_end_token),
        ] {
            if token < self.prompt_tokenizer.first_available_token_id {
                return Err(
                    SequenceBundleManifestError::BoundaryTokenCollidesWithPromptTokenizer {
                        name,
                        token,
                        first_available_token_id: self.prompt_tokenizer.first_available_token_id,
                    },
                );
            }
            if self.hf_added_token_block.contains(token) {
                return Err(
                    SequenceBundleManifestError::BoundaryTokenCollidesWithTctBlock {
                        name,
                        token,
                        start: self.hf_added_token_block.start,
                        end_exclusive: self.hf_added_token_block.end_exclusive,
                    },
                );
            }
            if !token_ids.insert(token) {
                return Err(SequenceBundleManifestError::DuplicateBoundaryToken { token });
            }
        }

        for schema in &self.tct_bundle.schemas {
            if let Some(sentinel) = &schema.schema_sentinel
                && text_tokens.contains(sentinel.as_str())
            {
                return Err(
                    SequenceBundleManifestError::SchemaSentinelCollidesWithBoundaryToken {
                        schema_id: schema.schema_id.clone(),
                        sentinel: sentinel.clone(),
                    },
                );
            }
        }
        Ok(())
    }

    fn checked_bundle_token_to_hf(
        &self,
        bundle_token: u32,
    ) -> Result<u32, SequenceBundleManifestError> {
        if bundle_token >= self.tct_bundle.vocab_size {
            return Err(SequenceBundleManifestError::BundleTokenOutsideVocab {
                token: bundle_token,
                vocab_size: self.tct_bundle.vocab_size,
            });
        }
        Ok(self.hf_added_token_block.start + bundle_token)
    }

    fn checked_hf_token_to_bundle(
        &self,
        hf_token: u32,
    ) -> Result<u32, SequenceBundleManifestError> {
        if !self.hf_added_token_block.contains(hf_token) {
            return Err(SequenceBundleManifestError::HfTokenOutsideTctBlock {
                token: hf_token,
                start: self.hf_added_token_block.start,
                end_exclusive: self.hf_added_token_block.end_exclusive,
            });
        }
        Ok(hf_token - self.hf_added_token_block.start)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SequenceBundleManifestError {
    UnsupportedManifestVersion {
        actual: u32,
        expected: u32,
    },
    UnsupportedBoundaryVersion {
        actual: u32,
        expected: u32,
    },
    NoSchemas,
    AliasMismatch {
        alias: &'static str,
        alias_value: u32,
        canonical: &'static str,
        canonical_value: u32,
    },
    DuplicateSpecialTokens,
    TokenOutsideVocab {
        token: u32,
        vocab_size: u32,
    },
    UnsupportedMaskingPolicy,
    EmptySchemaId,
    DuplicateSchemaId {
        schema_id: String,
    },
    DuplicateSchemaToken {
        schema_token: u32,
    },
    SchemaTokenCollidesWithSpecial {
        schema_id: String,
        schema_token: u32,
    },
    EmptySchemaSentinel {
        schema_id: String,
    },
    DuplicateSchemaSentinel {
        sentinel: String,
    },
    InvalidRawVocab {
        schema_id: String,
        raw_vocab_size: u32,
        raw_pad_token: u32,
    },
    InvalidPayloadRange {
        schema_id: String,
        range_name: &'static str,
        start: u32,
        end_exclusive: u32,
    },
    PayloadRangeCollidesWithReservedToken {
        schema_id: String,
        token: u32,
    },
    UnknownSchemaToken {
        schema_token: u32,
    },
    UnknownSchemaId {
        schema_id: String,
    },
    InvalidSequenceStart {
        expected: u32,
        actual: Option<u32>,
    },
    MissingSchemaToken {
        expected_position: usize,
    },
    MissingEosToken {
        schema_token: u32,
    },
    NonPaddingAfterEos {
        token: u32,
        offset: usize,
        expected_pad: u32,
    },
    InvalidLocalPayloadToken {
        schema_id: String,
        token: u32,
        offset: usize,
    },
    InvalidGlobalPayloadToken {
        schema_id: String,
        token: u32,
        offset: usize,
    },
    UnsupportedPromptResponseBoundaryVersion {
        actual: u32,
        expected: u32,
    },
    UnsupportedPromptResponseBoundaryKind {
        actual: String,
        expected: &'static str,
    },
    UnsupportedPromptResponseValidityScope {
        actual: String,
        expected: &'static str,
    },
    InvalidPromptTokenizerVocab {
        base_vocab_size: u32,
    },
    InvalidPromptTokenizerFirstAvailableToken {
        base_vocab_size: u32,
        first_available_token_id: u32,
    },
    InvalidHfAddedTokenBlock {
        start: u32,
        end_exclusive: u32,
        prompt_base_vocab_size: u32,
        prompt_first_available_token_id: u32,
        expected_len: u32,
    },
    EmptyBoundarySentinel {
        name: &'static str,
    },
    DuplicateBoundarySentinel {
        sentinel: String,
    },
    BoundaryTokenCollidesWithPromptTokenizer {
        name: &'static str,
        token: u32,
        first_available_token_id: u32,
    },
    BoundaryTokenCollidesWithTctBlock {
        name: &'static str,
        token: u32,
        start: u32,
        end_exclusive: u32,
    },
    DuplicateBoundaryToken {
        token: u32,
    },
    SchemaSentinelCollidesWithBoundaryToken {
        schema_id: String,
        sentinel: String,
    },
    SchemaSentinelRequiredForDirectProfile {
        schema_id: String,
    },
    UnsupportedDirectHfTctProfileName {
        actual: String,
        expected: &'static str,
    },
    UnsupportedDirectHfTctProfileVersion {
        actual: u32,
        expected: u32,
    },
    UnsupportedDirectHfTctHandoffShape {
        actual: String,
        expected: &'static str,
    },
    UnsupportedDirectHfTctEndRole {
        actual: String,
        expected: &'static str,
    },
    EmptyDirectHfTctProfileField {
        field: &'static str,
    },
    InvalidDirectHfTctPayloadRange {
        schema_id: String,
        local_start: u32,
        local_end_exclusive: u32,
        hf_start: u32,
        hf_end_exclusive: u32,
    },
    InvalidDirectHfTctRawPadPolicy {
        schema_id: String,
        raw_pad_token: u32,
        raw_pad_is_payload: bool,
    },
    InvalidDirectHfTctRawPadOffset {
        schema_id: String,
        raw_pad_token: u32,
        local_payload_start: u32,
        hf_payload_start: u32,
    },
    InvalidDirectHfTctRawPadHfToken {
        schema_id: String,
        actual: u32,
        expected: u32,
    },
    UnsupportedDirectHfTctMaskingPolicy,
    InvalidDirectHfTctGenerationMask {
        schema_id: String,
    },
    BundleTokenOutsideVocab {
        token: u32,
        vocab_size: u32,
    },
    HfTokenOutsideTctBlock {
        token: u32,
        start: u32,
        end_exclusive: u32,
    },
    InvalidTctModeStart {
        expected: u32,
        actual: Option<u32>,
    },
    InvalidTctModeEnd {
        expected: u32,
        actual: Option<u32>,
    },
    MissingTctResponseTokens,
}

impl fmt::Display for SequenceBundleManifestError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnsupportedManifestVersion { actual, expected } => write!(
                f,
                "unsupported sequence bundle manifest version {}; expected {}",
                actual, expected
            ),
            Self::UnsupportedBoundaryVersion { actual, expected } => write!(
                f,
                "unsupported sequence boundary version {}; expected {}",
                actual, expected
            ),
            Self::NoSchemas => write!(
                f,
                "sequence bundle manifest must contain at least one schema"
            ),
            Self::AliasMismatch {
                alias,
                alias_value,
                canonical,
                canonical_value,
            } => write!(
                f,
                "{}={} must match {}={}",
                alias, alias_value, canonical, canonical_value
            ),
            Self::DuplicateSpecialTokens => {
                write!(f, "sequence PAD, BOS, and EOS tokens must be distinct")
            }
            Self::TokenOutsideVocab { token, vocab_size } => {
                write!(
                    f,
                    "token {} is outside bundle vocab size {}",
                    token, vocab_size
                )
            }
            Self::UnsupportedMaskingPolicy => {
                write!(f, "unsupported sequence bundle masking policy")
            }
            Self::EmptySchemaId => write!(f, "schema id must be non-empty"),
            Self::DuplicateSchemaId { schema_id } => {
                write!(f, "duplicate schema id {}", schema_id)
            }
            Self::DuplicateSchemaToken { schema_token } => {
                write!(f, "duplicate schema token {}", schema_token)
            }
            Self::SchemaTokenCollidesWithSpecial {
                schema_id,
                schema_token,
            } => write!(
                f,
                "schema {} token {} collides with a sequence special token",
                schema_id, schema_token
            ),
            Self::EmptySchemaSentinel { schema_id } => write!(
                f,
                "schema {} sentinel must be non-empty when present",
                schema_id
            ),
            Self::DuplicateSchemaSentinel { sentinel } => {
                write!(f, "duplicate schema sentinel {}", sentinel)
            }
            Self::InvalidRawVocab {
                schema_id,
                raw_vocab_size,
                raw_pad_token,
            } => write!(
                f,
                "schema {} has invalid raw vocab size {} and raw PAD token {}",
                schema_id, raw_vocab_size, raw_pad_token
            ),
            Self::InvalidPayloadRange {
                schema_id,
                range_name,
                start,
                end_exclusive,
            } => write!(
                f,
                "schema {} has invalid {} range [{}, {})",
                schema_id, range_name, start, end_exclusive
            ),
            Self::PayloadRangeCollidesWithReservedToken { schema_id, token } => write!(
                f,
                "schema {} payload range collides with reserved token {}",
                schema_id, token
            ),
            Self::UnknownSchemaToken { schema_token } => {
                write!(f, "unknown schema token {}", schema_token)
            }
            Self::UnknownSchemaId { schema_id } => {
                write!(f, "unknown schema id {}", schema_id)
            }
            Self::InvalidSequenceStart { expected, actual } => write!(
                f,
                "sequence must start with BOS token {}; got {:?}",
                expected, actual
            ),
            Self::MissingSchemaToken { expected_position } => write!(
                f,
                "sequence is missing schema token at position {}",
                expected_position
            ),
            Self::MissingEosToken { schema_token } => {
                write!(
                    f,
                    "sequence for schema token {} is missing EOS",
                    schema_token
                )
            }
            Self::NonPaddingAfterEos {
                token,
                offset,
                expected_pad,
            } => write!(
                f,
                "token {} at sequence offset {} after EOS is not expected PAD {}",
                token, offset, expected_pad
            ),
            Self::InvalidLocalPayloadToken {
                schema_id,
                token,
                offset,
            } => write!(
                f,
                "schema {} local payload token {} at offset {} is outside raw payload range",
                schema_id, token, offset
            ),
            Self::InvalidGlobalPayloadToken {
                schema_id,
                token,
                offset,
            } => write!(
                f,
                "schema {} global payload token {} at offset {} is outside selected schema range",
                schema_id, token, offset
            ),
            Self::UnsupportedPromptResponseBoundaryVersion { actual, expected } => write!(
                f,
                "unsupported prompt/TCT boundary manifest version {}; expected {}",
                actual, expected
            ),
            Self::UnsupportedPromptResponseBoundaryKind { actual, expected } => write!(
                f,
                "unsupported prompt/TCT boundary kind {}; expected {}",
                actual, expected
            ),
            Self::UnsupportedPromptResponseValidityScope { actual, expected } => write!(
                f,
                "unsupported prompt/TCT validity scope {}; expected {}",
                actual, expected
            ),
            Self::InvalidPromptTokenizerVocab { base_vocab_size } => write!(
                f,
                "prompt tokenizer base vocabulary size must be nonzero; got {}",
                base_vocab_size
            ),
            Self::InvalidPromptTokenizerFirstAvailableToken {
                base_vocab_size,
                first_available_token_id,
            } => write!(
                f,
                "prompt tokenizer first available token {} must be at or after base vocab {}",
                first_available_token_id, base_vocab_size
            ),
            Self::InvalidHfAddedTokenBlock {
                start,
                end_exclusive,
                prompt_base_vocab_size: _,
                prompt_first_available_token_id,
                expected_len,
            } => write!(
                f,
                "HF added-token block [{}, {}) must start at or after prompt first available token {} and have length {}",
                start, end_exclusive, prompt_first_available_token_id, expected_len
            ),
            Self::EmptyBoundarySentinel { name } => write!(f, "{} must be non-empty", name),
            Self::DuplicateBoundarySentinel { sentinel } => {
                write!(f, "duplicate prompt/TCT boundary sentinel {}", sentinel)
            }
            Self::BoundaryTokenCollidesWithPromptTokenizer {
                name,
                token,
                first_available_token_id,
            } => write!(
                f,
                "{} token {} collides with occupied prompt tokenizer IDs below first available token {}",
                name, token, first_available_token_id
            ),
            Self::BoundaryTokenCollidesWithTctBlock {
                name,
                token,
                start,
                end_exclusive,
            } => write!(
                f,
                "{} token {} collides with TCT added-token block [{}, {})",
                name, token, start, end_exclusive
            ),
            Self::DuplicateBoundaryToken { token } => {
                write!(f, "duplicate prompt/TCT boundary token {}", token)
            }
            Self::SchemaSentinelCollidesWithBoundaryToken {
                schema_id,
                sentinel,
            } => write!(
                f,
                "schema {} sentinel {} collides with a prompt/TCT boundary token",
                schema_id, sentinel
            ),
            Self::SchemaSentinelRequiredForDirectProfile { schema_id } => write!(
                f,
                "schema {} requires a schema sentinel for the direct HF/TCT profile",
                schema_id
            ),
            Self::UnsupportedDirectHfTctProfileName { actual, expected } => write!(
                f,
                "unsupported direct HF/TCT profile name {}; expected {}",
                actual, expected
            ),
            Self::UnsupportedDirectHfTctProfileVersion { actual, expected } => write!(
                f,
                "unsupported direct HF/TCT profile version {}; expected {}",
                actual, expected
            ),
            Self::UnsupportedDirectHfTctHandoffShape { actual, expected } => write!(
                f,
                "unsupported direct HF/TCT handoff shape {}; expected {}",
                actual, expected
            ),
            Self::UnsupportedDirectHfTctEndRole { actual, expected } => write!(
                f,
                "unsupported direct HF/TCT end role {}; expected {}",
                actual, expected
            ),
            Self::EmptyDirectHfTctProfileField { field } => {
                write!(f, "direct HF/TCT profile field {} must be non-empty", field)
            }
            Self::InvalidDirectHfTctPayloadRange {
                schema_id,
                local_start,
                local_end_exclusive,
                hf_start,
                hf_end_exclusive,
            } => write!(
                f,
                "direct HF/TCT profile for schema {} has invalid local payload [{}, {}) and HF payload [{}, {})",
                schema_id, local_start, local_end_exclusive, hf_start, hf_end_exclusive
            ),
            Self::InvalidDirectHfTctRawPadPolicy {
                schema_id,
                raw_pad_token,
                raw_pad_is_payload,
            } => write!(
                f,
                "direct HF/TCT profile for schema {} must exclude raw PAD token {}; raw_pad_is_payload={}",
                schema_id, raw_pad_token, raw_pad_is_payload
            ),
            Self::InvalidDirectHfTctRawPadOffset {
                schema_id,
                raw_pad_token,
                local_payload_start,
                hf_payload_start,
            } => write!(
                f,
                "direct HF/TCT profile for schema {} has invalid raw PAD offset raw_pad_token={} local_payload_start={} hf_payload_start={}",
                schema_id, raw_pad_token, local_payload_start, hf_payload_start
            ),
            Self::InvalidDirectHfTctRawPadHfToken {
                schema_id,
                actual,
                expected,
            } => write!(
                f,
                "direct HF/TCT profile for schema {} maps raw PAD to HF token {}; expected {}",
                schema_id, actual, expected
            ),
            Self::UnsupportedDirectHfTctMaskingPolicy => {
                write!(f, "unsupported direct HF/TCT masking policy")
            }
            Self::InvalidDirectHfTctGenerationMask { schema_id } => write!(
                f,
                "direct HF/TCT profile for schema {} has an invalid generation mask",
                schema_id
            ),
            Self::BundleTokenOutsideVocab { token, vocab_size } => write!(
                f,
                "bundle token {} is outside TCT bundle vocab size {}",
                token, vocab_size
            ),
            Self::HfTokenOutsideTctBlock {
                token,
                start,
                end_exclusive,
            } => write!(
                f,
                "HF token {} is outside TCT added-token block [{}, {})",
                token, start, end_exclusive
            ),
            Self::InvalidTctModeStart { expected, actual } => write!(
                f,
                "handoff response must start with TCT mode token {}; got {:?}",
                expected, actual
            ),
            Self::InvalidTctModeEnd { expected, actual } => write!(
                f,
                "handoff response must end with TCT mode token {}; got {:?}",
                expected, actual
            ),
            Self::MissingTctResponseTokens => write!(
                f,
                "handoff response is missing TCT response tokens between mode start and mode end"
            ),
        }
    }
}

impl std::error::Error for SequenceBundleManifestError {}

#[cfg(test)]
mod tests {
    use super::*;

    fn two_schema_manifest() -> SequenceBundleManifest {
        SequenceBundleManifest::new(
            55,
            vec![
                SequenceBundleSchema::new("kubernetes", 3, 18, 17, 5, true)
                    .with_schema_sentinel("<|schema:kubernetes|>")
                    .with_generated_artifact("wheels/tct_kubernetes.whl"),
                SequenceBundleSchema::new("tsconfig", 4, 12, 11, 33, false)
                    .with_schema_sentinel("<|schema:tsconfig|>")
                    .with_generated_artifact("wheels/tct_tsconfig.whl"),
            ],
        )
    }

    #[test]
    fn special_tokens_constructor_preserves_start_end_aliases() {
        let tokens = SequenceBundleSpecialTokens::new(10, 11, 12);
        assert_eq!(tokens.pad_token, 10);
        assert_eq!(tokens.bos_token, 11);
        assert_eq!(tokens.start_token, tokens.bos_token);
        assert_eq!(tokens.eos_token, 12);
        assert_eq!(tokens.end_token, tokens.eos_token);
        assert_eq!(
            SequenceBundleSpecialTokens::default(),
            SequenceBundleSpecialTokens::new(0, 1, 2)
        );

        let mut manifest = SequenceBundleManifest::new(
            64,
            vec![SequenceBundleSchema::new("alpha", 13, 8, 7, 20, false)],
        );
        manifest.special_tokens = tokens;
        manifest.validate().unwrap();
    }

    #[test]
    fn sequence_bundle_errors_use_manual_display_messages() {
        let duplicate = SequenceBundleManifestError::DuplicateSchemaToken { schema_token: 7 };
        assert_eq!(duplicate.to_string(), "duplicate schema token 7");
        assert!(std::error::Error::source(&duplicate).is_none());

        let invalid_start = SequenceBundleManifestError::InvalidSequenceStart {
            expected: 1,
            actual: None,
        };
        assert_eq!(
            invalid_start.to_string(),
            "sequence must start with BOS token 1; got None"
        );

        let raw_pad = SequenceBundleManifestError::InvalidDirectHfTctRawPadOffset {
            schema_id: "kubernetes".to_string(),
            raw_pad_token: 0,
            local_payload_start: 1,
            hf_payload_start: 32000,
        };
        assert!(
            raw_pad
                .to_string()
                .contains("direct HF/TCT profile for schema kubernetes has invalid raw PAD offset")
        );
    }

    #[test]
    fn validates_two_schema_manifest_and_generation_mask() {
        let manifest = two_schema_manifest();
        manifest.validate().unwrap();

        let kubernetes = manifest.schema_for_id("kubernetes").unwrap();
        assert_eq!(kubernetes.local_to_global(0), Some(5));
        assert_eq!(kubernetes.local_to_global(16), Some(21));
        assert_eq!(kubernetes.local_to_global(17), None);
        assert_eq!(kubernetes.global_to_local(5), Some(0));
        assert_eq!(kubernetes.global_to_local(21), Some(16));
        assert_eq!(kubernetes.global_to_local(22), None);

        let mask = manifest.generation_mask_for_schema_token(3).unwrap();
        assert_eq!(mask.schema_id, "kubernetes");
        assert!(mask.allows(5));
        assert!(mask.allows(21));
        assert!(mask.allows(manifest.special_tokens.eos_token));
        assert!(!mask.allows(4));
        assert!(!mask.allows(22));
    }

    #[test]
    fn serializes_bundle_contract_with_masking_policy() {
        let manifest = two_schema_manifest();
        let value = serde_json::to_value(&manifest).unwrap();

        assert_eq!(value["manifest_version"], SEQUENCE_BUNDLE_MANIFEST_VERSION);
        assert_eq!(
            value["sequence_boundary_version"],
            CURRENT_SEQUENCE_BOUNDARY_VERSION
        );
        assert_eq!(
            value["masking"]["generation_allowed_tokens"],
            SELECTED_SCHEMA_PAYLOAD_PLUS_EOS
        );
        assert_eq!(value["masking"]["mask_eos_token_from_labels"], false);
        assert_eq!(value["schemas"][0]["schema_id"], "kubernetes");
        assert_eq!(value["schemas"][0]["global_payload"]["start"], 5);
    }

    #[test]
    fn manifest_hashes_cover_bundle_and_prompt_boundary_metadata() {
        let manifest = two_schema_manifest();
        let manifest_hash = manifest.manifest_sha256().unwrap();
        assert_eq!(
            manifest_hash,
            crate::provenance::json_sha256(&manifest).unwrap()
        );
        assert_eq!(manifest_hash.len(), 64);

        let changed_manifest = SequenceBundleManifest::new(
            55,
            vec![
                SequenceBundleSchema::new("kubernetes", 3, 18, 17, 5, true)
                    .with_schema_sentinel("<|schema:kubernetes-v2|>"),
                SequenceBundleSchema::new("tsconfig", 4, 12, 11, 33, false)
                    .with_schema_sentinel("<|schema:tsconfig|>"),
            ],
        );
        assert_ne!(manifest_hash, changed_manifest.manifest_sha256().unwrap());

        let boundary = PromptResponseBoundaryManifest::new(
            PromptTokenizerMetadata::new(100).with_first_available_token_id(104),
            PromptResponseBoundaryTokens::new(104, 105),
            3_000,
            manifest,
        );
        boundary.validate().unwrap();
        let boundary_hash = boundary.manifest_sha256().unwrap();
        assert_eq!(
            boundary_hash,
            crate::provenance::json_sha256(&boundary).unwrap()
        );
        assert_eq!(boundary_hash.len(), 64);
        assert_ne!(boundary_hash, changed_manifest.manifest_sha256().unwrap());
    }

    #[test]
    fn rejects_schema_token_and_payload_collisions() {
        let mut manifest = two_schema_manifest();
        manifest.schemas[1].schema_token = manifest.schemas[0].schema_token;
        assert!(matches!(
            manifest.validate(),
            Err(SequenceBundleManifestError::DuplicateSchemaToken { .. })
        ));

        let mut manifest = two_schema_manifest();
        manifest.schemas[0].global_payload = TokenRange::new(2, 19);
        assert!(matches!(
            manifest.validate(),
            Err(SequenceBundleManifestError::PayloadRangeCollidesWithReservedToken { .. })
        ));
    }

    #[test]
    fn rejects_raw_pad_as_payload_and_unsupported_masking() {
        let mut manifest = two_schema_manifest();
        manifest.schemas[0].raw_payload.end_exclusive = manifest.schemas[0].raw_pad_token + 1;
        assert!(matches!(
            manifest.validate(),
            Err(SequenceBundleManifestError::InvalidPayloadRange { .. })
        ));

        let mut manifest = two_schema_manifest();
        manifest.masking.mask_eos_token_from_labels = true;
        assert!(matches!(
            manifest.validate(),
            Err(SequenceBundleManifestError::UnsupportedMaskingPolicy)
        ));
    }

    #[test]
    fn wraps_and_unwraps_payloads_for_two_schemas() {
        let manifest = two_schema_manifest();

        let kubernetes = manifest
            .wrap_payload_for_schema_id("kubernetes", &[0, 16])
            .unwrap();
        assert_eq!(kubernetes, vec![1, 3, 5, 21, 2]);
        let kubernetes_payload = manifest.unwrap_payload(&kubernetes).unwrap();
        assert_eq!(kubernetes_payload.schema_id, "kubernetes");
        assert_eq!(kubernetes_payload.schema_token, 3);
        assert_eq!(kubernetes_payload.local_payload, vec![0, 16]);
        assert_eq!(kubernetes_payload.global_payload, vec![5, 21]);
        assert_eq!(kubernetes_payload.consumed_tokens, 5);
        assert_eq!(kubernetes_payload.total_tokens, 5);

        let mut tsconfig = manifest.wrap_payload_for_schema_token(4, &[0, 10]).unwrap();
        tsconfig.extend([0, 0]);
        assert_eq!(tsconfig, vec![1, 4, 33, 43, 2, 0, 0]);
        let tsconfig_payload = manifest.unwrap_payload(&tsconfig).unwrap();
        assert_eq!(tsconfig_payload.schema_id, "tsconfig");
        assert_eq!(tsconfig_payload.local_payload, vec![0, 10]);
        assert_eq!(tsconfig_payload.global_payload, vec![33, 43]);
        assert_eq!(tsconfig_payload.consumed_tokens, 5);
        assert_eq!(tsconfig_payload.total_tokens, 7);
    }

    #[test]
    fn rejects_invalid_bundle_sequences() {
        let manifest = two_schema_manifest();

        assert!(matches!(
            manifest.unwrap_payload(&[1, 99, 2]),
            Err(SequenceBundleManifestError::UnknownSchemaToken { .. })
        ));
        assert!(matches!(
            manifest.unwrap_payload(&[1, 3, 5, 6]),
            Err(SequenceBundleManifestError::MissingEosToken { .. })
        ));
        assert!(matches!(
            manifest.unwrap_payload(&[1]),
            Err(SequenceBundleManifestError::MissingSchemaToken { .. })
        ));
        assert!(matches!(
            manifest.unwrap_payload(&[0, 3, 5, 2]),
            Err(SequenceBundleManifestError::InvalidSequenceStart { .. })
        ));
        assert!(matches!(
            manifest.unwrap_payload(&[1, 3, 5, 2, 7]),
            Err(SequenceBundleManifestError::NonPaddingAfterEos { .. })
        ));
        assert!(matches!(
            manifest.wrap_payload_for_schema_id("kubernetes", &[17]),
            Err(SequenceBundleManifestError::InvalidLocalPayloadToken { .. })
        ));
        assert!(matches!(
            manifest.unwrap_payload(&[1, 3, 33, 2]),
            Err(SequenceBundleManifestError::InvalidGlobalPayloadToken { .. })
        ));
    }

    #[test]
    fn supports_huggingface_style_shared_global_payload_block() {
        let manifest = SequenceBundleManifest::new(
            122,
            vec![
                SequenceBundleSchema::new("kubernetes", 3, 18, 17, 100, true)
                    .with_schema_sentinel("<|schema:kubernetes|>"),
                SequenceBundleSchema::new("tsconfig", 4, 12, 11, 100, false)
                    .with_schema_sentinel("<|schema:tsconfig|>"),
            ],
        );

        manifest.validate().unwrap();
        let kubernetes = manifest
            .wrap_payload_for_schema_id("kubernetes", &[0, 16])
            .unwrap();
        let tsconfig = manifest
            .wrap_payload_for_schema_id("tsconfig", &[0, 10])
            .unwrap();
        assert_eq!(kubernetes, vec![1, 3, 100, 116, 2]);
        assert_eq!(tsconfig, vec![1, 4, 100, 110, 2]);

        let kubernetes_payload = manifest.unwrap_payload(&kubernetes).unwrap();
        let tsconfig_payload = manifest.unwrap_payload(&tsconfig).unwrap();
        assert_eq!(kubernetes_payload.local_payload, vec![0, 16]);
        assert_eq!(tsconfig_payload.local_payload, vec![0, 10]);

        let kubernetes_mask = manifest.generation_mask_for_schema_token(3).unwrap();
        let tsconfig_mask = manifest.generation_mask_for_schema_token(4).unwrap();
        assert!(kubernetes_mask.allows(116));
        assert!(!tsconfig_mask.allows(116));
        assert!(tsconfig_mask.allows(110));

        let value = serde_json::to_value(&manifest).unwrap();
        assert_eq!(
            value["schemas"][0]["schema_sentinel"],
            "<|schema:kubernetes|>"
        );
        assert_eq!(value["schemas"][0]["global_payload"]["start"], 100);
        assert_eq!(value["schemas"][1]["global_payload"]["start"], 100);
    }

    #[test]
    fn nanochat_direct_payload_profile_excludes_raw_pad() {
        let schema = SequenceBundleSchema::new("kubernetes", 3, 18, 17, 5, true)
            .with_schema_sentinel("<|schema:kubernetes|>");
        let manifest = SequenceBundleManifest::new(22, vec![schema.clone()]);
        manifest.validate().unwrap();

        let tct_token_start = 2_000;
        let tct_start_token = 104;
        let tct_end_token = 105;
        let schema_sentinel_token = 106;

        let direct_proto_tokens = vec![
            schema_sentinel_token,
            tct_start_token,
            tct_token_start,
            tct_token_start + schema.raw_pad_token - 1,
            tct_end_token,
        ];
        assert_eq!(direct_proto_tokens, vec![106, 104, 2000, 2016, 105]);

        let direct_proto_bad_raw_pad = tct_token_start + schema.raw_pad_token;
        let raw_payload_hf_range = TokenRange::new(
            tct_token_start + schema.raw_payload.start,
            tct_token_start + schema.raw_payload.end_exclusive,
        );
        assert!(raw_payload_hf_range.contains(tct_token_start));
        assert!(raw_payload_hf_range.contains(tct_token_start + schema.raw_pad_token - 1));
        assert!(!raw_payload_hf_range.contains(direct_proto_bad_raw_pad));

        let direct_proto_generation_mask = SequenceGenerationMask {
            schema_id: schema.schema_id.clone(),
            payload: raw_payload_hf_range,
            eos_token: tct_end_token,
        };
        assert!(direct_proto_generation_mask.allows(tct_token_start));
        assert!(direct_proto_generation_mask.allows(tct_token_start + schema.raw_pad_token - 1));
        assert!(direct_proto_generation_mask.allows(tct_end_token));
        assert!(!direct_proto_generation_mask.allows(direct_proto_bad_raw_pad));
        assert!(!direct_proto_generation_mask.allows(schema_sentinel_token));

        assert_eq!(schema.local_to_global(schema.raw_pad_token - 1), Some(21));
        assert_eq!(schema.local_to_global(schema.raw_pad_token), None);
        assert!(matches!(
            manifest.wrap_payload_for_schema_id("kubernetes", &[schema.raw_pad_token]),
            Err(SequenceBundleManifestError::InvalidLocalPayloadToken { .. })
        ));

        let canonical_payload = manifest
            .wrap_payload_for_schema_id("kubernetes", &[0, schema.raw_pad_token - 1])
            .unwrap();
        assert_eq!(canonical_payload, vec![1, 3, 5, 21, 2]);

        let boundary = PromptResponseBoundaryManifest::new(
            PromptTokenizerMetadata::new(100).with_first_available_token_id(104),
            PromptResponseBoundaryTokens::new(tct_start_token, tct_end_token),
            3_000,
            manifest,
        );
        boundary.validate().unwrap();
        let canonical_handoff = boundary
            .wrap_handoff_response_for_schema_id("kubernetes", &[0, schema.raw_pad_token - 1])
            .unwrap();
        assert_eq!(
            canonical_handoff,
            vec![104, 3001, 3003, 3005, 3021, 3002, 105]
        );
        assert_ne!(
            direct_proto_tokens, canonical_handoff,
            "the current nanochat prototype is a direct local-payload handoff, not the canonical bundle sequence"
        );

        let profile = boundary
            .direct_local_payload_profile_for_schema(
                "kubernetes",
                schema_sentinel_token,
                tct_token_start,
            )
            .unwrap();
        profile.validate().unwrap();
        assert_eq!(profile.profile_name, DIRECT_LOCAL_PAYLOAD_PROFILE_NAME);
        assert_eq!(
            profile.profile_version,
            DIRECT_LOCAL_PAYLOAD_PROFILE_VERSION
        );
        assert_eq!(profile.handoff_shape, DIRECT_LOCAL_PAYLOAD_HANDOFF_SHAPE);
        assert_eq!(profile.schema_sentinel, "<|schema:kubernetes|>");
        assert_eq!(profile.schema_sentinel_token, schema_sentinel_token);
        assert_eq!(
            profile.local_payload,
            TokenRange::new(0, schema.raw_pad_token)
        );
        assert_eq!(
            profile.hf_payload,
            TokenRange::new(tct_token_start, tct_token_start + schema.raw_pad_token)
        );
        assert_eq!(profile.raw_pad_token, schema.raw_pad_token);
        assert_eq!(profile.raw_pad_hf_token, direct_proto_bad_raw_pad);
        assert!(!profile.raw_pad_is_payload);
        assert_eq!(profile.tct_end_role, GLOBAL_HF_SPECIAL_TCT_END_ROLE);
        assert!(profile.generation_mask.allows(tct_token_start));
        assert!(
            profile
                .generation_mask
                .allows(tct_token_start + schema.raw_pad_token - 1)
        );
        assert!(profile.generation_mask.allows(tct_end_token));
        assert!(!profile.generation_mask.allows(direct_proto_bad_raw_pad));
        assert!(!profile.generation_mask.allows(schema_sentinel_token));

        let value = serde_json::to_value(&profile).unwrap();
        assert_eq!(value["profile_name"], DIRECT_LOCAL_PAYLOAD_PROFILE_NAME);
        assert_eq!(value["tct_end_role"], GLOBAL_HF_SPECIAL_TCT_END_ROLE);

        let profile_hash = profile.profile_sha256().unwrap();
        assert_eq!(profile_hash.len(), 64);
        let mut changed_profile = profile.clone();
        changed_profile.generation_mask.eos_token += 1;
        assert_ne!(
            profile_hash,
            changed_profile.profile_sha256().unwrap(),
            "profile hash must cover generation-mask metadata"
        );
    }

    #[test]
    fn direct_local_payload_profile_validation_rejects_stale_metadata() {
        let schema = SequenceBundleSchema::new("kubernetes", 3, 18, 17, 5, true)
            .with_schema_sentinel("<|schema:kubernetes|>");
        let manifest = SequenceBundleManifest::new(22, vec![schema]);
        let boundary = PromptResponseBoundaryManifest::new(
            PromptTokenizerMetadata::new(100).with_first_available_token_id(104),
            PromptResponseBoundaryTokens::new(104, 105),
            3_000,
            manifest,
        );
        let profile = boundary
            .direct_local_payload_profile_for_schema("kubernetes", 106, 2_000)
            .unwrap();

        let mut invalid = profile.clone();
        invalid.profile_name = "legacy_direct_payload".to_string();
        assert!(matches!(
            invalid.validate(),
            Err(SequenceBundleManifestError::UnsupportedDirectHfTctProfileName { .. })
        ));

        let mut invalid = profile.clone();
        invalid.profile_version = DIRECT_LOCAL_PAYLOAD_PROFILE_VERSION + 1;
        assert!(matches!(
            invalid.validate(),
            Err(SequenceBundleManifestError::UnsupportedDirectHfTctProfileVersion { .. })
        ));

        let mut invalid = profile.clone();
        invalid.raw_pad_is_payload = true;
        assert!(matches!(
            invalid.validate(),
            Err(SequenceBundleManifestError::InvalidDirectHfTctRawPadPolicy { .. })
        ));

        let mut invalid = profile.clone();
        invalid.raw_pad_hf_token += 1;
        assert!(matches!(
            invalid.validate(),
            Err(SequenceBundleManifestError::InvalidDirectHfTctRawPadHfToken { .. })
        ));

        let mut invalid = profile;
        invalid.generation_mask.eos_token += 1;
        assert!(matches!(
            invalid.validate(),
            Err(SequenceBundleManifestError::InvalidDirectHfTctGenerationMask { .. })
        ));
    }

    #[test]
    fn prompt_response_boundary_maps_bundle_tokens_into_hf_added_block() {
        let boundary = PromptResponseBoundaryManifest::new(
            PromptTokenizerMetadata::new(100)
                .with_tokenizer_id("demo-pretrained-bpe")
                .with_first_available_token_id(104),
            PromptResponseBoundaryTokens::new(104, 105),
            200,
            two_schema_manifest(),
        );

        boundary.validate().unwrap();
        assert_eq!(boundary.bundle_token_to_hf(0).unwrap(), 200);
        assert_eq!(boundary.bundle_token_to_hf(54).unwrap(), 254);
        assert_eq!(boundary.hf_token_to_bundle(221).unwrap(), 21);

        let response = boundary
            .wrap_response_payload_for_schema_id("kubernetes", &[0, 16])
            .unwrap();
        assert_eq!(response, vec![201, 203, 205, 221, 202]);

        let payload = boundary.unwrap_response_tokens(&response).unwrap();
        assert_eq!(payload.schema_id, "kubernetes");
        assert_eq!(payload.schema_token, 3);
        assert_eq!(payload.local_payload, vec![0, 16]);
        assert_eq!(payload.global_payload, vec![5, 21]);

        let handoff = boundary
            .wrap_handoff_response_for_schema_id("kubernetes", &[0, 16])
            .unwrap();
        assert_eq!(handoff, vec![104, 201, 203, 205, 221, 202, 105]);
        assert_eq!(
            boundary
                .unwrap_handoff_response_tokens(&handoff)
                .unwrap()
                .local_payload,
            vec![0, 16]
        );

        let mask = boundary.hf_generation_mask_for_schema_token(3).unwrap();
        assert_eq!(mask.schema_id, "kubernetes");
        assert!(mask.allows(205));
        assert!(mask.allows(221));
        assert!(mask.allows(202));
        assert!(!mask.allows(203));
        assert!(!mask.allows(222));

        let value = serde_json::to_value(&boundary).unwrap();
        assert_eq!(value["manifest_version"], 1);
        assert_eq!(value["boundary_kind"], PROMPT_TCT_RESPONSE_BOUNDARY_KIND);
        assert_eq!(value["validity_scope"], TCT_RESPONSE_SPAN_VALIDITY_SCOPE);
        assert_eq!(value["prompt_tokenizer"]["base_vocab_size"], 100);
        assert_eq!(value["prompt_tokenizer"]["first_available_token_id"], 104);
        assert_eq!(value["text_tokens"]["tct_start_token"], 104);
        assert_eq!(value["text_tokens"]["tct_end_token"], 105);
        assert_eq!(value["hf_added_token_block"]["start"], 200);
        assert_eq!(value["tct_bundle"]["schemas"][0]["schema_id"], "kubernetes");
    }

    #[test]
    fn prompt_response_boundary_rejects_added_block_and_token_collisions() {
        let boundary = PromptResponseBoundaryManifest::new(
            PromptTokenizerMetadata::new(100).with_first_available_token_id(150),
            PromptResponseBoundaryTokens::new(150, 151),
            140,
            two_schema_manifest(),
        );
        assert!(matches!(
            boundary.validate(),
            Err(SequenceBundleManifestError::InvalidHfAddedTokenBlock { .. })
        ));

        let boundary = PromptResponseBoundaryManifest::new(
            PromptTokenizerMetadata::new(100).with_first_available_token_id(104),
            PromptResponseBoundaryTokens::new(103, 104),
            200,
            two_schema_manifest(),
        );
        assert!(matches!(
            boundary.validate(),
            Err(SequenceBundleManifestError::BoundaryTokenCollidesWithPromptTokenizer { .. })
        ));

        let boundary = PromptResponseBoundaryManifest::new(
            PromptTokenizerMetadata::new(100),
            PromptResponseBoundaryTokens::new(201, 300),
            200,
            two_schema_manifest(),
        );
        assert!(matches!(
            boundary.validate(),
            Err(SequenceBundleManifestError::BoundaryTokenCollidesWithTctBlock { .. })
        ));

        let boundary = PromptResponseBoundaryManifest::new(
            PromptTokenizerMetadata::new(100),
            PromptResponseBoundaryTokens::new(100, 101),
            200,
            two_schema_manifest(),
        )
        .with_text_tokens(
            PromptResponseBoundaryTokens::new(100, 101)
                .with_sentinels("<|schema:kubernetes|>", "<|tct_end|>"),
        );
        assert!(matches!(
            boundary.validate(),
            Err(SequenceBundleManifestError::SchemaSentinelCollidesWithBoundaryToken { .. })
        ));

        let mut manifest = two_schema_manifest();
        manifest.schemas[1].schema_sentinel = manifest.schemas[0].schema_sentinel.clone();
        assert!(matches!(
            manifest.validate(),
            Err(SequenceBundleManifestError::DuplicateSchemaSentinel { .. })
        ));
    }

    #[test]
    fn prompt_response_boundary_rejects_tokens_outside_tct_block() {
        let boundary = PromptResponseBoundaryManifest::new(
            PromptTokenizerMetadata::new(100),
            PromptResponseBoundaryTokens::new(100, 101),
            200,
            two_schema_manifest(),
        );

        assert!(matches!(
            boundary.bundle_token_to_hf(55),
            Err(SequenceBundleManifestError::BundleTokenOutsideVocab { .. })
        ));
        assert!(matches!(
            boundary.hf_token_to_bundle(199),
            Err(SequenceBundleManifestError::HfTokenOutsideTctBlock { .. })
        ));
        assert!(matches!(
            boundary.unwrap_response_tokens(&[201, 203, 199, 202]),
            Err(SequenceBundleManifestError::HfTokenOutsideTctBlock { .. })
        ));
        assert!(matches!(
            boundary.unwrap_handoff_response_tokens(&[99, 201, 203, 205, 202, 101]),
            Err(SequenceBundleManifestError::InvalidTctModeStart { .. })
        ));
        assert!(matches!(
            boundary.unwrap_handoff_response_tokens(&[100, 201, 203, 205, 202, 99]),
            Err(SequenceBundleManifestError::InvalidTctModeEnd { .. })
        ));
    }

    #[test]
    fn direct_local_payload_profile_requires_schema_sentinel() {
        let manifest = SequenceBundleManifest::new(
            25,
            vec![SequenceBundleSchema::new("kubernetes", 3, 18, 17, 5, false)],
        );
        let boundary = PromptResponseBoundaryManifest::new(
            PromptTokenizerMetadata::new(100),
            PromptResponseBoundaryTokens::new(100, 101),
            200,
            manifest,
        );
        assert!(matches!(
            boundary.direct_local_payload_profile_for_schema("kubernetes", 102, 200),
            Err(SequenceBundleManifestError::SchemaSentinelRequiredForDirectProfile { .. })
        ));
    }
}
