//! Reusable Go source-record raw-token emission API.
//!
//! This module owns the checked source-record boundary between parser-backed
//! Go source-instance projection evidence and raw syntax-token streams. The
//! tokenizer-generator report consumes this API instead of rebuilding the
//! projection, raw state-machine decode, and render/reparse sidecar checks
//! inline.

use crate::go_syntax_metadata::GoSyntaxMetadataAdapter;
use crate::grammar_ir_structural_model::{
    GrammarStructuralModelSidecar, lower_grammar_ir_to_structural_model,
};
use crate::provenance::json_sha256;
use crate::syntax_backend_boundary::SyntaxBackendBoundaryError;
use crate::syntax_generated_codec::go_syntax_source_file_token_layout;
use crate::syntax_go_source_instance_projection::{
    SourceInstanceCanonicalRender, SourceInstanceProjectionArtifact,
    SourceInstanceProjectionRawTokenIds, SyntaxGoSourceInstanceProjectionReport,
};
use crate::syntax_go_unified_artifact::{GO_UNIFIED_GRAMMAR_ID, go_unified_grammar_artifact};
use crate::syntax_state_machine::{
    CompiledSyntaxModel, RawSyntaxDecodedTree, RawSyntaxState, RawSyntaxStatus,
};
use crate::syntax_token_layout_diff::LIVE_RAW_SYNTAX_STREAM_CONTRACT;
use std::path::Path;

pub const GO_SOURCE_RECORD_RAW_TOKEN_EMISSION_API_ID: &str =
    "go-source-record-raw-token-emission-api-v1";
pub const GO_SOURCE_RECORD_RAW_TOKEN_EMISSION_API_BOUNDARY: &str = "checked Go parser metadata/projection source record + token-layout/structural-model context -> validated raw syntax-token stream consumed by the Go source-record generator core; render/gofmt/reparse remain sidecar validation";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GoSourceRecordRawTokenEmission {
    pub api_id: String,
    pub api_boundary: String,
    /// Expanded (state-machine-native) stream evidence: segments, decoded
    /// events, and the strict decode check stay defined over this stream.
    pub raw_tokens: SourceInstanceProjectionRawTokenIds,
    pub raw_token_segment_count: usize,
    pub raw_state_machine_decode: RawSyntaxState,
    pub decoded_tree: RawSyntaxDecodedTree,
    pub compiled_model: CompiledSyntaxModel,
    pub grammar_rule_count: usize,
    pub grammar_field_count: usize,
    pub render_reparse_sidecar: GoSourceRecordRenderReparseSidecarEvidence,
    /// Canonical encoded stream contract (`LIVE_RAW_SYNTAX_STREAM_CONTRACT`,
    /// elided-v1): the canonical stream below contains no token whose
    /// admissible mask at its position is a singleton.
    pub stream_contract: String,
    /// Canonical (elided) encoded stream: `elide_forced_raw_tokens` over the
    /// expanded stream, strict-elided-decoded and re-expansion-checked.
    pub canonical_token_ids: Vec<u32>,
    pub canonical_token_count: usize,
    pub canonical_tokens_sha256: String,
    pub canonical_max_token_id: Option<u32>,
    /// Expanded-stream token count (evidence; equals `raw_tokens.raw_token_count`).
    pub expanded_token_count: usize,
    /// Mask-forced tokens elided from the canonical stream
    /// (`expanded_token_count - canonical_token_count`).
    pub forced_token_count: usize,
}

impl GoSourceRecordRawTokenEmission {
    pub fn validate(
        &self,
        checked_report: &SyntaxGoSourceInstanceProjectionReport,
    ) -> Result<(), SyntaxBackendBoundaryError> {
        require_string(
            "source_record_raw_token_emission.api_id",
            &self.api_id,
            GO_SOURCE_RECORD_RAW_TOKEN_EMISSION_API_ID,
        )?;
        require_string(
            "source_record_raw_token_emission.api_boundary",
            &self.api_boundary,
            GO_SOURCE_RECORD_RAW_TOKEN_EMISSION_API_BOUNDARY,
        )?;
        self.raw_tokens.validate()?;
        require_string(
            "source_record_raw_token_emission.candidate_grammar_id",
            &self.raw_tokens.candidate_grammar_id,
            &checked_report.candidate_grammar_id,
        )?;
        require_string(
            "source_record_raw_token_emission.source_sha256",
            &self.raw_tokens.source_sha256,
            &checked_report.source.sha256,
        )?;
        require_string(
            "source_record_raw_token_emission.metadata_sha256",
            &self.raw_tokens.metadata_sha256,
            &checked_report.metadata.sha256,
        )?;
        require_string(
            "source_record_raw_token_emission.structural_model_id",
            &self.raw_tokens.structural_model_id,
            &format!(
                "grammar-ir:{}:structural-model-v0",
                checked_report.candidate_grammar_id
            ),
        )?;
        require_bool(
            "source_record_raw_token_emission.raw_tokens_below_vocab_size",
            self.raw_tokens.raw_tokens_below_vocab_size,
            true,
        )?;
        require_bool(
            "source_record_raw_token_emission.pad_token_absent",
            self.raw_tokens.pad_token_absent,
            true,
        )?;
        require_usize(
            "source_record_raw_token_emission.raw_token_segment_count",
            self.raw_token_segment_count,
            self.raw_tokens.raw_token_segments.len(),
        )?;
        require_usize(
            "source_record_raw_token_emission.raw_state_machine_decode.consumed_raw_tokens",
            self.raw_state_machine_decode.consumed_raw_tokens,
            self.raw_tokens.raw_token_count,
        )?;
        require_bool(
            "source_record_raw_token_emission.raw_state_machine_decode.eof_valid",
            self.raw_state_machine_decode.eof_valid,
            true,
        )?;
        if self.raw_state_machine_decode.status != RawSyntaxStatus::Complete {
            return Err(SyntaxBackendBoundaryError::new(
                "source_record_raw_token_emission.raw_state_machine_decode.status",
                "source-record raw-token emission must decode to a complete raw state",
            ));
        }
        require_usize(
            "source_record_raw_token_emission.decoded_tree.consumed_raw_tokens",
            self.decoded_tree.state.consumed_raw_tokens,
            self.raw_tokens.raw_token_count,
        )?;
        require_bool(
            "source_record_raw_token_emission.decoded_tree.eof_valid",
            self.decoded_tree.state.eof_valid,
            true,
        )?;
        require_positive_usize(
            "source_record_raw_token_emission.grammar_rule_count",
            self.grammar_rule_count,
        )?;
        require_positive_usize(
            "source_record_raw_token_emission.grammar_field_count",
            self.grammar_field_count,
        )?;
        require_string(
            "source_record_raw_token_emission.stream_contract",
            &self.stream_contract,
            LIVE_RAW_SYNTAX_STREAM_CONTRACT,
        )?;
        require_usize(
            "source_record_raw_token_emission.canonical_token_count",
            self.canonical_token_count,
            self.canonical_token_ids.len(),
        )?;
        require_usize(
            "source_record_raw_token_emission.expanded_token_count",
            self.expanded_token_count,
            self.raw_tokens.raw_token_count,
        )?;
        require_usize(
            "source_record_raw_token_emission.forced_token_count",
            self.forced_token_count,
            self.expanded_token_count - self.canonical_token_count,
        )?;
        let canonical_tokens_sha256 = json_sha256(&self.canonical_token_ids).map_err(|error| {
            SyntaxBackendBoundaryError::new(
                "source_record_raw_token_emission.canonical_tokens_sha256",
                format!("failed to hash the canonical elided token stream: {error}"),
            )
        })?;
        require_string(
            "source_record_raw_token_emission.canonical_tokens_sha256",
            &self.canonical_tokens_sha256,
            &canonical_tokens_sha256,
        )?;
        if self.canonical_max_token_id != self.canonical_token_ids.iter().copied().max() {
            return Err(SyntaxBackendBoundaryError::new(
                "source_record_raw_token_emission.canonical_max_token_id",
                "canonical max token id must match the canonical elided stream",
            ));
        }
        self.render_reparse_sidecar.validate(checked_report)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GoSourceRecordRenderReparseSidecarEvidence {
    pub canonical_render: SourceInstanceCanonicalRender,
    pub canonical_render_ready: bool,
    pub gofmt_validation: String,
    pub go_parser_reparse_validation: String,
    pub render_reparse_sidecar_validation: bool,
}

impl GoSourceRecordRenderReparseSidecarEvidence {
    fn from_checked_report(report: &SyntaxGoSourceInstanceProjectionReport) -> Self {
        let canonical_render_ready = report.canonical_render.gofmt == "passed"
            && report.canonical_render.go_parser == "passed"
            && report.canonical_render.matches_source
            && report.canonical_render.canonical_sha256 == report.source.sha256;
        Self {
            canonical_render: report.canonical_render.clone(),
            canonical_render_ready,
            gofmt_validation: report.canonical_render.gofmt.clone(),
            go_parser_reparse_validation: report.canonical_render.go_parser.clone(),
            render_reparse_sidecar_validation: canonical_render_ready,
        }
    }

    fn validate(
        &self,
        checked_report: &SyntaxGoSourceInstanceProjectionReport,
    ) -> Result<(), SyntaxBackendBoundaryError> {
        require_string(
            "source_record_raw_token_emission.render_reparse_sidecar.formatter",
            &self.canonical_render.formatter,
            "go/format.Source",
        )?;
        require_string(
            "source_record_raw_token_emission.render_reparse_sidecar.gofmt",
            &self.gofmt_validation,
            "passed",
        )?;
        require_string(
            "source_record_raw_token_emission.render_reparse_sidecar.go_parser",
            &self.go_parser_reparse_validation,
            "passed",
        )?;
        require_bool(
            "source_record_raw_token_emission.render_reparse_sidecar.matches_source",
            self.canonical_render.matches_source,
            true,
        )?;
        require_string(
            "source_record_raw_token_emission.render_reparse_sidecar.canonical_sha256",
            &self.canonical_render.canonical_sha256,
            &checked_report.source.sha256,
        )?;
        require_usize(
            "source_record_raw_token_emission.render_reparse_sidecar.canonical_byte_count",
            self.canonical_render.canonical_byte_count,
            checked_report.source.bytes,
        )?;
        require_bool(
            "source_record_raw_token_emission.render_reparse_sidecar.canonical_render_ready",
            self.canonical_render_ready,
            true,
        )?;
        require_bool(
            "source_record_raw_token_emission.render_reparse_sidecar.validation",
            self.render_reparse_sidecar_validation,
            true,
        )
    }
}

pub fn emit_go_source_record_raw_tokens(
    checked_report: &SyntaxGoSourceInstanceProjectionReport,
    projection_artifact: &SourceInstanceProjectionArtifact,
) -> Result<GoSourceRecordRawTokenEmission, SyntaxBackendBoundaryError> {
    checked_report.validate()?;
    projection_artifact.validate_against_report(checked_report)?;
    let raw_tokens = projection_artifact.raw_token_ids_against_report(checked_report)?;
    raw_tokens.validate()?;

    let sidecar = source_instance_structural_model_sidecar(checked_report)?;
    let model = CompiledSyntaxModel::from_sidecar_and_layout(&sidecar, &raw_tokens.token_layout)?;
    let decoded_tree = model
        .decode_complete_tree(&raw_tokens.raw_token_ids)
        .map_err(|error| {
            SyntaxBackendBoundaryError::new(
                "source_record_raw_token_emission.raw_state_machine_decode",
                format!("raw syntax state machine rejected checked Go token stream: {error:?}"),
            )
        })?;
    let grammar_rule_count = sidecar.structural_model.records.len();
    let grammar_field_count = sidecar
        .structural_model
        .records
        .iter()
        .map(|record| record.fields.len())
        .sum();

    // Canonical encoded stream: elide mask-forced tokens, then prove the
    // canonical stream strict-elided-decodes and re-expands token-exactly.
    let canonical_token_ids = model
        .elide_forced_raw_tokens(&raw_tokens.raw_token_ids)
        .map_err(|error| {
            SyntaxBackendBoundaryError::new(
                "source_record_raw_token_emission.canonical_token_ids",
                format!("failed to elide mask-forced tokens from checked Go stream: {error:?}"),
            )
        })?;
    model
        .strict_decode_complete_elided(&canonical_token_ids)
        .map_err(|error| {
            SyntaxBackendBoundaryError::new(
                "source_record_raw_token_emission.canonical_token_ids",
                format!("canonical elided Go stream failed strict elided decode: {error:?}"),
            )
        })?;
    let expanded_round_trip = model
        .expand_elided_raw_tokens(&canonical_token_ids)
        .map_err(|error| {
            SyntaxBackendBoundaryError::new(
                "source_record_raw_token_emission.canonical_token_ids",
                format!("canonical elided Go stream failed re-expansion: {error:?}"),
            )
        })?;
    if expanded_round_trip != raw_tokens.raw_token_ids {
        return Err(SyntaxBackendBoundaryError::new(
            "source_record_raw_token_emission.canonical_token_ids",
            "re-expanded canonical stream must equal the expanded stream token-for-token",
        ));
    }
    let canonical_tokens_sha256 = json_sha256(&canonical_token_ids).map_err(|error| {
        SyntaxBackendBoundaryError::new(
            "source_record_raw_token_emission.canonical_tokens_sha256",
            format!("failed to hash the canonical elided token stream: {error}"),
        )
    })?;
    let canonical_token_count = canonical_token_ids.len();
    let canonical_max_token_id = canonical_token_ids.iter().copied().max();
    let expanded_token_count = raw_tokens.raw_token_count;
    let forced_token_count = expanded_token_count - canonical_token_count;

    let emission = GoSourceRecordRawTokenEmission {
        api_id: GO_SOURCE_RECORD_RAW_TOKEN_EMISSION_API_ID.to_string(),
        api_boundary: GO_SOURCE_RECORD_RAW_TOKEN_EMISSION_API_BOUNDARY.to_string(),
        raw_token_segment_count: raw_tokens.raw_token_segments.len(),
        raw_state_machine_decode: decoded_tree.state.clone(),
        decoded_tree,
        compiled_model: model,
        grammar_rule_count,
        grammar_field_count,
        render_reparse_sidecar: GoSourceRecordRenderReparseSidecarEvidence::from_checked_report(
            checked_report,
        ),
        raw_tokens,
        stream_contract: LIVE_RAW_SYNTAX_STREAM_CONTRACT.to_string(),
        canonical_token_ids,
        canonical_token_count,
        canonical_tokens_sha256,
        canonical_max_token_id,
        expanded_token_count,
        forced_token_count,
    };
    emission.validate(checked_report)?;
    Ok(emission)
}

fn source_instance_structural_model_sidecar(
    report: &SyntaxGoSourceInstanceProjectionReport,
) -> Result<GrammarStructuralModelSidecar, SyntaxBackendBoundaryError> {
    if report.candidate_grammar_id == GO_UNIFIED_GRAMMAR_ID {
        // Unified inventory route: source records validate against the one
        // pinned inventory-derived grammar artifact.
        return Ok(go_unified_grammar_artifact()?.sidecar.clone());
    }
    let grammar = GoSyntaxMetadataAdapter::new(&report.candidate_grammar_id)
        .lower_metadata_file(Path::new(&report.metadata.path))
        .map_err(|error| {
            SyntaxBackendBoundaryError::new(
                "source_record_raw_token_emission.metadata",
                format!("failed to lower Go syntax metadata: {error}"),
            )
        })?;
    lower_grammar_ir_to_structural_model(&grammar, go_syntax_source_file_token_layout()).map_err(
        |error| {
            SyntaxBackendBoundaryError::new(
                "source_record_raw_token_emission.structural_model",
                format!("failed to lower GrammarIR to StructuralModel: {error}"),
            )
        },
    )
}

fn require_string(
    field: &'static str,
    actual: &str,
    expected: &str,
) -> Result<(), SyntaxBackendBoundaryError> {
    if actual == expected {
        Ok(())
    } else {
        Err(SyntaxBackendBoundaryError::new(
            field,
            format!("expected {expected:?}, got {actual:?}"),
        ))
    }
}

fn require_bool(
    field: &'static str,
    actual: bool,
    expected: bool,
) -> Result<(), SyntaxBackendBoundaryError> {
    if actual == expected {
        Ok(())
    } else {
        Err(SyntaxBackendBoundaryError::new(
            field,
            format!("expected {expected}, got {actual}"),
        ))
    }
}

fn require_usize(
    field: &'static str,
    actual: usize,
    expected: usize,
) -> Result<(), SyntaxBackendBoundaryError> {
    if actual == expected {
        Ok(())
    } else {
        Err(SyntaxBackendBoundaryError::new(
            field,
            format!("expected {expected}, got {actual}"),
        ))
    }
}

fn require_positive_usize(
    field: &'static str,
    actual: usize,
) -> Result<(), SyntaxBackendBoundaryError> {
    if actual > 0 {
        Ok(())
    } else {
        Err(SyntaxBackendBoundaryError::new(
            field,
            "expected a positive value",
        ))
    }
}
