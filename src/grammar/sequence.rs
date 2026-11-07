//! Emits generated sequence-boundary wrapper APIs.
//!
//! Raw generated tokenizers encode only schema-local payload tokens. This
//! module renders the optional model-facing wrapper layer that reserves PAD,
//! BOS/EOS, schema, and payload-offset tokens while exposing metadata that lets
//! downstream bundles distinguish raw vocabulary ranges from wrapped sequence
//! ranges.

use proc_macro2::TokenStream;
use quote::quote;

use super::{Grammar, quote_option_str, quote_option_u32, quote_option_usize};

impl Grammar {
    /// Generate model-facing sequence boundary helpers.
    pub(super) fn generate_sequence_boundary_wrappers(&self) -> TokenStream {
        let bpe_enabled = self.bpe_merge_table.is_some();
        let decimal_number_encoding = self.decimal_number_encoding;
        let raw_bpe_merge_count = self
            .bpe_merge_table
            .as_ref()
            .map(|table| table.merges.len() as u32)
            .unwrap_or(0);
        let bpe_schema = self
            .bpe_merge_table
            .as_ref()
            .map(|table| table.schema.as_str());
        let bpe_first_new_token = self
            .bpe_merge_table
            .as_ref()
            .and_then(|table| table.merges.first().map(|merge| merge.new_token));
        let bpe_last_new_token = self
            .bpe_merge_table
            .as_ref()
            .and_then(|table| table.merges.last().map(|merge| merge.new_token));
        let bpe_metadata_trained_on_instances = self
            .bpe_merge_table
            .as_ref()
            .map(|table| table.metadata.trained_on_instances);
        let bpe_metadata_training_date = self
            .bpe_merge_table
            .as_ref()
            .map(|table| table.metadata.training_date.as_str());
        let bpe_metadata_max_merges = self
            .bpe_merge_table
            .as_ref()
            .map(|table| table.metadata.max_merges);
        let bpe_schema = quote_option_str(bpe_schema);
        let bpe_first_new_token = quote_option_u32(bpe_first_new_token);
        let bpe_last_new_token = quote_option_u32(bpe_last_new_token);
        let bpe_metadata_trained_on_instances =
            quote_option_usize(bpe_metadata_trained_on_instances);
        let bpe_metadata_training_date = quote_option_str(bpe_metadata_training_date);
        let bpe_metadata_max_merges = quote_option_usize(bpe_metadata_max_merges);
        let decode_prefix_supported = self.streaming_encode;
        let encode_payload = if self.bpe_merge_table.is_some() {
            quote! {
                let payload = super::codec::encode(value);
            }
        } else {
            quote! {
                let mut payload = Vec::new();
                value.encode(&mut payload);
            }
        };

        let decode_payload = if self.bpe_merge_table.is_some() {
            quote! {
                let wrapper_payload_total = payload.len();
                let (value, base_consumed, base_total) = super::codec::decode::<T>(payload)?;
                Ok((
                    value,
                    wrapper_payload_total,
                    wrapper_payload_total,
                    base_consumed,
                    base_total,
                ))
            }
        } else {
            quote! {
                let total = payload.len();
                let mut iter = crate::tokenizer::BufferedTokenIterator::from_vec(payload);
                let value = T::decode(&mut iter)?;
                let consumed = total.saturating_sub(iter.remaining());
                Ok((value, consumed, total, consumed, total))
            }
        };

        quote! {
            /// Model-facing sequence boundary helpers.
            ///
            /// Raw schema-local tokenization remains content-only. This module is
            /// for training or serving wrappers that need explicit sequence
            /// boundaries around a complete generated-tokenizer payload.
            pub mod sequence {
                use crate::tokenizer::{DecodeError, JsonTokenEncoder};

                /// Version of the serialized single-schema metadata shape.
                pub const SEQUENCE_METADATA_VERSION: u32 = 1;

                /// Stable artifact-kind identifier for this metadata payload.
                pub const SEQUENCE_ARTIFACT_KIND: &str = "tct.sequence_boundary.single_schema";

                /// Half-open token range `[start, end_exclusive)`.
                #[derive(Clone, Debug, PartialEq, Eq, serde::Serialize)]
                pub struct SequenceTokenRange {
                    pub start: u32,
                    pub end_exclusive: u32,
                }

                /// Wrapper-special token reservations for model-facing sequences.
                #[derive(Clone, Debug, PartialEq, Eq, serde::Serialize)]
                pub struct SequenceSpecialTokenPolicy {
                    pub reserved_start: u32,
                    pub reserved_end_exclusive: u32,
                    pub pad_token: u32,
                    pub bos_token: u32,
                    pub eos_token: u32,
                    pub start_token: u32,
                    pub end_token: u32,
                    pub schema_token: u32,
                    pub payload_offset: u32,
                }

                /// BPE provenance exposed by generated sequence metadata.
                #[derive(Clone, Debug, PartialEq, Eq, serde::Serialize)]
                pub struct SequenceBpeMetadata {
                    pub enabled: bool,
                    pub schema: Option<&'static str>,
                    pub base_vocab_size: u32,
                    pub bpe_start: u32,
                    pub merge_count: u32,
                    pub first_new_token: Option<u32>,
                    pub last_new_token: Option<u32>,
                    pub metadata_trained_on_instances: Option<usize>,
                    pub metadata_training_date: Option<&'static str>,
                    pub metadata_max_merges: Option<usize>,
                }

                /// Serializable single-schema sequence-boundary manifest.
                ///
                /// Multi-schema bundles should generalize this shape by adding one
                /// entry per schema-decision token while preserving the raw-vs-wrapper
                /// vocabulary separation.
                #[derive(Clone, Debug, PartialEq, Eq, serde::Serialize)]
                pub struct SequenceBoundaryMetadata {
                    pub artifact_kind: &'static str,
                    pub artifact_version: u32,
                    pub metadata_version: u32,
                    pub version: u32,
                    pub schema_id: &'static str,
                    pub schema_token: u32,
                    pub raw_base_vocab_size: u32,
                    pub raw_base_tokens: SequenceTokenRange,
                    pub raw_bpe_tokens: SequenceTokenRange,
                    pub raw_bpe_merge_count: u32,
                    pub raw_vocab_size: u32,
                    pub raw_pad_token: u32,
                    pub raw_payload_start: u32,
                    pub raw_payload_end_exclusive: u32,
                    pub payload_offset: u32,
                    pub sequence_vocab_size: u32,
                    pub sequence_pad_token: u32,
                    pub sequence_bos_token: u32,
                    pub sequence_eos_token: u32,
                    pub sequence_start_token: u32,
                    pub sequence_end_token: u32,
                    pub sequence_reserved_tokens: SequenceSpecialTokenPolicy,
                    pub decode_prefix_supported: bool,
                    pub bpe_enabled: bool,
                    pub bpe: SequenceBpeMetadata,
                    /// Unbounded JSON numbers encode as decimal text byte
                    /// tokens (digit-aligned with the JSON text) when true.
                    pub decimal_number_encoding: bool,
                }

                /// Return the generated single-schema boundary manifest.
                pub fn metadata() -> SequenceBoundaryMetadata {
                    SequenceBoundaryMetadata {
                        artifact_kind: SEQUENCE_ARTIFACT_KIND,
                        artifact_version: SEQUENCE_METADATA_VERSION,
                        metadata_version: SEQUENCE_METADATA_VERSION,
                        version: super::SEQUENCE_BOUNDARY_VERSION,
                        schema_id: super::SEQUENCE_SCHEMA_ID,
                        schema_token: super::SEQUENCE_SCHEMA_TOKEN,
                        raw_base_vocab_size: super::BASE_VOCAB_SIZE,
                        raw_base_tokens: SequenceTokenRange {
                            start: 0,
                            end_exclusive: super::BASE_VOCAB_SIZE,
                        },
                        raw_bpe_tokens: SequenceTokenRange {
                            start: super::BASE_VOCAB_SIZE,
                            end_exclusive: super::PAD_TOKEN,
                        },
                        raw_bpe_merge_count: #raw_bpe_merge_count,
                        raw_vocab_size: super::VOCAB_SIZE,
                        raw_pad_token: super::PAD_TOKEN,
                        raw_payload_start: 0,
                        raw_payload_end_exclusive: super::PAD_TOKEN,
                        payload_offset: super::SEQUENCE_PAYLOAD_OFFSET,
                        sequence_vocab_size: super::SEQUENCE_VOCAB_SIZE,
                        sequence_pad_token: super::SEQUENCE_PAD_TOKEN,
                        sequence_bos_token: super::SEQUENCE_BOS_TOKEN,
                        sequence_eos_token: super::SEQUENCE_EOS_TOKEN,
                        sequence_start_token: super::SEQUENCE_START_TOKEN,
                        sequence_end_token: super::SEQUENCE_END_TOKEN,
                        sequence_reserved_tokens: SequenceSpecialTokenPolicy {
                            reserved_start: super::SEQUENCE_PAD_TOKEN,
                            reserved_end_exclusive: super::SEQUENCE_PAYLOAD_OFFSET,
                            pad_token: super::SEQUENCE_PAD_TOKEN,
                            bos_token: super::SEQUENCE_BOS_TOKEN,
                            eos_token: super::SEQUENCE_EOS_TOKEN,
                            start_token: super::SEQUENCE_START_TOKEN,
                            end_token: super::SEQUENCE_END_TOKEN,
                            schema_token: super::SEQUENCE_SCHEMA_TOKEN,
                            payload_offset: super::SEQUENCE_PAYLOAD_OFFSET,
                        },
                        decode_prefix_supported: #decode_prefix_supported,
                        bpe_enabled: #bpe_enabled,
                        bpe: SequenceBpeMetadata {
                            enabled: #bpe_enabled,
                            schema: #bpe_schema,
                            base_vocab_size: super::BASE_VOCAB_SIZE,
                            bpe_start: super::BASE_VOCAB_SIZE,
                            merge_count: #raw_bpe_merge_count,
                            first_new_token: #bpe_first_new_token,
                            last_new_token: #bpe_last_new_token,
                            metadata_trained_on_instances: #bpe_metadata_trained_on_instances,
                            metadata_training_date: #bpe_metadata_training_date,
                            metadata_max_merges: #bpe_metadata_max_merges,
                        },
                        decimal_number_encoding: #decimal_number_encoding,
                    }
                }

                /// Encode a value as
                /// `[SEQUENCE_BOS_TOKEN, SEQUENCE_SCHEMA_TOKEN, payload..., SEQUENCE_EOS_TOKEN]`.
                ///
                /// The payload uses the same compiled representation as the raw
                /// generated tokenizer, including BPE when the generated code was
                /// compiled with a merge table. Raw payload tokens are offset so
                /// they cannot collide with model-facing special tokens.
                pub fn encode<T: JsonTokenEncoder>(value: &T) -> Vec<u32> {
                    #encode_payload

                    let mut tokens = Vec::with_capacity(payload.len() + 3);
                    tokens.push(super::SEQUENCE_BOS_TOKEN);
                    tokens.push(super::SEQUENCE_SCHEMA_TOKEN);
                    tokens.extend(payload.into_iter().map(|token| {
                        debug_assert!(
                            token < super::PAD_TOKEN,
                            "raw PAD or reserved token cannot appear in sequence payload"
                        );
                        token + super::SEQUENCE_PAYLOAD_OFFSET
                    }));
                    tokens.push(super::SEQUENCE_EOS_TOKEN);
                    tokens
                }

                /// Return the raw schema-local payload between the sequence
                /// header and `SEQUENCE_EOS_TOKEN`.
                ///
                /// Trailing `SEQUENCE_PAD_TOKEN`s after `SEQUENCE_EOS_TOKEN` are
                /// accepted. Wrapper tokens inside the payload are rejected so
                /// callers do not accidentally feed special tokens into raw decode.
                pub fn payload_tokens(tokens: &[u32]) -> Result<Vec<u32>, DecodeError> {
                    if tokens.first().copied() != Some(super::SEQUENCE_BOS_TOKEN) {
                        return Err(DecodeError::InvalidData(
                            "sequence must start with SEQUENCE_BOS_TOKEN".to_string(),
                        ));
                    }
                    if tokens.get(1).copied() != Some(super::SEQUENCE_SCHEMA_TOKEN) {
                        return Err(DecodeError::InvalidData(
                            "sequence must include the expected schema-decision token".to_string(),
                        ));
                    }

                    let eos_index = tokens
                        .iter()
                        .enumerate()
                        .skip(2)
                        .find_map(|(index, token)| {
                            (*token == super::SEQUENCE_EOS_TOKEN).then_some(index)
                        })
                        .ok_or_else(|| {
                            DecodeError::InvalidData(
                                "sequence is missing SEQUENCE_EOS_TOKEN".to_string(),
                            )
                        })?;

                    if tokens[eos_index + 1..]
                        .iter()
                        .any(|token| *token != super::SEQUENCE_PAD_TOKEN)
                    {
                        return Err(DecodeError::InvalidData(
                            "tokens after SEQUENCE_EOS_TOKEN must be SEQUENCE_PAD_TOKEN".to_string(),
                        ));
                    }

                    let mut payload = Vec::with_capacity(eos_index.saturating_sub(2));
                    for (offset, token) in tokens[2..eos_index].iter().copied().enumerate() {
                        if token < super::SEQUENCE_PAYLOAD_OFFSET
                            || token >= super::SEQUENCE_VOCAB_SIZE
                        {
                            return Err(DecodeError::InvalidData(format!(
                                "invalid sequence payload token {} at payload offset {}",
                                token, offset
                            )));
                        }
                        payload.push(token - super::SEQUENCE_PAYLOAD_OFFSET);
                    }

                    Ok(payload)
                }

                /// Decode a sequence produced by [`encode`].
                ///
                /// Returns `(value, consumed_tokens, total_tokens)`. Trailing
                /// padding after `SEQUENCE_EOS_TOKEN` is accepted but not
                /// counted as consumed.
                pub fn decode<T: JsonTokenEncoder>(
                    tokens: &[u32],
                ) -> Result<(T, usize, usize), DecodeError> {
                    let payload = payload_tokens(tokens)?;
                    let (
                        value,
                        wrapper_payload_consumed,
                        wrapper_payload_total,
                        raw_payload_consumed,
                        raw_payload_total,
                    ) = {
                        #decode_payload
                    }?;

                    if raw_payload_consumed != raw_payload_total {
                        return Err(DecodeError::InvalidData(format!(
                            "sequence payload left {} surplus tokens",
                            raw_payload_total.saturating_sub(raw_payload_consumed)
                        )));
                    }

                    if wrapper_payload_consumed != wrapper_payload_total {
                        return Err(DecodeError::InvalidData(format!(
                            "sequence wrapper payload left {} surplus tokens",
                            wrapper_payload_total.saturating_sub(wrapper_payload_consumed)
                        )));
                    }

                    Ok((value, wrapper_payload_consumed + 3, tokens.len()))
                }
            }
        }
    }
}
