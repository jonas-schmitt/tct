//! Go parser-backed source-instance projection report gate.
//!
//! This module consumes checked Go source-instance projection reports as typed
//! Rust evidence. It produces target-sidecar raw syntax tokens and a
//! generated-codec/source-route report while keeping corpus promotion, arena
//! IR, and stable backend claims blocked.

use crate::go_syntax_metadata::{
    GO_SYNTAX_METADATA_ARTIFACT_KIND, GO_SYNTAX_METADATA_VERSION, GoSyntaxMetadataAdapter,
};
use crate::grammar_ir::GrammarTerminalKind;
use crate::grammar_ir_structural_model::{
    GrammarStructuralModelSidecar, lower_grammar_ir_to_structural_model,
};
use crate::provenance::{json_sha256, sha256_hex};
use crate::syntax_backend_boundary::SyntaxBackendBoundaryError;
use crate::syntax_generated_codec::go_syntax_source_file_token_layout;
use crate::syntax_go_unified_artifact::{
    GO_AST_SCHEMA_INVENTORY_METADATA_ARTIFACT_KIND, GO_UNIFIED_GRAMMAR_ARTIFACT_ID,
    GO_UNIFIED_GRAMMAR_ID, go_unified_grammar_artifact,
};
use crate::syntax_token_layout::token_range_by_role;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::Path;

pub use crate::syntax_token_layout::{
    ProjectionTokenLayoutRangeSummary, ProjectionTokenLayoutSummary,
    RAW_TOKEN_ALTERNATIVE_NODE_START_OFFSET, node_start_alternative_token_ids_from_sidecar,
};

pub const TOOL: &str = "syntax_backend_go_source_instance_projection_gate";
pub const VERSION: u32 = 1;
pub const ARTIFACT_KIND: &str = "tct_syntax_backend_go_source_instance_projection_gate";
pub const CODEC_INPUT_ARTIFACT_KIND: &str =
    "tct_go_syntax_source_instance_projection_codec_input_v1";
pub const TOKEN_LAYOUT_MAPPING_ARTIFACT_KIND: &str =
    "tct_go_syntax_source_instance_projection_token_layout_mapping_v1";
pub const RAW_TOKEN_IDS_ARTIFACT_KIND: &str =
    "tct_go_syntax_source_instance_projection_raw_token_ids_v1";
pub const GENERATED_CODEC_REPORT_ARTIFACT_KIND: &str =
    "tct_go_syntax_source_instance_generated_codec_report_v1";
pub const PROJECTION_ARTIFACT_KIND: &str = "tct_go_syntax_source_instance_projection_v1";
pub const REPORT_ARTIFACT_KIND: &str = "tct_go_syntax_source_instance_projection_report_v1";
pub const REPORT_TOOL: &str = "go_syntax_source_instance_project";
pub const STATUS: &str = "blocked_before_generated_codec_report";
pub const PROJECTION_STATUS: &str =
    "source_instance_projection_ready_generated_codec_report_blocked";
pub const TARGET: &str = "go";
pub const PARSER_TOOL: &str = "go";
pub const PARSER_PACKAGE: &str = "go/parser";
pub const PARSER_VERSION: &str = "go1.24.3";
pub const NEXT_MISSING_WORKSTREAM_BLOCKER: &str =
    "projection-driven Go generated-codec emitter and report consumer";
pub const CODEC_INPUT_NEXT_MISSING_WORKSTREAM_BLOCKER: &str = "map projection codec input events through Go syntax token layout and generated-codec report producer";
pub const TOKEN_LAYOUT_MAPPING_STATUS: &str =
    "projection_codec_input_events_mapped_to_source_file_token_layout_roles";
pub const TOKEN_LAYOUT_MAPPING_NEXT_MISSING_WORKSTREAM_BLOCKER: &str = "emit contextual Go syntax raw token IDs and generated-codec report from token-layout-mapped projection events";
pub const RAW_TOKEN_IDS_STATUS: &str =
    "projection_contextual_raw_token_ids_ready_generated_codec_report_blocked";
pub const RAW_TOKEN_IDS_NEXT_MISSING_WORKSTREAM_BLOCKER: &str =
    "produce generated-codec report from contextual Go syntax raw token IDs";
pub const GENERATED_CODEC_REPORT_STATUS: &str =
    "source_instance_generated_codec_report_ready_manifest_promotion_blocked";
pub const GENERATED_CODEC_REPORT_ROUTE: &str =
    "go_source_instance_projection_contextual_raw_tokens";
pub const GENERATED_CODEC_REPORT_DIR: &str = "target/tct-syntax-real-source-demo";
pub const GENERATED_CODEC_REPORT_COMMAND: &str = "cargo test --test syntax_go_source_instance_projection_test go_source_instance_projection_produces_generated_codec_reports_from_checked_data -- --exact";

const RAW_TOKEN_NODE_START_OFFSET: u32 = 0;
const RAW_TOKEN_NODE_END_OFFSET: u32 = 1;
const RAW_TOKEN_FIELD_START_OFFSET: u32 = 2;
const RAW_TOKEN_FIELD_END_OFFSET: u32 = 3;
const RAW_TOKEN_FIELD_MISSING_OFFSET: u32 = 4;
const RAW_TOKEN_REPEATED_EMPTY_OFFSET: u32 = 5;
const RAW_TOKEN_TERMINAL_OFFSET: u32 = 6;
const RAW_TOKEN_PAYLOAD_END_OFFSET: u32 = 7;
const RAW_TOKEN_BOOLEAN_FALSE_OFFSET: u32 = 8;
const RAW_TOKEN_BOOLEAN_TRUE_OFFSET: u32 = 9;

const REQUIRED_BLOCKERS: &[&str] = &[
    "generated_codec_report_blocked_by_missing_projection_consumer",
    "report_ingestion_manifest_record_missing",
];

const REQUIRED_NO_STABLE_CLAIMS: &[&str] = &[
    "no_generated_codec_report_claim",
    "no_training_corpus_manifest_promotion",
    "no_report_ingestion_manifest_promotion",
    "no_source_validity_by_construction_claim",
    "no_type_correctness_claim",
    "no_model_quality_claim",
    "no_paper_facing_claim",
    "no_generated_hot_path_claim",
    "no_arena_ir_promotion",
    "no_stable_backend_promotion",
];

const REQUIRED_GENERATED_CODEC_REPORT_NO_STABLE_CLAIMS: &[&str] = &[
    "no_training_corpus_manifest_promotion",
    "no_report_ingestion_manifest_promotion",
    "no_broad_corpus_claim",
    "no_source_validity_by_construction_claim",
    "no_type_correctness_claim",
    "no_model_quality_claim",
    "no_paper_facing_claim",
    "no_generated_hot_path_claim",
    "no_arena_ir_promotion",
    "no_stable_backend_promotion",
];

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SyntaxGoSourceInstanceProjectionReport {
    pub artifact_kind: String,
    pub version: u32,
    pub tool: String,
    pub status: String,
    pub target: String,
    pub candidate_grammar_id: String,
    /// Recorded-blocker escape hatch for retired per-file Go grammar ids.
    /// Forward records carry the unified inventory grammar id and must leave
    /// this unset; a historical checked fixture keeps its
    /// `go_*_source_file_syntax_v0` id only by recording this blocker.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub per_file_grammar_blocker: Option<SourceInstancePerFileGrammarBlocker>,
    pub manifest_path: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub report_ingestion_manifest_path: String,
    pub source: SourceInstanceProjectionSource,
    pub metadata: SourceInstanceProjectionMetadata,
    pub parser: SourceInstanceProjectionParser,
    pub projection_output: String,
    pub projection_sha256: String,
    pub projection_stats: SourceInstanceProjectionStats,
    pub canonical_render: SourceInstanceCanonicalRender,
    pub current_evidence: SourceInstanceCurrentEvidence,
    pub current_blockers: Vec<SourceInstanceCurrentBlocker>,
    pub next_concrete_task: String,
    pub validation_commands: Vec<String>,
    pub claim_boundary: String,
    pub no_stable_claims: Vec<String>,
}

impl SyntaxGoSourceInstanceProjectionReport {
    pub fn validate(&self) -> Result<(), SyntaxBackendBoundaryError> {
        require_string("artifact_kind", &self.artifact_kind, REPORT_ARTIFACT_KIND)?;
        require_u32("version", self.version, VERSION)?;
        require_string("tool", &self.tool, REPORT_TOOL)?;
        require_string("status", &self.status, STATUS)?;
        require_string("target", &self.target, TARGET)?;
        validate_go_syntax_grammar_id("candidate_grammar_id", &self.candidate_grammar_id)?;
        validate_forward_go_grammar_registration(
            &self.candidate_grammar_id,
            self.per_file_grammar_blocker.as_ref(),
        )?;
        require_non_empty("manifest_path", &self.manifest_path)?;
        self.source
            .validate_checked_file(&self.candidate_grammar_id)?;
        self.metadata.validate_checked_file(
            &self.candidate_grammar_id,
            &self.source,
            &self.parser,
        )?;
        self.parser.validate()?;
        require_non_empty("projection_output", &self.projection_output)?;
        if !self.projection_output.starts_with("target/") {
            return Err(SyntaxBackendBoundaryError::new(
                "projection_output",
                "source-instance projection value artifact must remain target/ sidecar evidence until a later persistence gate",
            ));
        }
        require_sha256("projection_sha256", &self.projection_sha256)?;
        self.projection_stats.validate()?;
        self.canonical_render.validate(&self.source)?;
        self.current_evidence
            .validate(&self.projection_stats, &self.metadata)?;
        self.validate_blockers()?;
        require_contains(
            "next_concrete_task",
            &self.next_concrete_task,
            "consume the checked Go source-instance projection",
        )?;
        require_contains(
            "next_concrete_task",
            &self.next_concrete_task,
            "reusable generated-codec renderer/report machinery",
        )?;
        validate_projection_report_commands(
            &self.validation_commands,
            &self.source.path,
            &self.metadata.path,
            &self.manifest_path,
            &self.candidate_grammar_id,
            &self.projection_output,
        )?;
        require_contains(
            "claim_boundary",
            &self.claim_boundary,
            "parser-backed source-instance projection evidence only",
        )?;
        require_contains(
            "claim_boundary",
            &self.claim_boundary,
            "generated-codec token production",
        )?;
        require_contains("claim_boundary", &self.claim_boundary, "arena-IR")?;
        require_contains("claim_boundary", &self.claim_boundary, "stable-backend")?;
        require_all(
            "no_stable_claims",
            &self.no_stable_claims,
            REQUIRED_NO_STABLE_CLAIMS,
        )?;
        Ok(())
    }

    pub fn gate_summary(
        &self,
    ) -> Result<SourceInstanceProjectionGateSummary, SyntaxBackendBoundaryError> {
        self.validate()?;
        Ok(SourceInstanceProjectionGateSummary {
            tool: TOOL.to_string(),
            version: VERSION,
            artifact_kind: ARTIFACT_KIND.to_string(),
            source_path: self.source.path.clone(),
            source_sha256: self.source.sha256.clone(),
            metadata_path: self.metadata.path.clone(),
            metadata_sha256: self.metadata.sha256.clone(),
            candidate_grammar_id: self.candidate_grammar_id.clone(),
            projection_sha256: self.projection_sha256.clone(),
            node_count: self.projection_stats.node_count,
            field_value_count: self.projection_stats.field_value_count,
            terminal_payload_count: self.projection_stats.terminal_payload_count,
            canonical_render_matches_source: self.canonical_render.matches_source,
            generated_codec_report_ready: self.current_evidence.generated_codec_report_ready,
            reusable_renderer_consumer_ready: self
                .current_evidence
                .reusable_renderer_consumer_ready,
            next_missing_workstream_blocker: NEXT_MISSING_WORKSTREAM_BLOCKER.to_string(),
        })
    }

    fn validate_blockers(&self) -> Result<(), SyntaxBackendBoundaryError> {
        let kinds = self
            .current_blockers
            .iter()
            .map(|blocker| blocker.kind.as_str())
            .collect::<BTreeSet<_>>();
        for required in REQUIRED_BLOCKERS {
            if !kinds.contains(required) {
                return Err(SyntaxBackendBoundaryError::new(
                    "current_blockers",
                    format!("missing blocker {required}"),
                ));
            }
        }
        for blocker in &self.current_blockers {
            blocker.validate(&self.candidate_grammar_id)?;
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SourceInstanceProjectionGateSummary {
    pub tool: String,
    pub version: u32,
    pub artifact_kind: String,
    pub source_path: String,
    pub source_sha256: String,
    pub metadata_path: String,
    pub metadata_sha256: String,
    pub candidate_grammar_id: String,
    pub projection_sha256: String,
    pub node_count: usize,
    pub field_value_count: usize,
    pub terminal_payload_count: usize,
    pub canonical_render_matches_source: bool,
    pub generated_codec_report_ready: bool,
    pub reusable_renderer_consumer_ready: bool,
    pub next_missing_workstream_blocker: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SourceInstanceProjectionArtifact {
    pub artifact_kind: String,
    pub version: u32,
    pub tool: String,
    pub status: String,
    pub target: String,
    pub candidate_grammar_id: String,
    pub manifest_path: String,
    pub source: SourceInstanceProjectionSource,
    pub metadata: SourceInstanceProjectionMetadata,
    pub parser: SourceInstanceProjectionParser,
    pub root: SourceInstanceProjectionNode,
    pub stats: SourceInstanceProjectionStats,
    pub claim_boundary: String,
    pub no_stable_claims: Vec<String>,
    #[serde(default)]
    pub notes: BTreeMap<String, serde_json::Value>,
}

impl SourceInstanceProjectionArtifact {
    pub fn validate_against_report(
        &self,
        report: &SyntaxGoSourceInstanceProjectionReport,
    ) -> Result<SourceInstanceProjectionArtifactSummary, SyntaxBackendBoundaryError> {
        report.validate()?;
        require_string(
            "projection.artifact_kind",
            &self.artifact_kind,
            PROJECTION_ARTIFACT_KIND,
        )?;
        require_u32("projection.version", self.version, VERSION)?;
        require_string("projection.tool", &self.tool, REPORT_TOOL)?;
        require_string("projection.status", &self.status, PROJECTION_STATUS)?;
        require_string("projection.target", &self.target, TARGET)?;
        require_string(
            "projection.candidate_grammar_id",
            &self.candidate_grammar_id,
            &report.candidate_grammar_id,
        )?;
        require_string(
            "projection.manifest_path",
            &self.manifest_path,
            &report.manifest_path,
        )?;
        self.parser.validate()?;
        require_projection_identity("projection.source", &self.source, &report.source)?;
        require_projection_identity("projection.metadata", &self.metadata, &report.metadata)?;
        require_projection_identity("projection.parser", &self.parser, &report.parser)?;
        require_projection_stats_match(
            "projection.stats.report",
            &self.stats,
            &report.projection_stats,
        )?;
        self.stats.validate()?;
        require_contains(
            "projection.claim_boundary",
            &self.claim_boundary,
            "parser-backed source-instance projection evidence only",
        )?;
        require_contains(
            "projection.claim_boundary",
            &self.claim_boundary,
            "generated-codec token production",
        )?;
        require_contains(
            "projection.claim_boundary",
            &self.claim_boundary,
            "arena-IR",
        )?;
        require_contains(
            "projection.claim_boundary",
            &self.claim_boundary,
            "stable-backend",
        )?;
        require_all(
            "projection.no_stable_claims",
            &self.no_stable_claims,
            REQUIRED_NO_STABLE_CLAIMS,
        )?;

        let observed_stats = self.root.observed_stats()?;
        require_projection_stats_match("projection.stats.tree", &self.stats, &observed_stats)?;
        require_string("projection.root.node_kind", &self.root.node_kind, "File")?;
        require_string("projection.root.rule_name", &self.root.rule_name, "File")?;
        Ok(SourceInstanceProjectionArtifactSummary {
            artifact_kind: PROJECTION_ARTIFACT_KIND.to_string(),
            source_sha256: self.source.sha256.clone(),
            metadata_sha256: self.metadata.sha256.clone(),
            candidate_grammar_id: self.candidate_grammar_id.clone(),
            root_node_kind: self.root.node_kind.clone(),
            node_count: observed_stats.node_count,
            field_count: observed_stats.field_count,
            field_value_count: observed_stats.field_value_count,
            terminal_payload_count: observed_stats.terminal_payload_count,
            optional_missing_count: observed_stats.optional_missing_count,
            repeated_empty_count: observed_stats.repeated_empty_count,
            max_depth: observed_stats.max_depth,
            distinct_node_kind_count: observed_stats.distinct_node_kinds.len(),
            next_missing_workstream_blocker: NEXT_MISSING_WORKSTREAM_BLOCKER.to_string(),
        })
    }

    pub fn codec_input_against_report(
        &self,
        report: &SyntaxGoSourceInstanceProjectionReport,
    ) -> Result<SourceInstanceProjectionCodecInput, SyntaxBackendBoundaryError> {
        let artifact_summary = self.validate_against_report(report)?;
        let mut builder = ProjectionCodecInputBuilder::default();
        self.root.append_codec_input_events(&mut builder)?;
        let events_sha256 = json_sha256(&builder.events).map_err(|error| {
            SyntaxBackendBoundaryError::new(
                "projection.codec_input.events_sha256",
                format!("failed to hash projection codec input events: {error}"),
            )
        })?;
        Ok(SourceInstanceProjectionCodecInput {
            artifact_kind: CODEC_INPUT_ARTIFACT_KIND.to_string(),
            version: VERSION,
            source_projection_artifact_kind: PROJECTION_ARTIFACT_KIND.to_string(),
            candidate_grammar_id: self.candidate_grammar_id.clone(),
            source_sha256: self.source.sha256.clone(),
            metadata_sha256: self.metadata.sha256.clone(),
            projection_node_count: artifact_summary.node_count,
            projection_field_value_count: artifact_summary.field_value_count,
            event_count: builder.events.len(),
            events_sha256,
            counts: builder.counts,
            events: builder.events,
            token_production_ready: false,
            next_missing_workstream_blocker: CODEC_INPUT_NEXT_MISSING_WORKSTREAM_BLOCKER
                .to_string(),
        })
    }

    pub fn token_layout_mapping_against_report(
        &self,
        report: &SyntaxGoSourceInstanceProjectionReport,
    ) -> Result<SourceInstanceProjectionTokenLayoutMapping, SyntaxBackendBoundaryError> {
        let codec_input = self.codec_input_against_report(report)?;
        let sidecar = source_instance_structural_model_sidecar(report)?;
        validate_source_instance_sidecar(&sidecar, report)?;
        let context = ProjectionTokenLayoutContext::from_sidecar(&sidecar)?;
        let mut builder = ProjectionTokenLayoutMappingBuilder::default();
        builder.append_node(&self.root, &context)?;
        let mapping_events_sha256 = json_sha256(&builder.events).map_err(|error| {
            SyntaxBackendBoundaryError::new(
                "projection.token_layout_mapping.events_sha256",
                format!("failed to hash token-layout mapping events: {error}"),
            )
        })?;

        Ok(SourceInstanceProjectionTokenLayoutMapping {
            artifact_kind: TOKEN_LAYOUT_MAPPING_ARTIFACT_KIND.to_string(),
            version: VERSION,
            status: TOKEN_LAYOUT_MAPPING_STATUS.to_string(),
            source_projection_artifact_kind: PROJECTION_ARTIFACT_KIND.to_string(),
            source_codec_input_artifact_kind: CODEC_INPUT_ARTIFACT_KIND.to_string(),
            candidate_grammar_id: self.candidate_grammar_id.clone(),
            source_sha256: self.source.sha256.clone(),
            metadata_sha256: self.metadata.sha256.clone(),
            structural_model_id: sidecar.structural_model.model_id.clone(),
            grammar_rule_count: sidecar.structural_model.records.len(),
            grammar_field_count: sidecar
                .structural_model
                .records
                .iter()
                .map(|record| record.fields.len())
                .sum(),
            token_layout: ProjectionTokenLayoutSummary::from_layout(
                &sidecar.structural_model.token_ranges,
            ),
            codec_input_event_count: codec_input.event_count,
            codec_input_events_sha256: codec_input.events_sha256,
            mapped_event_count: builder.events.len(),
            mapping_events_sha256,
            counts: builder.counts,
            mapped_events: builder.events,
            token_layout_mapping_ready: true,
            raw_token_ids_ready: false,
            generated_codec_report_ready: false,
            next_missing_workstream_blocker: TOKEN_LAYOUT_MAPPING_NEXT_MISSING_WORKSTREAM_BLOCKER
                .to_string(),
        })
    }

    pub fn raw_token_ids_against_report(
        &self,
        report: &SyntaxGoSourceInstanceProjectionReport,
    ) -> Result<SourceInstanceProjectionRawTokenIds, SyntaxBackendBoundaryError> {
        let mapping = self.token_layout_mapping_against_report(report)?;
        mapping.validate()?;
        let sidecar = source_instance_structural_model_sidecar(report)?;
        validate_source_instance_sidecar(&sidecar, report)?;
        let context = ProjectionTokenLayoutContext::from_sidecar(&sidecar)?;
        let mut builder = ProjectionRawTokenBuilder::new(&context)?;
        builder.append_node(&self.root, &context, None)?;

        let raw_token_ids_sha256 = json_sha256(&builder.raw_token_ids).map_err(|error| {
            SyntaxBackendBoundaryError::new(
                "projection.raw_token_ids.raw_token_ids_sha256",
                format!("failed to hash contextual raw token IDs: {error}"),
            )
        })?;
        let max_raw_token_id = builder.raw_token_ids.iter().copied().max();
        let raw_tokens_below_vocab_size = builder
            .raw_token_ids
            .iter()
            .all(|token| *token < mapping.token_layout.vocab_size);
        let pad_token_absent = builder
            .raw_token_ids
            .iter()
            .all(|token| *token != mapping.token_layout.pad_token);

        Ok(SourceInstanceProjectionRawTokenIds {
            artifact_kind: RAW_TOKEN_IDS_ARTIFACT_KIND.to_string(),
            version: VERSION,
            status: RAW_TOKEN_IDS_STATUS.to_string(),
            source_projection_artifact_kind: PROJECTION_ARTIFACT_KIND.to_string(),
            source_token_layout_mapping_artifact_kind: TOKEN_LAYOUT_MAPPING_ARTIFACT_KIND
                .to_string(),
            candidate_grammar_id: self.candidate_grammar_id.clone(),
            source_sha256: self.source.sha256.clone(),
            metadata_sha256: self.metadata.sha256.clone(),
            structural_model_id: mapping.structural_model_id,
            token_layout: mapping.token_layout,
            mapped_event_count: mapping.mapped_event_count,
            mapping_events_sha256: mapping.mapping_events_sha256,
            raw_token_count: builder.raw_token_ids.len(),
            raw_token_ids_sha256,
            max_raw_token_id,
            raw_tokens_below_vocab_size,
            pad_token_absent,
            counts: builder.counts,
            raw_token_segments: builder.segments,
            raw_token_ids: builder.raw_token_ids,
            raw_token_ids_ready: true,
            generated_codec_report_ready: false,
            next_missing_workstream_blocker: RAW_TOKEN_IDS_NEXT_MISSING_WORKSTREAM_BLOCKER
                .to_string(),
        })
    }

    /// Emit the contextual raw token IDs for this projection against an
    /// explicit structural-model sidecar instead of the per-file sidecar
    /// derived from a checked report.
    ///
    /// This is the unified-artifact emission leg: the divergence witness in
    /// `tests/syntax_go_unified_artifact_test.rs` runs one checked record
    /// against the pinned inventory sidecar
    /// (`syntax_go_unified_artifact::go_unified_grammar_artifact`) and pins
    /// the current blocker — projections embed per-file field shapes, so
    /// unified emission rejects with the field-shape drift until the
    /// projector produces inventory-shaped projections. The M1 Increment 3
    /// corpus switch re-points the production sidecar builders at the
    /// unified sidecar through this same emission core once that blocker is
    /// cleared.
    pub fn contextual_raw_token_ids_for_sidecar(
        &self,
        sidecar: &GrammarStructuralModelSidecar,
    ) -> Result<Vec<u32>, SyntaxBackendBoundaryError> {
        let context = ProjectionTokenLayoutContext::from_sidecar(sidecar)?;
        let mut builder = ProjectionRawTokenBuilder::new(&context)?;
        builder.append_node(&self.root, &context, None)?;
        Ok(builder.raw_token_ids)
    }

    pub fn generated_codec_report_against_report(
        &self,
        report: &SyntaxGoSourceInstanceProjectionReport,
    ) -> Result<SourceInstanceGeneratedCodecReport, SyntaxBackendBoundaryError> {
        let raw_tokens = self.raw_token_ids_against_report(report)?;
        raw_tokens.validate()?;
        // Canonical encoded stream: elide mask-forced tokens via the record's
        // compiled state machine; the expanded stream stays event/count
        // evidence.
        let sidecar = source_instance_structural_model_sidecar(report)?;
        let model = crate::syntax_state_machine::CompiledSyntaxModel::from_sidecar_and_layout(
            &sidecar,
            &raw_tokens.token_layout,
        )?;
        let tct_tokens = model
            .elide_forced_raw_tokens(&raw_tokens.raw_token_ids)
            .map_err(|error| {
                SyntaxBackendBoundaryError::new(
                    "projection.generated_codec_report.tct_tokens",
                    format!(
                        "failed to elide mask-forced tokens from the generated-codec report token stream: {error:?}"
                    ),
                )
            })?;
        let tct_tokens_sha256 = json_sha256(&tct_tokens).map_err(|error| {
            SyntaxBackendBoundaryError::new(
                "projection.generated_codec_report.tct_tokens_sha256",
                format!("failed to hash generated-codec report token stream: {error}"),
            )
        })?;
        let expanded_token_count = raw_tokens.raw_token_count;
        let forced_token_count = expanded_token_count - tct_tokens.len();

        Ok(SourceInstanceGeneratedCodecReport {
            artifact_kind: GENERATED_CODEC_REPORT_ARTIFACT_KIND.to_string(),
            version: VERSION,
            tool: TOOL.to_string(),
            status: GENERATED_CODEC_REPORT_STATUS.to_string(),
            target: TARGET.to_string(),
            route: GENERATED_CODEC_REPORT_ROUTE.to_string(),
            report_path: generated_codec_report_path_from_projection_output(
                &report.projection_output,
            )?,
            command: GENERATED_CODEC_REPORT_COMMAND.to_string(),
            candidate_grammar_id: self.candidate_grammar_id.clone(),
            manifest_path: self.manifest_path.clone(),
            source: self.source.clone(),
            metadata: self.metadata.clone(),
            parser: self.parser.clone(),
            projection_report_path: projection_report_path_from_validation_commands(
                &report.validation_commands,
            )?,
            source_projection_artifact_kind: PROJECTION_ARTIFACT_KIND.to_string(),
            source_projection_artifact_sha256: report.projection_sha256.clone(),
            raw_token_artifact_kind: RAW_TOKEN_IDS_ARTIFACT_KIND.to_string(),
            token_layout_mapping_artifact_kind: TOKEN_LAYOUT_MAPPING_ARTIFACT_KIND.to_string(),
            structural_model_id: raw_tokens.structural_model_id.clone(),
            token_layout: raw_tokens.token_layout.clone(),
            canonical_render: report.canonical_render.clone(),
            raw_token_count: tct_tokens.len(),
            raw_token_ids_sha256: tct_tokens_sha256.clone(),
            max_raw_token_id: tct_tokens.iter().copied().max(),
            raw_tokens_below_vocab_size: raw_tokens.raw_tokens_below_vocab_size,
            pad_token_absent: raw_tokens.pad_token_absent,
            stream_contract: crate::syntax_token_layout_diff::LIVE_RAW_SYNTAX_STREAM_CONTRACT
                .to_string(),
            expanded_token_count,
            forced_token_count,
            counts: raw_tokens.counts.clone(),
            tct_token_count: tct_tokens.len(),
            tct_tokens_sha256,
            tct_tokens,
            generated_codec_report_ready: true,
            report_ingestion_manifest_ready: false,
            training_manifest_promotion_ready: false,
            validation_commands: generated_codec_report_validation_commands(report),
            next_missing_workstream_blocker: format!(
                "add {} source-instance generated-codec report to next-version Go syntax corpus and report-ingestion manifests",
                report.manifest_path
            ),
            claim_boundary: "Parser-backed Go metadata lowered through GrammarIR, StructuralModel, source-file token layout, checked source-instance projection values, and contextual raw tokens now produces a target-sidecar generated-codec/source-route report. This is syntax-level report evidence only; it is not arena-IR evidence, not broad-corpus evidence, and not stable-backend promotion."
                .to_string(),
            no_stable_claims: REQUIRED_GENERATED_CODEC_REPORT_NO_STABLE_CLAIMS
                .iter()
                .map(|claim| (*claim).to_string())
                .collect(),
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SourceInstanceProjectionArtifactSummary {
    pub artifact_kind: String,
    pub source_sha256: String,
    pub metadata_sha256: String,
    pub candidate_grammar_id: String,
    pub root_node_kind: String,
    pub node_count: usize,
    pub field_count: usize,
    pub field_value_count: usize,
    pub terminal_payload_count: usize,
    pub optional_missing_count: usize,
    pub repeated_empty_count: usize,
    pub max_depth: usize,
    pub distinct_node_kind_count: usize,
    pub next_missing_workstream_blocker: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SourceInstanceProjectionCodecInput {
    pub artifact_kind: String,
    pub version: u32,
    pub source_projection_artifact_kind: String,
    pub candidate_grammar_id: String,
    pub source_sha256: String,
    pub metadata_sha256: String,
    pub projection_node_count: usize,
    pub projection_field_value_count: usize,
    pub event_count: usize,
    pub events_sha256: String,
    pub counts: ProjectionCodecInputCounts,
    pub events: Vec<ProjectionCodecInputEvent>,
    pub token_production_ready: bool,
    pub next_missing_workstream_blocker: String,
}

impl SourceInstanceProjectionCodecInput {
    pub fn validate(&self) -> Result<(), SyntaxBackendBoundaryError> {
        require_string(
            "projection.codec_input.artifact_kind",
            &self.artifact_kind,
            CODEC_INPUT_ARTIFACT_KIND,
        )?;
        require_u32("projection.codec_input.version", self.version, VERSION)?;
        require_string(
            "projection.codec_input.source_projection_artifact_kind",
            &self.source_projection_artifact_kind,
            PROJECTION_ARTIFACT_KIND,
        )?;
        validate_go_syntax_grammar_id(
            "projection.codec_input.candidate_grammar_id",
            &self.candidate_grammar_id,
        )?;
        require_sha256("projection.codec_input.source_sha256", &self.source_sha256)?;
        require_sha256(
            "projection.codec_input.metadata_sha256",
            &self.metadata_sha256,
        )?;
        require_positive_usize(
            "projection.codec_input.projection_node_count",
            self.projection_node_count,
        )?;
        require_positive_usize(
            "projection.codec_input.projection_field_value_count",
            self.projection_field_value_count,
        )?;
        require_usize(
            "projection.codec_input.event_count",
            self.event_count,
            self.events.len(),
        )?;
        require_sha256("projection.codec_input.events_sha256", &self.events_sha256)?;
        let actual_hash = json_sha256(&self.events).map_err(|error| {
            SyntaxBackendBoundaryError::new(
                "projection.codec_input.events_sha256",
                format!("failed to hash projection codec input events: {error}"),
            )
        })?;
        require_string(
            "projection.codec_input.events_sha256",
            &self.events_sha256,
            &actual_hash,
        )?;
        self.counts
            .validate(self.projection_node_count, self.event_count)?;
        require_bool(
            "projection.codec_input.token_production_ready",
            self.token_production_ready,
            false,
        )?;
        require_string(
            "projection.codec_input.next_missing_workstream_blocker",
            &self.next_missing_workstream_blocker,
            CODEC_INPUT_NEXT_MISSING_WORKSTREAM_BLOCKER,
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SourceInstanceProjectionTokenLayoutMapping {
    pub artifact_kind: String,
    pub version: u32,
    pub status: String,
    pub source_projection_artifact_kind: String,
    pub source_codec_input_artifact_kind: String,
    pub candidate_grammar_id: String,
    pub source_sha256: String,
    pub metadata_sha256: String,
    pub structural_model_id: String,
    pub grammar_rule_count: usize,
    pub grammar_field_count: usize,
    pub token_layout: ProjectionTokenLayoutSummary,
    pub codec_input_event_count: usize,
    pub codec_input_events_sha256: String,
    pub mapped_event_count: usize,
    pub mapping_events_sha256: String,
    pub counts: ProjectionTokenLayoutMappingCounts,
    pub mapped_events: Vec<ProjectionTokenLayoutMappedEvent>,
    pub token_layout_mapping_ready: bool,
    pub raw_token_ids_ready: bool,
    pub generated_codec_report_ready: bool,
    pub next_missing_workstream_blocker: String,
}

impl SourceInstanceProjectionTokenLayoutMapping {
    pub fn validate(&self) -> Result<(), SyntaxBackendBoundaryError> {
        require_string(
            "projection.token_layout_mapping.artifact_kind",
            &self.artifact_kind,
            TOKEN_LAYOUT_MAPPING_ARTIFACT_KIND,
        )?;
        require_u32(
            "projection.token_layout_mapping.version",
            self.version,
            VERSION,
        )?;
        require_string(
            "projection.token_layout_mapping.status",
            &self.status,
            TOKEN_LAYOUT_MAPPING_STATUS,
        )?;
        require_string(
            "projection.token_layout_mapping.source_projection_artifact_kind",
            &self.source_projection_artifact_kind,
            PROJECTION_ARTIFACT_KIND,
        )?;
        require_string(
            "projection.token_layout_mapping.source_codec_input_artifact_kind",
            &self.source_codec_input_artifact_kind,
            CODEC_INPUT_ARTIFACT_KIND,
        )?;
        validate_go_syntax_grammar_id(
            "projection.token_layout_mapping.candidate_grammar_id",
            &self.candidate_grammar_id,
        )?;
        require_sha256(
            "projection.token_layout_mapping.source_sha256",
            &self.source_sha256,
        )?;
        require_sha256(
            "projection.token_layout_mapping.metadata_sha256",
            &self.metadata_sha256,
        )?;
        require_string(
            "projection.token_layout_mapping.structural_model_id",
            &self.structural_model_id,
            &format!(
                "grammar-ir:{}:structural-model-v0",
                self.candidate_grammar_id
            ),
        )?;
        require_positive_usize(
            "projection.token_layout_mapping.grammar_rule_count",
            self.grammar_rule_count,
        )?;
        require_positive_usize(
            "projection.token_layout_mapping.grammar_field_count",
            self.grammar_field_count,
        )?;
        self.token_layout.validate()?;
        require_positive_usize(
            "projection.token_layout_mapping.codec_input_event_count",
            self.codec_input_event_count,
        )?;
        require_sha256(
            "projection.token_layout_mapping.codec_input_events_sha256",
            &self.codec_input_events_sha256,
        )?;
        require_usize(
            "projection.token_layout_mapping.mapped_event_count",
            self.mapped_event_count,
            self.mapped_events.len(),
        )?;
        require_usize(
            "projection.token_layout_mapping.mapped_event_count",
            self.mapped_event_count,
            self.codec_input_event_count,
        )?;
        require_sha256(
            "projection.token_layout_mapping.mapping_events_sha256",
            &self.mapping_events_sha256,
        )?;
        let actual_hash = json_sha256(&self.mapped_events).map_err(|error| {
            SyntaxBackendBoundaryError::new(
                "projection.token_layout_mapping.mapping_events_sha256",
                format!("failed to hash token-layout mapping events: {error}"),
            )
        })?;
        require_string(
            "projection.token_layout_mapping.mapping_events_sha256",
            &self.mapping_events_sha256,
            &actual_hash,
        )?;
        self.counts.validate(self.mapped_event_count)?;
        require_bool(
            "projection.token_layout_mapping.token_layout_mapping_ready",
            self.token_layout_mapping_ready,
            true,
        )?;
        require_bool(
            "projection.token_layout_mapping.raw_token_ids_ready",
            self.raw_token_ids_ready,
            false,
        )?;
        require_bool(
            "projection.token_layout_mapping.generated_codec_report_ready",
            self.generated_codec_report_ready,
            false,
        )?;
        require_string(
            "projection.token_layout_mapping.next_missing_workstream_blocker",
            &self.next_missing_workstream_blocker,
            TOKEN_LAYOUT_MAPPING_NEXT_MISSING_WORKSTREAM_BLOCKER,
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SourceInstanceProjectionRawTokenIds {
    pub artifact_kind: String,
    pub version: u32,
    pub status: String,
    pub source_projection_artifact_kind: String,
    pub source_token_layout_mapping_artifact_kind: String,
    pub candidate_grammar_id: String,
    pub source_sha256: String,
    pub metadata_sha256: String,
    pub structural_model_id: String,
    pub token_layout: ProjectionTokenLayoutSummary,
    pub mapped_event_count: usize,
    pub mapping_events_sha256: String,
    pub raw_token_count: usize,
    pub raw_token_ids_sha256: String,
    pub max_raw_token_id: Option<u32>,
    pub raw_tokens_below_vocab_size: bool,
    pub pad_token_absent: bool,
    pub counts: ProjectionRawTokenCounts,
    pub raw_token_segments: Vec<ProjectionRawTokenSegment>,
    pub raw_token_ids: Vec<u32>,
    pub raw_token_ids_ready: bool,
    pub generated_codec_report_ready: bool,
    pub next_missing_workstream_blocker: String,
}

impl SourceInstanceProjectionRawTokenIds {
    pub fn validate(&self) -> Result<(), SyntaxBackendBoundaryError> {
        require_string(
            "projection.raw_token_ids.artifact_kind",
            &self.artifact_kind,
            RAW_TOKEN_IDS_ARTIFACT_KIND,
        )?;
        require_u32("projection.raw_token_ids.version", self.version, VERSION)?;
        require_string(
            "projection.raw_token_ids.status",
            &self.status,
            RAW_TOKEN_IDS_STATUS,
        )?;
        require_string(
            "projection.raw_token_ids.source_projection_artifact_kind",
            &self.source_projection_artifact_kind,
            PROJECTION_ARTIFACT_KIND,
        )?;
        require_string(
            "projection.raw_token_ids.source_token_layout_mapping_artifact_kind",
            &self.source_token_layout_mapping_artifact_kind,
            TOKEN_LAYOUT_MAPPING_ARTIFACT_KIND,
        )?;
        validate_go_syntax_grammar_id(
            "projection.raw_token_ids.candidate_grammar_id",
            &self.candidate_grammar_id,
        )?;
        require_sha256(
            "projection.raw_token_ids.source_sha256",
            &self.source_sha256,
        )?;
        require_sha256(
            "projection.raw_token_ids.metadata_sha256",
            &self.metadata_sha256,
        )?;
        require_string(
            "projection.raw_token_ids.structural_model_id",
            &self.structural_model_id,
            &format!(
                "grammar-ir:{}:structural-model-v0",
                self.candidate_grammar_id
            ),
        )?;
        self.token_layout.validate()?;
        require_positive_usize(
            "projection.raw_token_ids.mapped_event_count",
            self.mapped_event_count,
        )?;
        require_sha256(
            "projection.raw_token_ids.mapping_events_sha256",
            &self.mapping_events_sha256,
        )?;
        require_usize(
            "projection.raw_token_ids.raw_token_count",
            self.raw_token_count,
            self.raw_token_ids.len(),
        )?;
        require_sha256(
            "projection.raw_token_ids.raw_token_ids_sha256",
            &self.raw_token_ids_sha256,
        )?;
        let actual_hash = json_sha256(&self.raw_token_ids).map_err(|error| {
            SyntaxBackendBoundaryError::new(
                "projection.raw_token_ids.raw_token_ids_sha256",
                format!("failed to hash contextual raw token IDs: {error}"),
            )
        })?;
        require_string(
            "projection.raw_token_ids.raw_token_ids_sha256",
            &self.raw_token_ids_sha256,
            &actual_hash,
        )?;
        if self.max_raw_token_id != self.raw_token_ids.iter().copied().max() {
            return Err(SyntaxBackendBoundaryError::new(
                "projection.raw_token_ids.max_raw_token_id",
                "max raw token ID must match the emitted token stream",
            ));
        }
        require_bool(
            "projection.raw_token_ids.raw_tokens_below_vocab_size",
            self.raw_tokens_below_vocab_size,
            true,
        )?;
        if !self
            .raw_token_ids
            .iter()
            .all(|token| *token < self.token_layout.vocab_size)
        {
            return Err(SyntaxBackendBoundaryError::new(
                "projection.raw_token_ids.raw_token_ids",
                "every raw token ID must be below VOCAB_SIZE",
            ));
        }
        require_bool(
            "projection.raw_token_ids.pad_token_absent",
            self.pad_token_absent,
            true,
        )?;
        if self.raw_token_ids.contains(&self.token_layout.pad_token) {
            return Err(SyntaxBackendBoundaryError::new(
                "projection.raw_token_ids.raw_token_ids",
                "contextual raw token stream must not include PAD_TOKEN",
            ));
        }
        self.counts.validate(self.raw_token_count)?;
        require_usize(
            "projection.raw_token_ids.raw_token_segments",
            self.raw_token_segments.len(),
            self.mapped_event_count,
        )?;
        validate_raw_token_segments(&self.raw_token_segments, self.raw_token_count)?;
        require_bool(
            "projection.raw_token_ids.raw_token_ids_ready",
            self.raw_token_ids_ready,
            true,
        )?;
        require_bool(
            "projection.raw_token_ids.generated_codec_report_ready",
            self.generated_codec_report_ready,
            false,
        )?;
        require_string(
            "projection.raw_token_ids.next_missing_workstream_blocker",
            &self.next_missing_workstream_blocker,
            RAW_TOKEN_IDS_NEXT_MISSING_WORKSTREAM_BLOCKER,
        )
    }
}

#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProjectionRawTokenCounts {
    pub structural_event_token_count: usize,
    pub terminal_decision_token_count: usize,
    pub utf8_payload_byte_token_count: usize,
    pub integer_payload_byte_token_count: usize,
    pub boolean_payload_token_count: usize,
    pub payload_end_token_count: usize,
}

impl ProjectionRawTokenCounts {
    fn validate(&self, raw_token_count: usize) -> Result<(), SyntaxBackendBoundaryError> {
        require_usize(
            "projection.raw_token_ids.counts.payload_end_token_count",
            self.payload_end_token_count,
            self.terminal_decision_token_count
                .checked_sub(self.boolean_payload_token_count)
                .ok_or_else(|| {
                    SyntaxBackendBoundaryError::new(
                        "projection.raw_token_ids.counts.payload_end_token_count",
                        "boolean payload tokens cannot exceed terminal decision tokens",
                    )
                })?,
        )?;
        require_usize(
            "projection.raw_token_ids.counts.total",
            self.structural_event_token_count
                + self.terminal_decision_token_count
                + self.utf8_payload_byte_token_count
                + self.integer_payload_byte_token_count
                + self.boolean_payload_token_count
                + self.payload_end_token_count,
            raw_token_count,
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProjectionRawTokenSegment {
    pub event_ordinal: usize,
    pub event_kind: String,
    pub primary_token_range_role: String,
    pub raw_token_offset: usize,
    pub raw_token_count: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub owner_rule: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub owner_field: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub node_kind: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub payload_kind: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SourceInstanceGeneratedCodecReport {
    pub artifact_kind: String,
    pub version: u32,
    pub tool: String,
    pub status: String,
    pub target: String,
    pub route: String,
    pub report_path: String,
    pub command: String,
    pub candidate_grammar_id: String,
    pub manifest_path: String,
    pub source: SourceInstanceProjectionSource,
    pub metadata: SourceInstanceProjectionMetadata,
    pub parser: SourceInstanceProjectionParser,
    pub projection_report_path: String,
    pub source_projection_artifact_kind: String,
    pub source_projection_artifact_sha256: String,
    pub raw_token_artifact_kind: String,
    pub token_layout_mapping_artifact_kind: String,
    pub structural_model_id: String,
    pub token_layout: ProjectionTokenLayoutSummary,
    pub canonical_render: SourceInstanceCanonicalRender,
    pub raw_token_count: usize,
    pub raw_token_ids_sha256: String,
    pub max_raw_token_id: Option<u32>,
    pub raw_tokens_below_vocab_size: bool,
    pub pad_token_absent: bool,
    /// Canonical encoded stream contract (`LIVE_RAW_SYNTAX_STREAM_CONTRACT`,
    /// elided-v1): `tct_tokens` carries no mask-forced tokens.
    pub stream_contract: String,
    /// Expanded (state-machine-native) stream length; `counts` event totals
    /// are defined over this stream.
    pub expanded_token_count: usize,
    /// Mask-forced tokens elided from the canonical stream.
    pub forced_token_count: usize,
    pub counts: ProjectionRawTokenCounts,
    pub tct_token_count: usize,
    pub tct_tokens_sha256: String,
    pub tct_tokens: Vec<u32>,
    pub generated_codec_report_ready: bool,
    pub report_ingestion_manifest_ready: bool,
    pub training_manifest_promotion_ready: bool,
    pub validation_commands: Vec<String>,
    pub next_missing_workstream_blocker: String,
    pub claim_boundary: String,
    pub no_stable_claims: Vec<String>,
}

impl SourceInstanceGeneratedCodecReport {
    pub fn validate(&self) -> Result<(), SyntaxBackendBoundaryError> {
        let projection_report = load_checked_projection_report(&self.projection_report_path)?;
        require_string(
            "projection.generated_codec_report.artifact_kind",
            &self.artifact_kind,
            GENERATED_CODEC_REPORT_ARTIFACT_KIND,
        )?;
        require_u32(
            "projection.generated_codec_report.version",
            self.version,
            VERSION,
        )?;
        require_string("projection.generated_codec_report.tool", &self.tool, TOOL)?;
        require_string(
            "projection.generated_codec_report.status",
            &self.status,
            GENERATED_CODEC_REPORT_STATUS,
        )?;
        require_string(
            "projection.generated_codec_report.target",
            &self.target,
            TARGET,
        )?;
        require_string(
            "projection.generated_codec_report.route",
            &self.route,
            GENERATED_CODEC_REPORT_ROUTE,
        )?;
        require_string(
            "projection.generated_codec_report.report_path",
            &self.report_path,
            &generated_codec_report_path_from_projection_output(
                &projection_report.projection_output,
            )?,
        )?;
        if !self.report_path.starts_with("target/") {
            return Err(SyntaxBackendBoundaryError::new(
                "projection.generated_codec_report.report_path",
                "generated-codec report must remain target/ sidecar evidence until manifest promotion",
            ));
        }
        require_string(
            "projection.generated_codec_report.command",
            &self.command,
            GENERATED_CODEC_REPORT_COMMAND,
        )?;
        require_string(
            "projection.generated_codec_report.candidate_grammar_id",
            &self.candidate_grammar_id,
            &projection_report.candidate_grammar_id,
        )?;
        require_string(
            "projection.generated_codec_report.manifest_path",
            &self.manifest_path,
            &projection_report.manifest_path,
        )?;
        require_projection_identity(
            "projection.generated_codec_report.source",
            &self.source,
            &projection_report.source,
        )?;
        require_projection_identity(
            "projection.generated_codec_report.metadata",
            &self.metadata,
            &projection_report.metadata,
        )?;
        self.parser.validate()?;
        require_projection_identity(
            "projection.generated_codec_report.parser",
            &self.parser,
            &projection_report.parser,
        )?;
        require_non_empty(
            "projection.generated_codec_report.projection_report_path",
            &self.projection_report_path,
        )?;
        require_string(
            "projection.generated_codec_report.source_projection_artifact_kind",
            &self.source_projection_artifact_kind,
            PROJECTION_ARTIFACT_KIND,
        )?;
        require_sha256(
            "projection.generated_codec_report.source_projection_artifact_sha256",
            &self.source_projection_artifact_sha256,
        )?;
        require_string(
            "projection.generated_codec_report.source_projection_artifact_sha256",
            &self.source_projection_artifact_sha256,
            &projection_report.projection_sha256,
        )?;
        require_string(
            "projection.generated_codec_report.raw_token_artifact_kind",
            &self.raw_token_artifact_kind,
            RAW_TOKEN_IDS_ARTIFACT_KIND,
        )?;
        require_string(
            "projection.generated_codec_report.token_layout_mapping_artifact_kind",
            &self.token_layout_mapping_artifact_kind,
            TOKEN_LAYOUT_MAPPING_ARTIFACT_KIND,
        )?;
        require_string(
            "projection.generated_codec_report.structural_model_id",
            &self.structural_model_id,
            &format!(
                "grammar-ir:{}:structural-model-v0",
                self.candidate_grammar_id
            ),
        )?;
        self.token_layout.validate()?;
        self.canonical_render.validate(&self.source)?;
        require_projection_identity(
            "projection.generated_codec_report.canonical_render",
            &self.canonical_render,
            &projection_report.canonical_render,
        )?;
        require_usize(
            "projection.generated_codec_report.raw_token_count",
            self.raw_token_count,
            self.tct_tokens.len(),
        )?;
        require_sha256(
            "projection.generated_codec_report.raw_token_ids_sha256",
            &self.raw_token_ids_sha256,
        )?;
        require_sha256(
            "projection.generated_codec_report.tct_tokens_sha256",
            &self.tct_tokens_sha256,
        )?;
        let actual_hash = json_sha256(&self.tct_tokens).map_err(|error| {
            SyntaxBackendBoundaryError::new(
                "projection.generated_codec_report.tct_tokens_sha256",
                format!("failed to hash generated-codec report token stream: {error}"),
            )
        })?;
        require_string(
            "projection.generated_codec_report.raw_token_ids_sha256",
            &self.raw_token_ids_sha256,
            &actual_hash,
        )?;
        require_string(
            "projection.generated_codec_report.tct_tokens_sha256",
            &self.tct_tokens_sha256,
            &actual_hash,
        )?;
        if self.max_raw_token_id != self.tct_tokens.iter().copied().max() {
            return Err(SyntaxBackendBoundaryError::new(
                "projection.generated_codec_report.max_raw_token_id",
                "max raw token ID must match the generated-codec report token stream",
            ));
        }
        require_bool(
            "projection.generated_codec_report.raw_tokens_below_vocab_size",
            self.raw_tokens_below_vocab_size,
            true,
        )?;
        if !self
            .tct_tokens
            .iter()
            .all(|token| *token < self.token_layout.vocab_size)
        {
            return Err(SyntaxBackendBoundaryError::new(
                "projection.generated_codec_report.tct_tokens",
                "every generated-codec report token ID must be below VOCAB_SIZE",
            ));
        }
        require_bool(
            "projection.generated_codec_report.pad_token_absent",
            self.pad_token_absent,
            true,
        )?;
        if self.tct_tokens.contains(&self.token_layout.pad_token) {
            return Err(SyntaxBackendBoundaryError::new(
                "projection.generated_codec_report.tct_tokens",
                "generated-codec report token stream must not include PAD_TOKEN",
            ));
        }
        require_string(
            "projection.generated_codec_report.stream_contract",
            &self.stream_contract,
            crate::syntax_token_layout_diff::LIVE_RAW_SYNTAX_STREAM_CONTRACT,
        )?;
        require_usize(
            "projection.generated_codec_report.forced_token_count",
            self.forced_token_count,
            self.expanded_token_count - self.tct_token_count,
        )?;
        self.counts.validate(self.expanded_token_count)?;
        require_usize(
            "projection.generated_codec_report.tct_token_count",
            self.tct_token_count,
            self.raw_token_count,
        )?;
        require_bool(
            "projection.generated_codec_report.generated_codec_report_ready",
            self.generated_codec_report_ready,
            true,
        )?;
        require_bool(
            "projection.generated_codec_report.report_ingestion_manifest_ready",
            self.report_ingestion_manifest_ready,
            false,
        )?;
        require_bool(
            "projection.generated_codec_report.training_manifest_promotion_ready",
            self.training_manifest_promotion_ready,
            false,
        )?;
        validate_generated_codec_report_commands(&self.validation_commands, &projection_report)?;
        require_string(
            "projection.generated_codec_report.next_missing_workstream_blocker",
            &self.next_missing_workstream_blocker,
            &format!(
                "add {} source-instance generated-codec report to next-version Go syntax corpus and report-ingestion manifests",
                projection_report.manifest_path
            ),
        )?;
        require_contains(
            "projection.generated_codec_report.claim_boundary",
            &self.claim_boundary,
            "Parser-backed Go metadata",
        )?;
        require_contains(
            "projection.generated_codec_report.claim_boundary",
            &self.claim_boundary,
            "GrammarIR",
        )?;
        require_contains(
            "projection.generated_codec_report.claim_boundary",
            &self.claim_boundary,
            "StructuralModel",
        )?;
        require_contains(
            "projection.generated_codec_report.claim_boundary",
            &self.claim_boundary,
            "contextual raw tokens",
        )?;
        require_contains(
            "projection.generated_codec_report.claim_boundary",
            &self.claim_boundary,
            "not arena-IR",
        )?;
        require_contains(
            "projection.generated_codec_report.claim_boundary",
            &self.claim_boundary,
            "not stable-backend",
        )?;
        require_all(
            "projection.generated_codec_report.no_stable_claims",
            &self.no_stable_claims,
            REQUIRED_GENERATED_CODEC_REPORT_NO_STABLE_CLAIMS,
        )
    }
}

#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProjectionTokenLayoutMappingCounts {
    pub structural_event_count: usize,
    pub payload_event_count: usize,
    pub utf8_payload_event_count: usize,
    pub integer_payload_event_count: usize,
    pub integer_payload_overflow_count: usize,
    pub max_integer_payload_value: Option<i64>,
    pub boolean_payload_event_count: usize,
    pub utf8_payload_byte_count: usize,
    pub integer_payload_byte_count: usize,
    pub payload_byte_count: usize,
}

impl ProjectionTokenLayoutMappingCounts {
    fn validate(&self, mapped_event_count: usize) -> Result<(), SyntaxBackendBoundaryError> {
        require_usize(
            "projection.token_layout_mapping.counts.utf8_payload_event_count",
            self.utf8_payload_event_count,
            self.payload_event_count
                .checked_sub(self.integer_payload_event_count)
                .and_then(|count| count.checked_sub(self.boolean_payload_event_count))
                .ok_or_else(|| {
                    SyntaxBackendBoundaryError::new(
                        "projection.token_layout_mapping.counts.utf8_payload_event_count",
                        "integer and boolean payload events cannot exceed total payload events",
                    )
                })?,
        )?;
        require_usize(
            "projection.token_layout_mapping.counts.payload_byte_count",
            self.payload_byte_count,
            self.utf8_payload_byte_count + self.integer_payload_byte_count,
        )?;
        require_usize(
            "projection.token_layout_mapping.counts.total",
            self.structural_event_count + self.payload_event_count,
            mapped_event_count,
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProjectionTokenLayoutMappedEvent {
    pub ordinal: usize,
    pub event_kind: String,
    pub token_range_id: String,
    pub token_range_role: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub owner_rule: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub owner_field: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub node_kind: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub decision_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub payload_kind: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub payload_byte_len: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub integer_value: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub payload_fits_token_range: Option<bool>,
}

#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProjectionCodecInputCounts {
    pub node_start_count: usize,
    pub node_end_count: usize,
    pub field_start_count: usize,
    pub field_end_count: usize,
    pub terminal_count: usize,
    pub missing_field_count: usize,
    pub repeated_empty_field_count: usize,
}

impl ProjectionCodecInputCounts {
    fn validate(
        &self,
        projection_node_count: usize,
        event_count: usize,
    ) -> Result<(), SyntaxBackendBoundaryError> {
        require_usize(
            "projection.codec_input.counts.node_start_count",
            self.node_start_count,
            projection_node_count,
        )?;
        require_usize(
            "projection.codec_input.counts.node_end_count",
            self.node_end_count,
            projection_node_count,
        )?;
        require_usize(
            "projection.codec_input.counts.field_end_count",
            self.field_end_count,
            self.field_start_count,
        )?;
        let counted_events = self.node_start_count
            + self.node_end_count
            + self.field_start_count
            + self.field_end_count
            + self.terminal_count
            + self.missing_field_count
            + self.repeated_empty_field_count;
        require_usize(
            "projection.codec_input.counts.total",
            counted_events,
            event_count,
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProjectionCodecInputEvent {
    pub kind: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub node_kind: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rule_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub field_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub required: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub multiple: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub terminal: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub payload: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub token_name: Option<String>,
}

impl ProjectionCodecInputEvent {
    fn node_start(node_kind: &str, rule_name: &str) -> Self {
        Self {
            kind: "node_start".to_string(),
            node_kind: Some(node_kind.to_string()),
            rule_name: Some(rule_name.to_string()),
            field_name: None,
            required: None,
            multiple: None,
            terminal: None,
            payload: None,
            token_name: None,
        }
    }

    fn node_end(node_kind: &str, rule_name: &str) -> Self {
        Self {
            kind: "node_end".to_string(),
            node_kind: Some(node_kind.to_string()),
            rule_name: Some(rule_name.to_string()),
            field_name: None,
            required: None,
            multiple: None,
            terminal: None,
            payload: None,
            token_name: None,
        }
    }

    fn field_start(field: &SourceInstanceProjectionField) -> Self {
        Self {
            kind: "field_start".to_string(),
            node_kind: None,
            rule_name: None,
            field_name: Some(field.name.clone()),
            required: Some(field.required),
            multiple: Some(field.multiple),
            terminal: field.terminal.clone(),
            payload: None,
            token_name: None,
        }
    }

    fn field_end(field_name: &str) -> Self {
        Self {
            kind: "field_end".to_string(),
            node_kind: None,
            rule_name: None,
            field_name: Some(field_name.to_string()),
            required: None,
            multiple: None,
            terminal: None,
            payload: None,
            token_name: None,
        }
    }

    fn field_missing(field_name: &str) -> Self {
        Self {
            kind: "field_missing".to_string(),
            node_kind: None,
            rule_name: None,
            field_name: Some(field_name.to_string()),
            required: None,
            multiple: None,
            terminal: None,
            payload: None,
            token_name: None,
        }
    }

    fn field_repeated_empty(field_name: &str) -> Self {
        Self {
            kind: "field_repeated_empty".to_string(),
            node_kind: None,
            rule_name: None,
            field_name: Some(field_name.to_string()),
            required: None,
            multiple: None,
            terminal: None,
            payload: None,
            token_name: None,
        }
    }

    fn terminal(value: &SourceInstanceProjectionValue) -> Result<Self, SyntaxBackendBoundaryError> {
        Ok(Self {
            kind: "terminal".to_string(),
            node_kind: Some(value.node_kind.clone()),
            rule_name: None,
            field_name: None,
            required: None,
            multiple: None,
            terminal: Some(value.terminal.clone().ok_or_else(|| {
                SyntaxBackendBoundaryError::new(
                    "projection.codec_input.terminal",
                    "terminal events require a terminal kind",
                )
            })?),
            payload: Some(value.payload.clone().ok_or_else(|| {
                SyntaxBackendBoundaryError::new(
                    "projection.codec_input.payload",
                    "terminal events require a payload",
                )
            })?),
            token_name: value.token_name.clone(),
        })
    }
}

#[derive(Debug, Default, Clone, PartialEq, Eq)]
struct ProjectionTokenLayoutMappingBuilder {
    events: Vec<ProjectionTokenLayoutMappedEvent>,
    counts: ProjectionTokenLayoutMappingCounts,
}

impl ProjectionTokenLayoutMappingBuilder {
    fn append_node(
        &mut self,
        node: &SourceInstanceProjectionNode,
        context: &ProjectionTokenLayoutContext,
    ) -> Result<(), SyntaxBackendBoundaryError> {
        context.require_record(&node.rule_name, &node.node_kind)?;
        self.push_structural_event(
            "node_start",
            Some(&node.rule_name),
            None,
            Some(&node.node_kind),
            Some(format!("decision.rule.{}", node.rule_name)),
            context,
        );
        for field in &node.fields {
            self.append_field(&node.rule_name, field, context)?;
        }
        self.push_structural_event(
            "node_end",
            Some(&node.rule_name),
            None,
            Some(&node.node_kind),
            Some(format!("codec.control.rule.{}.end", node.rule_name)),
            context,
        );
        Ok(())
    }

    fn append_field(
        &mut self,
        owner_rule: &str,
        field: &SourceInstanceProjectionField,
        context: &ProjectionTokenLayoutContext,
    ) -> Result<(), SyntaxBackendBoundaryError> {
        field.ensure_presence_contract()?;
        context.require_field(owner_rule, field)?;
        self.push_structural_event(
            "field_start",
            Some(owner_rule),
            Some(&field.name),
            None,
            Some(format!("decision.rule.{owner_rule}.field.{}", field.name)),
            context,
        );

        if field.missing {
            context.require_optional(owner_rule, &field.name)?;
            self.push_structural_event(
                "field_missing",
                Some(owner_rule),
                Some(&field.name),
                None,
                Some(format!(
                    "codec.control.rule.{owner_rule}.field.{}.missing",
                    field.name
                )),
                context,
            );
        } else if field.multiple && field.values.is_empty() {
            context.require_list(owner_rule, &field.name)?;
            self.push_structural_event(
                "field_repeated_empty",
                Some(owner_rule),
                Some(&field.name),
                None,
                Some(format!(
                    "codec.control.rule.{owner_rule}.field.{}.repeated_empty",
                    field.name
                )),
                context,
            );
        } else {
            for value in &field.values {
                self.append_value(owner_rule, field, value, context)?;
            }
        }

        self.push_structural_event(
            "field_end",
            Some(owner_rule),
            Some(&field.name),
            None,
            Some(format!(
                "codec.control.rule.{owner_rule}.field.{}.end",
                field.name
            )),
            context,
        );
        Ok(())
    }

    fn append_value(
        &mut self,
        owner_rule: &str,
        field: &SourceInstanceProjectionField,
        value: &SourceInstanceProjectionValue,
        context: &ProjectionTokenLayoutContext,
    ) -> Result<(), SyntaxBackendBoundaryError> {
        match value.kind.as_str() {
            "node" => {
                context.require_reference(owner_rule, &field.name, &value.node_kind)?;
                let node = value.node.as_ref().ok_or_else(|| {
                    SyntaxBackendBoundaryError::new(
                        "projection.token_layout_mapping.node",
                        "node projection values must carry a child node",
                    )
                })?;
                self.append_node(node, context)
            }
            "terminal" => self.append_terminal(owner_rule, field, value, context),
            other => Err(SyntaxBackendBoundaryError::new(
                "projection.token_layout_mapping.value_kind",
                format!("unexpected projection value kind {other}"),
            )),
        }
    }

    fn append_terminal(
        &mut self,
        owner_rule: &str,
        field: &SourceInstanceProjectionField,
        value: &SourceInstanceProjectionValue,
        context: &ProjectionTokenLayoutContext,
    ) -> Result<(), SyntaxBackendBoundaryError> {
        let terminal = value.terminal.as_deref().ok_or_else(|| {
            SyntaxBackendBoundaryError::new(
                "projection.token_layout_mapping.terminal",
                "terminal projection values must name a terminal payload kind",
            )
        })?;
        if field.terminal.as_deref() != Some(terminal) {
            return Err(SyntaxBackendBoundaryError::new(
                "projection.token_layout_mapping.terminal",
                format!(
                    "field {} terminal {:?} does not match value terminal {terminal}",
                    field.name, field.terminal
                ),
            ));
        }
        let payload = value.payload.as_deref().ok_or_else(|| {
            SyntaxBackendBoundaryError::new(
                "projection.token_layout_mapping.payload",
                "terminal projection values must carry a payload",
            )
        })?;
        let payload_kind = terminal_payload_kind(terminal)?;
        context.require_payload(owner_rule, &field.name, &value.node_kind, terminal)?;

        match terminal {
            "identifier_payload" | "string_payload" => {
                self.counts.utf8_payload_event_count += 1;
                self.counts.utf8_payload_byte_count += payload.len();
                self.counts.payload_byte_count += payload.len();
                self.push_payload_event(
                    owner_rule,
                    &field.name,
                    &value.node_kind,
                    terminal,
                    payload_kind,
                    Some(payload.len()),
                    None,
                    true,
                    context.utf8_range.clone(),
                );
                Ok(())
            }
            "integer_payload" => {
                let integer = payload.parse::<i64>().ok();
                let fits_range = match context.integer_encoding {
                    ProjectionIntegerPayloadEncoding::Utf8Lexeme => true,
                    ProjectionIntegerPayloadEncoding::U8Value {
                        valid_min,
                        valid_max,
                    } => {
                        let integer = integer.ok_or_else(|| {
                            SyntaxBackendBoundaryError::new(
                                "projection.token_layout_mapping.integer_payload",
                                format!(
                                    "integer payload {payload:?} is not a decimal integer for u8 value encoding"
                                ),
                            )
                        })?;
                        integer >= valid_min && integer <= valid_max
                    }
                };
                if !fits_range {
                    self.counts.integer_payload_overflow_count += 1;
                }
                self.counts.integer_payload_event_count += 1;
                self.counts.integer_payload_byte_count += payload.len();
                self.counts.payload_byte_count += payload.len();
                if let Some(integer) = integer {
                    self.counts.max_integer_payload_value = Some(
                        self.counts
                            .max_integer_payload_value
                            .map_or(integer, |max| max.max(integer)),
                    );
                }
                self.push_payload_event(
                    owner_rule,
                    &field.name,
                    &value.node_kind,
                    terminal,
                    payload_kind,
                    None,
                    integer,
                    fits_range,
                    context.integer_range.clone(),
                );
                Ok(())
            }
            "boolean_payload" => {
                if payload != "true" && payload != "false" {
                    return Err(SyntaxBackendBoundaryError::new(
                        "projection.token_layout_mapping.boolean_payload",
                        format!("boolean payload {payload:?} is not true or false"),
                    ));
                }
                self.counts.boolean_payload_event_count += 1;
                self.push_payload_event(
                    owner_rule,
                    &field.name,
                    &value.node_kind,
                    terminal,
                    payload_kind,
                    None,
                    None,
                    true,
                    context.structural_range.clone(),
                );
                Ok(())
            }
            other => Err(SyntaxBackendBoundaryError::new(
                "projection.token_layout_mapping.terminal",
                format!("unsupported terminal payload kind {other}"),
            )),
        }
    }

    fn push_structural_event(
        &mut self,
        event_kind: &str,
        owner_rule: Option<&str>,
        owner_field: Option<&str>,
        node_kind: Option<&str>,
        decision_id: Option<String>,
        context: &ProjectionTokenLayoutContext,
    ) {
        self.events.push(ProjectionTokenLayoutMappedEvent {
            ordinal: self.events.len(),
            event_kind: event_kind.to_string(),
            token_range_id: context.structural_range.id.clone(),
            token_range_role: context.structural_range.role.clone(),
            owner_rule: owner_rule.map(str::to_string),
            owner_field: owner_field.map(str::to_string),
            node_kind: node_kind.map(str::to_string),
            decision_id,
            payload_kind: None,
            payload_byte_len: None,
            integer_value: None,
            payload_fits_token_range: None,
        });
        self.counts.structural_event_count += 1;
    }

    #[allow(clippy::too_many_arguments)]
    fn push_payload_event(
        &mut self,
        owner_rule: &str,
        owner_field: &str,
        node_kind: &str,
        terminal: &str,
        payload_kind: &str,
        payload_byte_len: Option<usize>,
        integer_value: Option<i64>,
        payload_fits_token_range: bool,
        range: ProjectionTokenLayoutRangeSummary,
    ) {
        self.events.push(ProjectionTokenLayoutMappedEvent {
            ordinal: self.events.len(),
            event_kind: "terminal".to_string(),
            token_range_id: range.id,
            token_range_role: range.role,
            owner_rule: Some(owner_rule.to_string()),
            owner_field: Some(owner_field.to_string()),
            node_kind: Some(node_kind.to_string()),
            decision_id: Some(format!(
                "decision.rule.{owner_rule}.field.{owner_field}.alternative.{node_kind}"
            )),
            payload_kind: Some(format!("{terminal}:{payload_kind}")),
            payload_byte_len,
            integer_value,
            payload_fits_token_range: Some(payload_fits_token_range),
        });
        self.counts.payload_event_count += 1;
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ProjectionRawTokenBuilder {
    raw_token_ids: Vec<u32>,
    segments: Vec<ProjectionRawTokenSegment>,
    counts: ProjectionRawTokenCounts,
    node_start_token: u32,
    node_end_token: u32,
    field_start_token: u32,
    field_end_token: u32,
    field_missing_token: u32,
    repeated_empty_token: u32,
    terminal_token: u32,
    payload_end_token: u32,
    boolean_false_token: u32,
    boolean_true_token: u32,
}

impl ProjectionRawTokenBuilder {
    fn new(context: &ProjectionTokenLayoutContext) -> Result<Self, SyntaxBackendBoundaryError> {
        if context.integer_encoding != ProjectionIntegerPayloadEncoding::Utf8Lexeme {
            return Err(SyntaxBackendBoundaryError::new(
                "projection.raw_token_ids.integer_encoding",
                "source-instance raw token IDs require integer lexeme payload encoding",
            ));
        }
        Ok(Self {
            raw_token_ids: Vec::new(),
            segments: Vec::new(),
            counts: ProjectionRawTokenCounts::default(),
            node_start_token: contextual_structural_token(
                context,
                RAW_TOKEN_NODE_START_OFFSET,
                "node_start",
            )?,
            node_end_token: contextual_structural_token(
                context,
                RAW_TOKEN_NODE_END_OFFSET,
                "node_end",
            )?,
            field_start_token: contextual_structural_token(
                context,
                RAW_TOKEN_FIELD_START_OFFSET,
                "field_start",
            )?,
            field_end_token: contextual_structural_token(
                context,
                RAW_TOKEN_FIELD_END_OFFSET,
                "field_end",
            )?,
            field_missing_token: contextual_structural_token(
                context,
                RAW_TOKEN_FIELD_MISSING_OFFSET,
                "field_missing",
            )?,
            repeated_empty_token: contextual_structural_token(
                context,
                RAW_TOKEN_REPEATED_EMPTY_OFFSET,
                "field_repeated_empty",
            )?,
            terminal_token: contextual_structural_token(
                context,
                RAW_TOKEN_TERMINAL_OFFSET,
                "terminal",
            )?,
            payload_end_token: contextual_structural_token(
                context,
                RAW_TOKEN_PAYLOAD_END_OFFSET,
                "payload_end",
            )?,
            boolean_false_token: contextual_structural_token(
                context,
                RAW_TOKEN_BOOLEAN_FALSE_OFFSET,
                "boolean_false",
            )?,
            boolean_true_token: contextual_structural_token(
                context,
                RAW_TOKEN_BOOLEAN_TRUE_OFFSET,
                "boolean_true",
            )?,
        })
    }

    fn append_node(
        &mut self,
        node: &SourceInstanceProjectionNode,
        context: &ProjectionTokenLayoutContext,
        parent_field: Option<(&str, &str)>,
    ) -> Result<(), SyntaxBackendBoundaryError> {
        context.require_record(&node.rule_name, &node.node_kind)?;
        let node_start_token = if let Some((owner_rule, owner_field)) = parent_field {
            context.node_start_token_for_child(
                owner_rule,
                owner_field,
                &node.node_kind,
                self.node_start_token,
            )?
        } else {
            self.node_start_token
        };
        self.push_structural_segment(
            "node_start",
            node_start_token,
            Some(&node.rule_name),
            None,
            Some(&node.node_kind),
            context,
        );
        for field in &node.fields {
            self.append_field(&node.rule_name, field, context)?;
        }
        self.push_structural_segment(
            "node_end",
            self.node_end_token,
            Some(&node.rule_name),
            None,
            Some(&node.node_kind),
            context,
        );
        Ok(())
    }

    fn append_field(
        &mut self,
        owner_rule: &str,
        field: &SourceInstanceProjectionField,
        context: &ProjectionTokenLayoutContext,
    ) -> Result<(), SyntaxBackendBoundaryError> {
        field.ensure_presence_contract()?;
        context.require_field(owner_rule, field)?;
        self.push_structural_segment(
            "field_start",
            self.field_start_token,
            Some(owner_rule),
            Some(&field.name),
            None,
            context,
        );

        if field.missing {
            context.require_optional(owner_rule, &field.name)?;
            self.push_structural_segment(
                "field_missing",
                self.field_missing_token,
                Some(owner_rule),
                Some(&field.name),
                None,
                context,
            );
        } else if field.multiple && field.values.is_empty() {
            context.require_list(owner_rule, &field.name)?;
            self.push_structural_segment(
                "field_repeated_empty",
                self.repeated_empty_token,
                Some(owner_rule),
                Some(&field.name),
                None,
                context,
            );
        } else {
            for value in &field.values {
                self.append_value(owner_rule, field, value, context)?;
            }
        }

        self.push_structural_segment(
            "field_end",
            self.field_end_token,
            Some(owner_rule),
            Some(&field.name),
            None,
            context,
        );
        Ok(())
    }

    fn append_value(
        &mut self,
        owner_rule: &str,
        field: &SourceInstanceProjectionField,
        value: &SourceInstanceProjectionValue,
        context: &ProjectionTokenLayoutContext,
    ) -> Result<(), SyntaxBackendBoundaryError> {
        match value.kind.as_str() {
            "node" => {
                context.require_reference(owner_rule, &field.name, &value.node_kind)?;
                let node = value.node.as_ref().ok_or_else(|| {
                    SyntaxBackendBoundaryError::new(
                        "projection.raw_token_ids.node",
                        "node projection values must carry a child node",
                    )
                })?;
                self.append_node(node, context, Some((owner_rule, &field.name)))
            }
            "terminal" => self.append_terminal(owner_rule, field, value, context),
            other => Err(SyntaxBackendBoundaryError::new(
                "projection.raw_token_ids.value_kind",
                format!("unexpected projection value kind {other}"),
            )),
        }
    }

    fn append_terminal(
        &mut self,
        owner_rule: &str,
        field: &SourceInstanceProjectionField,
        value: &SourceInstanceProjectionValue,
        context: &ProjectionTokenLayoutContext,
    ) -> Result<(), SyntaxBackendBoundaryError> {
        let terminal = value.terminal.as_deref().ok_or_else(|| {
            SyntaxBackendBoundaryError::new(
                "projection.raw_token_ids.terminal",
                "terminal projection values must name a terminal payload kind",
            )
        })?;
        if field.terminal.as_deref() != Some(terminal) {
            return Err(SyntaxBackendBoundaryError::new(
                "projection.raw_token_ids.terminal",
                format!(
                    "field {} terminal {:?} does not match value terminal {terminal}",
                    field.name, field.terminal
                ),
            ));
        }
        let payload = value.payload.as_deref().ok_or_else(|| {
            SyntaxBackendBoundaryError::new(
                "projection.raw_token_ids.payload",
                "terminal projection values must carry a payload",
            )
        })?;
        let payload_kind = terminal_payload_kind(terminal)?;
        context.require_payload(owner_rule, &field.name, &value.node_kind, terminal)?;
        let segment_owner = RawTokenSegmentOwner {
            owner_rule: Some(owner_rule),
            owner_field: Some(&field.name),
            node_kind: Some(&value.node_kind),
        };

        match terminal {
            "identifier_payload" | "string_payload" => self.push_byte_payload_segment(
                segment_owner,
                format!("{terminal}:{payload_kind}"),
                payload,
                &context.utf8_range,
                RawPayloadCounter::Utf8,
            ),
            "integer_payload" => self.push_byte_payload_segment(
                segment_owner,
                format!("{terminal}:{payload_kind}"),
                payload,
                &context.integer_range,
                RawPayloadCounter::Integer,
            ),
            "boolean_payload" => {
                let bool_token = match payload {
                    "false" => self.boolean_false_token,
                    "true" => self.boolean_true_token,
                    other => {
                        return Err(SyntaxBackendBoundaryError::new(
                            "projection.raw_token_ids.boolean_payload",
                            format!("boolean payload {other:?} is not true or false"),
                        ));
                    }
                };
                let start = self.raw_token_ids.len();
                self.raw_token_ids.push(self.terminal_token);
                self.raw_token_ids.push(bool_token);
                self.counts.terminal_decision_token_count += 1;
                self.counts.boolean_payload_token_count += 1;
                self.push_segment_record(
                    "terminal",
                    context.structural_range.role.clone(),
                    start,
                    segment_owner,
                    Some(format!("{terminal}:{payload_kind}")),
                );
                Ok(())
            }
            other => Err(SyntaxBackendBoundaryError::new(
                "projection.raw_token_ids.terminal",
                format!("unsupported terminal payload kind {other}"),
            )),
        }
    }

    fn push_structural_segment(
        &mut self,
        event_kind: &str,
        token: u32,
        owner_rule: Option<&str>,
        owner_field: Option<&str>,
        node_kind: Option<&str>,
        context: &ProjectionTokenLayoutContext,
    ) {
        let start = self.raw_token_ids.len();
        self.raw_token_ids.push(token);
        self.counts.structural_event_token_count += 1;
        self.push_segment_record(
            event_kind,
            context.structural_range.role.clone(),
            start,
            RawTokenSegmentOwner {
                owner_rule,
                owner_field,
                node_kind,
            },
            None,
        );
    }

    fn push_byte_payload_segment(
        &mut self,
        owner: RawTokenSegmentOwner<'_>,
        payload_kind: String,
        payload: &str,
        range: &ProjectionTokenLayoutRangeSummary,
        counter: RawPayloadCounter,
    ) -> Result<(), SyntaxBackendBoundaryError> {
        let start = self.raw_token_ids.len();
        self.raw_token_ids.push(self.terminal_token);
        self.counts.terminal_decision_token_count += 1;
        for byte in payload.as_bytes() {
            let token = range.start + u32::from(*byte);
            if token >= range.end_exclusive {
                return Err(SyntaxBackendBoundaryError::new(
                    "projection.raw_token_ids.payload",
                    format!("payload byte {byte} does not fit token range {}", range.id),
                ));
            }
            self.raw_token_ids.push(token);
        }
        match counter {
            RawPayloadCounter::Utf8 => {
                self.counts.utf8_payload_byte_token_count += payload.len();
            }
            RawPayloadCounter::Integer => {
                self.counts.integer_payload_byte_token_count += payload.len();
            }
        }
        self.raw_token_ids.push(self.payload_end_token);
        self.counts.payload_end_token_count += 1;
        self.push_segment_record(
            "terminal",
            range.role.clone(),
            start,
            owner,
            Some(payload_kind),
        );
        Ok(())
    }

    fn push_segment_record(
        &mut self,
        event_kind: &str,
        primary_token_range_role: String,
        raw_token_offset: usize,
        owner: RawTokenSegmentOwner<'_>,
        payload_kind: Option<String>,
    ) {
        self.segments.push(ProjectionRawTokenSegment {
            event_ordinal: self.segments.len(),
            event_kind: event_kind.to_string(),
            primary_token_range_role,
            raw_token_offset,
            raw_token_count: self.raw_token_ids.len() - raw_token_offset,
            owner_rule: owner.owner_rule.map(str::to_string),
            owner_field: owner.owner_field.map(str::to_string),
            node_kind: owner.node_kind.map(str::to_string),
            payload_kind,
        });
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RawPayloadCounter {
    Utf8,
    Integer,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct RawTokenSegmentOwner<'a> {
    owner_rule: Option<&'a str>,
    owner_field: Option<&'a str>,
    node_kind: Option<&'a str>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ProjectionTokenLayoutContext {
    structural_range: ProjectionTokenLayoutRangeSummary,
    utf8_range: ProjectionTokenLayoutRangeSummary,
    integer_range: ProjectionTokenLayoutRangeSummary,
    integer_encoding: ProjectionIntegerPayloadEncoding,
    records: BTreeMap<String, String>,
    fields: BTreeMap<(String, String), ProjectionTokenLayoutFieldShape>,
    references: BTreeSet<(String, String, String)>,
    payloads: BTreeSet<(String, String, String, String)>,
    alternatives: BTreeSet<(String, String, String, String)>,
    multi_alternative_nonterminal_fields: BTreeSet<(String, String)>,
    node_start_alternative_tokens: BTreeMap<String, u32>,
    optionals: BTreeSet<(String, String)>,
    lists: BTreeSet<(String, String)>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ProjectionIntegerPayloadEncoding {
    Utf8Lexeme,
    U8Value { valid_min: i64, valid_max: i64 },
}

impl ProjectionTokenLayoutContext {
    fn from_sidecar(
        sidecar: &GrammarStructuralModelSidecar,
    ) -> Result<Self, SyntaxBackendBoundaryError> {
        let layout = &sidecar.structural_model.token_ranges;
        let structural_range = token_range_by_role(layout, "structural_decisions")?;
        let utf8_range = token_range_by_role(layout, "utf8_byte_payload")?;
        let integer_range = token_range_by_role(layout, "integer_lexeme_payload")
            .or_else(|_| token_range_by_role(layout, "u8_integer_payload"))?;
        let integer_encoding = if integer_range.role == "integer_lexeme_payload" {
            ProjectionIntegerPayloadEncoding::Utf8Lexeme
        } else {
            let integer_value_range = layout.integer_value_range.as_ref().ok_or_else(|| {
                SyntaxBackendBoundaryError::new(
                    "projection.token_layout_mapping.integer_value_range",
                    "u8 integer token layout must expose an integer value range",
                )
            })?;
            ProjectionIntegerPayloadEncoding::U8Value {
                valid_min: integer_value_range.valid_min,
                valid_max: integer_value_range.valid_max,
            }
        };

        let records = sidecar
            .structural_model
            .records
            .iter()
            .map(|record| (record.grammar_rule.clone(), record.record_type.clone()))
            .collect();
        let fields = sidecar
            .structural_model
            .records
            .iter()
            .flat_map(|record| {
                record.fields.iter().map(|field| {
                    (
                        (record.grammar_rule.clone(), field.name.clone()),
                        ProjectionTokenLayoutFieldShape {
                            required: field.required,
                            multiple: field.multiple,
                        },
                    )
                })
            })
            .collect();
        let references = sidecar
            .structural_model
            .references
            .iter()
            .map(|reference| {
                (
                    reference.owner_rule.clone(),
                    reference.field.clone(),
                    reference.target_node_kind.clone(),
                )
            })
            .collect();
        let payloads = sidecar
            .structural_model
            .payload_leaves
            .iter()
            .map(|payload| {
                (
                    payload.owner_rule.clone(),
                    payload.field.clone(),
                    payload.node_kind.clone(),
                    payload.payload_kind.clone(),
                )
            })
            .collect();
        let alternatives = sidecar
            .structural_model
            .finite_domains
            .iter()
            .flat_map(|domain| {
                domain.alternatives.iter().map(|alternative| {
                    (
                        domain.owner_rule.clone(),
                        domain.owner_field.clone(),
                        alternative.node_kind.clone(),
                        terminal_kind_name(alternative.terminal).to_string(),
                    )
                })
            })
            .collect();
        let multi_alternative_nonterminal_fields = sidecar
            .structural_model
            .finite_domains
            .iter()
            .filter(|domain| domain.alternatives.len() > 1)
            .filter(|domain| {
                domain
                    .alternatives
                    .iter()
                    .any(|alternative| alternative.terminal == GrammarTerminalKind::Nonterminal)
            })
            .map(|domain| (domain.owner_rule.clone(), domain.owner_field.clone()))
            .collect();
        let node_start_alternative_tokens = node_start_alternative_token_ids_from_sidecar(sidecar)?;
        let optionals = sidecar
            .structural_model
            .optionals
            .iter()
            .map(|optional| (optional.owner_rule.clone(), optional.field.clone()))
            .collect();
        let lists = sidecar
            .structural_model
            .lists
            .iter()
            .map(|list| (list.owner_rule.clone(), list.field.clone()))
            .collect();

        Ok(Self {
            structural_range,
            utf8_range,
            integer_range,
            integer_encoding,
            records,
            fields,
            references,
            payloads,
            alternatives,
            multi_alternative_nonterminal_fields,
            node_start_alternative_tokens,
            optionals,
            lists,
        })
    }

    fn require_record(
        &self,
        rule_name: &str,
        node_kind: &str,
    ) -> Result<(), SyntaxBackendBoundaryError> {
        let Some(expected_node_kind) = self.records.get(rule_name) else {
            return Err(SyntaxBackendBoundaryError::new(
                "projection.token_layout_mapping.record",
                format!("projected rule {rule_name} is missing from StructuralModel records"),
            ));
        };
        if expected_node_kind != node_kind {
            return Err(SyntaxBackendBoundaryError::new(
                "projection.token_layout_mapping.record",
                format!(
                    "projected rule {rule_name} has node kind {node_kind}, expected {expected_node_kind}"
                ),
            ));
        }
        Ok(())
    }

    fn require_field(
        &self,
        owner_rule: &str,
        field: &SourceInstanceProjectionField,
    ) -> Result<(), SyntaxBackendBoundaryError> {
        let Some(shape) = self
            .fields
            .get(&(owner_rule.to_string(), field.name.clone()))
        else {
            return Err(SyntaxBackendBoundaryError::new(
                "projection.token_layout_mapping.field",
                format!(
                    "field {owner_rule}.{} is missing from StructuralModel",
                    field.name
                ),
            ));
        };
        if shape.required != field.required || shape.multiple != field.multiple {
            return Err(SyntaxBackendBoundaryError::new(
                "projection.token_layout_mapping.field",
                format!(
                    "field {owner_rule}.{} shape drifted: projected required={} multiple={}, StructuralModel required={} multiple={}",
                    field.name, field.required, field.multiple, shape.required, shape.multiple
                ),
            ));
        }
        Ok(())
    }

    fn require_reference(
        &self,
        owner_rule: &str,
        field_name: &str,
        target_node_kind: &str,
    ) -> Result<(), SyntaxBackendBoundaryError> {
        let key = (
            owner_rule.to_string(),
            field_name.to_string(),
            target_node_kind.to_string(),
        );
        if self.references.contains(&key)
            && self.alternatives.contains(&(
                owner_rule.to_string(),
                field_name.to_string(),
                target_node_kind.to_string(),
                "nonterminal".to_string(),
            ))
        {
            Ok(())
        } else {
            Err(SyntaxBackendBoundaryError::new(
                "projection.token_layout_mapping.reference",
                format!(
                    "field {owner_rule}.{field_name} cannot map nonterminal {target_node_kind}"
                ),
            ))
        }
    }

    fn node_start_token_for_child(
        &self,
        owner_rule: &str,
        field_name: &str,
        target_node_kind: &str,
        default_token: u32,
    ) -> Result<u32, SyntaxBackendBoundaryError> {
        if !self
            .multi_alternative_nonterminal_fields
            .contains(&(owner_rule.to_string(), field_name.to_string()))
        {
            return Ok(default_token);
        }
        self.node_start_alternative_tokens
            .get(target_node_kind)
            .copied()
            .ok_or_else(|| {
                SyntaxBackendBoundaryError::new(
                    "projection.raw_token_ids.alternative_node_start_token",
                    format!(
                        "field {owner_rule}.{field_name} selected node kind {target_node_kind} without a tokenized alternative identity"
                    ),
                )
            })
    }

    fn require_payload(
        &self,
        owner_rule: &str,
        field_name: &str,
        node_kind: &str,
        terminal: &str,
    ) -> Result<(), SyntaxBackendBoundaryError> {
        let payload_kind = terminal_payload_kind(terminal)?;
        if !self.alternatives.contains(&(
            owner_rule.to_string(),
            field_name.to_string(),
            node_kind.to_string(),
            terminal.to_string(),
        )) {
            return Err(SyntaxBackendBoundaryError::new(
                "projection.token_layout_mapping.payload",
                format!("field {owner_rule}.{field_name} cannot map terminal {node_kind}"),
            ));
        }
        if !self.payloads.contains(&(
            owner_rule.to_string(),
            field_name.to_string(),
            node_kind.to_string(),
            payload_kind.to_string(),
        )) {
            return Err(SyntaxBackendBoundaryError::new(
                "projection.token_layout_mapping.payload",
                format!(
                    "field {owner_rule}.{field_name} terminal {node_kind} is missing payload leaf {payload_kind}"
                ),
            ));
        }
        Ok(())
    }

    fn require_optional(
        &self,
        owner_rule: &str,
        field_name: &str,
    ) -> Result<(), SyntaxBackendBoundaryError> {
        if self
            .optionals
            .contains(&(owner_rule.to_string(), field_name.to_string()))
        {
            Ok(())
        } else {
            Err(SyntaxBackendBoundaryError::new(
                "projection.token_layout_mapping.optional",
                format!("field {owner_rule}.{field_name} is not optional"),
            ))
        }
    }

    fn require_list(
        &self,
        owner_rule: &str,
        field_name: &str,
    ) -> Result<(), SyntaxBackendBoundaryError> {
        if self
            .lists
            .contains(&(owner_rule.to_string(), field_name.to_string()))
        {
            Ok(())
        } else {
            Err(SyntaxBackendBoundaryError::new(
                "projection.token_layout_mapping.list",
                format!("field {owner_rule}.{field_name} is not a list"),
            ))
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ProjectionTokenLayoutFieldShape {
    required: bool,
    multiple: bool,
}

#[derive(Debug, Default, Clone, PartialEq, Eq)]
struct ProjectionCodecInputBuilder {
    events: Vec<ProjectionCodecInputEvent>,
    counts: ProjectionCodecInputCounts,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SourceInstanceProjectionNode {
    pub node_kind: String,
    pub rule_name: String,
    pub fields: Vec<SourceInstanceProjectionField>,
}

impl SourceInstanceProjectionNode {
    fn observed_stats(&self) -> Result<SourceInstanceProjectionStats, SyntaxBackendBoundaryError> {
        let mut observed = ObservedProjectionTreeStats::default();
        self.collect_stats(1, &mut observed)?;
        Ok(observed.into_projection_stats())
    }

    fn collect_stats(
        &self,
        depth: usize,
        observed: &mut ObservedProjectionTreeStats,
    ) -> Result<(), SyntaxBackendBoundaryError> {
        require_non_empty("projection.node.node_kind", &self.node_kind)?;
        require_non_empty("projection.node.rule_name", &self.rule_name)?;
        observed.node_count += 1;
        observed.max_depth = observed.max_depth.max(depth);
        observed.node_kinds.insert(self.node_kind.clone());
        for field in &self.fields {
            field.collect_stats(depth + 1, observed)?;
        }
        Ok(())
    }

    fn append_codec_input_events(
        &self,
        builder: &mut ProjectionCodecInputBuilder,
    ) -> Result<(), SyntaxBackendBoundaryError> {
        builder.events.push(ProjectionCodecInputEvent::node_start(
            &self.node_kind,
            &self.rule_name,
        ));
        builder.counts.node_start_count += 1;
        for field in &self.fields {
            field.append_codec_input_events(builder)?;
        }
        builder.events.push(ProjectionCodecInputEvent::node_end(
            &self.node_kind,
            &self.rule_name,
        ));
        builder.counts.node_end_count += 1;
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SourceInstanceProjectionField {
    pub name: String,
    pub required: bool,
    pub multiple: bool,
    #[serde(default)]
    pub terminal: Option<String>,
    #[serde(default)]
    pub values: Vec<SourceInstanceProjectionValue>,
    #[serde(default)]
    pub missing: bool,
}

impl SourceInstanceProjectionField {
    /// Reject projection field shapes that would emit a non-canonical stream.
    ///
    /// Optional singular fields must either set `missing=true` or carry a value.
    /// `missing=false` with an empty `values` list would emit bare
    /// `field_start`/`field_end`, which strict decode rejects.
    fn ensure_presence_contract(&self) -> Result<(), SyntaxBackendBoundaryError> {
        require_non_empty("projection.field.name", &self.name)?;
        if self.missing {
            if self.required || !self.values.is_empty() {
                return Err(SyntaxBackendBoundaryError::new(
                    "projection.field.missing",
                    "missing fields must be optional and cannot carry values",
                ));
            }
            return Ok(());
        }
        if self.multiple && self.values.is_empty() {
            if self.required {
                return Err(SyntaxBackendBoundaryError::new(
                    "projection.field.values",
                    "required repeated fields cannot be empty",
                ));
            }
            return Ok(());
        }
        if !self.multiple && self.values.len() > 1 {
            return Err(SyntaxBackendBoundaryError::new(
                "projection.field.values",
                "single-value fields cannot carry multiple values",
            ));
        }
        if !self.multiple && self.values.is_empty() {
            if self.required {
                return Err(SyntaxBackendBoundaryError::new(
                    "projection.field.values",
                    "required fields must carry a projected value",
                ));
            }
            return Err(SyntaxBackendBoundaryError::new(
                "projection.field.values",
                format!(
                    "optional field `{}` has missing=false with no values; set missing=true \
                     or provide a value (empty present fields emit field_start/field_end, \
                     which strict decode rejects)",
                    self.name
                ),
            ));
        }
        Ok(())
    }

    fn collect_stats(
        &self,
        value_depth: usize,
        observed: &mut ObservedProjectionTreeStats,
    ) -> Result<(), SyntaxBackendBoundaryError> {
        self.ensure_presence_contract()?;
        observed.field_count += 1;
        if self.missing {
            observed.optional_missing_count += 1;
            return Ok(());
        }
        if self.multiple && self.values.is_empty() {
            observed.repeated_empty_count += 1;
            return Ok(());
        }
        for value in &self.values {
            value.collect_stats(value_depth, observed)?;
        }
        Ok(())
    }

    fn append_codec_input_events(
        &self,
        builder: &mut ProjectionCodecInputBuilder,
    ) -> Result<(), SyntaxBackendBoundaryError> {
        self.ensure_presence_contract()?;
        builder
            .events
            .push(ProjectionCodecInputEvent::field_start(self));
        builder.counts.field_start_count += 1;
        if self.missing {
            builder
                .events
                .push(ProjectionCodecInputEvent::field_missing(&self.name));
            builder.counts.missing_field_count += 1;
        } else if self.multiple && self.values.is_empty() {
            builder
                .events
                .push(ProjectionCodecInputEvent::field_repeated_empty(&self.name));
            builder.counts.repeated_empty_field_count += 1;
        } else {
            for value in &self.values {
                value.append_codec_input_events(builder)?;
            }
        }
        builder
            .events
            .push(ProjectionCodecInputEvent::field_end(&self.name));
        builder.counts.field_end_count += 1;
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SourceInstanceProjectionValue {
    pub kind: String,
    pub node_kind: String,
    #[serde(default)]
    pub terminal: Option<String>,
    #[serde(default)]
    pub payload: Option<String>,
    #[serde(default)]
    pub token_name: Option<String>,
    #[serde(default)]
    pub node: Option<Box<SourceInstanceProjectionNode>>,
}

impl SourceInstanceProjectionValue {
    fn collect_stats(
        &self,
        value_depth: usize,
        observed: &mut ObservedProjectionTreeStats,
    ) -> Result<(), SyntaxBackendBoundaryError> {
        require_non_empty("projection.value.kind", &self.kind)?;
        require_non_empty("projection.value.node_kind", &self.node_kind)?;
        observed.field_value_count += 1;
        match self.kind.as_str() {
            "node" => {
                let node = self.node.as_ref().ok_or_else(|| {
                    SyntaxBackendBoundaryError::new(
                        "projection.value.node",
                        "node values must carry a projected child node",
                    )
                })?;
                node.collect_stats(value_depth, observed)
            }
            "terminal" => {
                let terminal = self.terminal.as_deref().ok_or_else(|| {
                    SyntaxBackendBoundaryError::new(
                        "projection.value.terminal",
                        "terminal values must name a terminal payload kind",
                    )
                })?;
                if self.payload.is_none() {
                    return Err(SyntaxBackendBoundaryError::new(
                        "projection.value.payload",
                        "terminal values must carry a payload",
                    ));
                }
                if self.node.is_some() {
                    return Err(SyntaxBackendBoundaryError::new(
                        "projection.value.node",
                        "terminal values cannot carry a child node",
                    ));
                }
                observed.terminal_payload_count += 1;
                *observed
                    .terminal_payloads_by_kind
                    .entry(terminal.to_string())
                    .or_default() += 1;
                Ok(())
            }
            other => Err(SyntaxBackendBoundaryError::new(
                "projection.value.kind",
                format!("unexpected projected value kind {other}"),
            )),
        }
    }

    fn append_codec_input_events(
        &self,
        builder: &mut ProjectionCodecInputBuilder,
    ) -> Result<(), SyntaxBackendBoundaryError> {
        match self.kind.as_str() {
            "node" => {
                let node = self.node.as_ref().ok_or_else(|| {
                    SyntaxBackendBoundaryError::new(
                        "projection.codec_input.node",
                        "node projection values must carry a child node",
                    )
                })?;
                node.append_codec_input_events(builder)
            }
            "terminal" => {
                builder
                    .events
                    .push(ProjectionCodecInputEvent::terminal(self)?);
                builder.counts.terminal_count += 1;
                Ok(())
            }
            other => Err(SyntaxBackendBoundaryError::new(
                "projection.codec_input.value_kind",
                format!("unexpected projection value kind {other}"),
            )),
        }
    }
}

#[derive(Debug, Default, Clone, PartialEq, Eq)]
struct ObservedProjectionTreeStats {
    node_count: usize,
    field_count: usize,
    field_value_count: usize,
    terminal_payload_count: usize,
    optional_missing_count: usize,
    repeated_empty_count: usize,
    max_depth: usize,
    node_kinds: BTreeSet<String>,
    terminal_payloads_by_kind: BTreeMap<String, usize>,
}

impl ObservedProjectionTreeStats {
    fn into_projection_stats(self) -> SourceInstanceProjectionStats {
        SourceInstanceProjectionStats {
            node_count: self.node_count,
            field_count: self.field_count,
            field_value_count: self.field_value_count,
            terminal_payload_count: self.terminal_payload_count,
            optional_missing_count: self.optional_missing_count,
            repeated_empty_count: self.repeated_empty_count,
            max_depth: self.max_depth,
            distinct_node_kinds: self.node_kinds.into_iter().collect(),
            terminal_payloads_by_kind: self.terminal_payloads_by_kind,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SourceInstanceProjectionSource {
    pub path: String,
    pub sha256: String,
    pub bytes: usize,
}

impl SourceInstanceProjectionSource {
    fn validate_checked_file(
        &self,
        candidate_grammar_id: &str,
    ) -> Result<(), SyntaxBackendBoundaryError> {
        require_non_empty("source.path", &self.path)?;
        if candidate_grammar_id == GO_UNIFIED_GRAMMAR_ID {
            // Unified inventory grammar accepts any readable *.go path. Do not
            // force GOROOT's `/src/` layout: external stress modules, module
            // checkouts, and already gofmt-canonical sources are first-class
            // inputs. Gofmt working copies may still inject `/src/` for
            // historical path shapes, but that is not required for validation.
            require_go_source_path_suffix("source.path", &self.path)?;
        } else {
            require_contains(
                "source.path",
                &self.path,
                "tests/fixtures/go_syntax_corpus/src/",
            )?;
        }
        require_sha256("source.sha256", &self.sha256)?;
        let bytes = fs::read(&self.path).map_err(|error| {
            SyntaxBackendBoundaryError::new(
                "source.path",
                format!("failed to read checked Go source {}: {error}", self.path),
            )
        })?;
        require_string("source.sha256", &self.sha256, &sha256_hex(&bytes))?;
        require_usize("source.bytes", self.bytes, bytes.len())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SourceInstanceProjectionMetadata {
    pub path: String,
    pub sha256: String,
    pub artifact_kind: String,
    pub version: u32,
    pub rule_count: usize,
    pub field_count: usize,
}

impl SourceInstanceProjectionMetadata {
    fn validate_checked_file(
        &self,
        candidate_grammar_id: &str,
        source: &SourceInstanceProjectionSource,
        parser: &SourceInstanceProjectionParser,
    ) -> Result<(), SyntaxBackendBoundaryError> {
        require_non_empty("metadata.path", &self.path)?;
        if candidate_grammar_id == GO_UNIFIED_GRAMMAR_ID {
            return self.validate_checked_inventory_file();
        }
        require_contains(
            "metadata.path",
            &self.path,
            "tests/fixtures/go_syntax_metadata/",
        )?;
        require_sha256("metadata.sha256", &self.sha256)?;
        let bytes = fs::read(&self.path).map_err(|error| {
            SyntaxBackendBoundaryError::new(
                "metadata.path",
                format!("failed to read checked Go metadata {}: {error}", self.path),
            )
        })?;
        require_string("metadata.sha256", &self.sha256, &sha256_hex(&bytes))?;
        require_string(
            "metadata.artifact_kind",
            &self.artifact_kind,
            GO_SYNTAX_METADATA_ARTIFACT_KIND,
        )?;
        require_u32("metadata.version", self.version, GO_SYNTAX_METADATA_VERSION)?;
        let grammar = GoSyntaxMetadataAdapter::new(candidate_grammar_id)
            .lower_metadata_file(Path::new(&self.path))
            .map_err(|error| {
                SyntaxBackendBoundaryError::new(
                    "metadata.path",
                    format!("failed to lower Go syntax metadata: {error}"),
                )
            })?;
        require_string(
            "metadata.source_path",
            &grammar.provenance.source_path.unwrap_or_default(),
            &source.path,
        )?;
        require_string(
            "metadata.source_sha256",
            &grammar.provenance.source_sha256.unwrap_or_default(),
            &source.sha256,
        )?;
        if let Some(metadata_parser) = grammar.provenance.parser {
            require_string(
                "metadata.parser.tool",
                &metadata_parser.tool,
                &parser.metadata_tool,
            )?;
            require_string(
                "metadata.parser.version",
                &metadata_parser.version,
                &parser.metadata_version,
            )?;
            require_string(
                "metadata.parser.package",
                metadata_parser.package.as_deref().unwrap_or_default(),
                &parser.metadata_package,
            )?;
            require_string(
                "metadata.parser.package_version",
                metadata_parser
                    .package_version
                    .as_deref()
                    .unwrap_or_default(),
                &parser.metadata_package_version,
            )?;
        } else {
            return Err(SyntaxBackendBoundaryError::new(
                "metadata.parser",
                "checked Go metadata must carry parser provenance",
            ));
        }
        require_usize("metadata.rule_count", self.rule_count, grammar.rules.len())?;
        let field_count = grammar.rules.iter().map(|rule| rule.fields.len()).sum();
        require_usize("metadata.field_count", self.field_count, field_count)
    }

    /// Unified inventory-as-metadata branch: the metadata block records the
    /// checked go/ast schema inventory fixture (source-independent), so the
    /// per-file source/parser provenance cross-checks are replaced by
    /// unified-artifact equality checks.
    fn validate_checked_inventory_file(&self) -> Result<(), SyntaxBackendBoundaryError> {
        require_contains(
            "metadata.path",
            &self.path,
            "tests/fixtures/go_ast_schema_inventory/",
        )?;
        require_sha256("metadata.sha256", &self.sha256)?;
        let unified = go_unified_grammar_artifact()?;
        require_string(
            "metadata.path",
            &self.path,
            &unified.identity.inventory_fixture_path,
        )?;
        require_string(
            "metadata.sha256",
            &self.sha256,
            &unified.identity.inventory_fixture_sha256,
        )?;
        require_string(
            "metadata.artifact_kind",
            &self.artifact_kind,
            GO_AST_SCHEMA_INVENTORY_METADATA_ARTIFACT_KIND,
        )?;
        require_u32("metadata.version", self.version, 1)?;
        require_usize(
            "metadata.rule_count",
            self.rule_count,
            unified.grammar.rules.len(),
        )?;
        let field_count = unified
            .grammar
            .rules
            .iter()
            .map(|rule| rule.fields.len())
            .sum();
        require_usize("metadata.field_count", self.field_count, field_count)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SourceInstanceProjectionParser {
    pub metadata_tool: String,
    pub metadata_version: String,
    pub metadata_package: String,
    pub metadata_package_version: String,
    pub runtime_go_version: String,
}

impl SourceInstanceProjectionParser {
    fn validate(&self) -> Result<(), SyntaxBackendBoundaryError> {
        require_string("parser.metadata_tool", &self.metadata_tool, PARSER_TOOL)?;
        require_string(
            "parser.metadata_version",
            &self.metadata_version,
            PARSER_VERSION,
        )?;
        require_string(
            "parser.metadata_package",
            &self.metadata_package,
            PARSER_PACKAGE,
        )?;
        require_string(
            "parser.metadata_package_version",
            &self.metadata_package_version,
            PARSER_VERSION,
        )?;
        require_string(
            "parser.runtime_go_version",
            &self.runtime_go_version,
            PARSER_VERSION,
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SourceInstanceProjectionStats {
    pub node_count: usize,
    pub field_count: usize,
    pub field_value_count: usize,
    pub terminal_payload_count: usize,
    pub optional_missing_count: usize,
    pub repeated_empty_count: usize,
    pub max_depth: usize,
    pub distinct_node_kinds: Vec<String>,
    pub terminal_payloads_by_kind: BTreeMap<String, usize>,
}

impl SourceInstanceProjectionStats {
    fn validate(&self) -> Result<(), SyntaxBackendBoundaryError> {
        require_positive_usize("projection_stats.node_count", self.node_count)?;
        require_positive_usize("projection_stats.field_count", self.field_count)?;
        require_positive_usize("projection_stats.field_value_count", self.field_value_count)?;
        require_positive_usize(
            "projection_stats.terminal_payload_count",
            self.terminal_payload_count,
        )?;
        require_positive_usize("projection_stats.max_depth", self.max_depth)?;
        if self.field_value_count < self.terminal_payload_count {
            return Err(SyntaxBackendBoundaryError::new(
                "projection_stats.field_value_count",
                "field values must cover terminal payload values",
            ));
        }
        require_sorted_unique_strings(
            "projection_stats.distinct_node_kinds",
            &self.distinct_node_kinds,
        )?;
        if !self.distinct_node_kinds.iter().any(|kind| kind == "File") {
            return Err(SyntaxBackendBoundaryError::new(
                "projection_stats.distinct_node_kinds",
                "source-instance projections must include the Go File root node kind",
            ));
        }
        let payload_total: usize = self.terminal_payloads_by_kind.values().sum();
        require_usize(
            "projection_stats.terminal_payloads_by_kind.total",
            payload_total,
            self.terminal_payload_count,
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SourceInstanceCanonicalRender {
    pub formatter: String,
    pub gofmt: String,
    pub go_parser: String,
    pub matches_source: bool,
    pub canonical_sha256: String,
    pub canonical_byte_count: usize,
}

impl SourceInstanceCanonicalRender {
    fn validate(
        &self,
        source: &SourceInstanceProjectionSource,
    ) -> Result<(), SyntaxBackendBoundaryError> {
        require_string(
            "canonical_render.formatter",
            &self.formatter,
            "go/format.Source",
        )?;
        require_string("canonical_render.gofmt", &self.gofmt, "passed")?;
        require_string("canonical_render.go_parser", &self.go_parser, "passed")?;
        require_bool("canonical_render.matches_source", self.matches_source, true)?;
        require_sha256("canonical_render.canonical_sha256", &self.canonical_sha256)?;
        require_string(
            "canonical_render.canonical_sha256",
            &self.canonical_sha256,
            &source.sha256,
        )?;
        require_usize(
            "canonical_render.canonical_byte_count",
            self.canonical_byte_count,
            source.bytes,
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SourceInstanceCurrentEvidence {
    pub metadata_rules_projected: usize,
    pub metadata_fields_projected: usize,
    pub projection_node_kinds_covered: Vec<String>,
    pub source_instance_projection_ready: bool,
    pub generated_codec_report_ready: bool,
    pub reusable_renderer_consumer_ready: bool,
    pub report_ingestion_ready: bool,
    pub training_manifest_promotion_ready: bool,
}

impl SourceInstanceCurrentEvidence {
    fn validate(
        &self,
        stats: &SourceInstanceProjectionStats,
        metadata: &SourceInstanceProjectionMetadata,
    ) -> Result<(), SyntaxBackendBoundaryError> {
        require_usize(
            "current_evidence.metadata_rules_projected",
            self.metadata_rules_projected,
            metadata.rule_count,
        )?;
        require_usize(
            "current_evidence.metadata_fields_projected",
            self.metadata_fields_projected,
            metadata.field_count,
        )?;
        require_sorted_unique_strings(
            "current_evidence.projection_node_kinds_covered",
            &self.projection_node_kinds_covered,
        )?;
        require_bool(
            "current_evidence.source_instance_projection_ready",
            self.source_instance_projection_ready,
            true,
        )?;
        require_bool(
            "current_evidence.generated_codec_report_ready",
            self.generated_codec_report_ready,
            false,
        )?;
        require_bool(
            "current_evidence.reusable_renderer_consumer_ready",
            self.reusable_renderer_consumer_ready,
            false,
        )?;
        require_bool(
            "current_evidence.report_ingestion_ready",
            self.report_ingestion_ready,
            false,
        )?;
        require_bool(
            "current_evidence.training_manifest_promotion_ready",
            self.training_manifest_promotion_ready,
            false,
        )?;
        if self.projection_node_kinds_covered != stats.distinct_node_kinds {
            return Err(SyntaxBackendBoundaryError::new(
                "current_evidence.projection_node_kinds_covered",
                "current evidence must match the projected distinct node kind inventory",
            ));
        }
        Ok(())
    }
}

/// Only accepted `per_file_grammar_blocker.kind`: a checked pre-unification
/// fixture retained as historical replay/mask-evidence input. New kinds
/// require a deliberate extension of the forward-gate policy.
pub const PER_FILE_GRAMMAR_BLOCKER_KIND_HISTORICAL_FIXTURE: &str =
    "historical_pre_unification_checked_fixture";

/// Recorded-blocker escape hatch for retired per-file Go grammar ids
/// (forward gate, M1). Mirrors the `SourceInstanceCurrentBlocker` pattern:
/// a typed kind plus a human-readable reason naming why the unified pinned
/// grammar artifact cannot cover the record.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SourceInstancePerFileGrammarBlocker {
    pub kind: String,
    pub reason: String,
}

impl SourceInstancePerFileGrammarBlocker {
    fn validate(&self) -> Result<(), SyntaxBackendBoundaryError> {
        require_non_empty("per_file_grammar_blocker.reason", &self.reason)?;
        match self.kind.as_str() {
            PER_FILE_GRAMMAR_BLOCKER_KIND_HISTORICAL_FIXTURE => Ok(()),
            other => Err(SyntaxBackendBoundaryError::new(
                "per_file_grammar_blocker.kind",
                format!("unexpected per-file grammar blocker kind {other}"),
            )),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SourceInstanceCurrentBlocker {
    pub kind: String,
    pub path: String,
    pub reason: String,
}

impl SourceInstanceCurrentBlocker {
    fn validate(&self, candidate_grammar_id: &str) -> Result<(), SyntaxBackendBoundaryError> {
        require_non_empty("current_blockers.path", &self.path)?;
        require_non_empty("current_blockers.reason", &self.reason)?;
        match self.kind.as_str() {
            "generated_codec_report_blocked_by_missing_projection_consumer" => {
                require_contains(
                    "current_blockers.reason",
                    &self.reason,
                    "projected AST field values",
                )?;
                require_contains("current_blockers.reason", &self.reason, "reusable renderer")
            }
            "generated_codec_route_unregistered" => {
                require_contains(
                    "current_blockers.path",
                    &self.path,
                    "syntax_production_route",
                )?;
                require_contains(
                    "current_blockers.reason",
                    &self.reason,
                    candidate_grammar_id,
                )
            }
            "report_ingestion_manifest_record_missing" => {
                require_contains(
                    "current_blockers.path",
                    &self.path,
                    "training-report-ingestion",
                )?;
                require_contains(
                    "current_blockers.reason",
                    &self.reason,
                    "generated-codec report",
                )
            }
            other => Err(SyntaxBackendBoundaryError::new(
                "current_blockers.kind",
                format!("unexpected source-instance projection blocker {other}"),
            )),
        }
    }
}

pub fn parse_checked_report(
    contents: &str,
) -> Result<SyntaxGoSourceInstanceProjectionReport, SyntaxBackendBoundaryError> {
    let report: SyntaxGoSourceInstanceProjectionReport =
        serde_json::from_str(contents).map_err(|error| {
            SyntaxBackendBoundaryError::new(
                "source_instance_projection_report",
                format!("failed to parse projection report: {error}"),
            )
        })?;
    report.validate()?;
    Ok(report)
}

pub fn parse_projection_artifact(
    contents: &str,
) -> Result<SourceInstanceProjectionArtifact, SyntaxBackendBoundaryError> {
    // Real Go stdlib projections can nest past serde_json's default recursion
    // limit of 128 (observed on cmd/cgo/gcc.go and other large AST trees).
    // Disable only the artificial depth counter; deserialization is still
    // stack-recursive, so pathological multi-thousand-depth inputs can still
    // overflow the stack, but inventory-derived Go ASTs stay well below that.
    let mut deserializer = serde_json::Deserializer::from_str(contents);
    deserializer.disable_recursion_limit();
    SourceInstanceProjectionArtifact::deserialize(&mut deserializer).map_err(|error| {
        SyntaxBackendBoundaryError::new(
            "source_instance_projection_artifact",
            format!("failed to parse projection artifact: {error}"),
        )
    })
}

pub fn parse_generated_codec_report(
    contents: &str,
) -> Result<SourceInstanceGeneratedCodecReport, SyntaxBackendBoundaryError> {
    let report: SourceInstanceGeneratedCodecReport =
        serde_json::from_str(contents).map_err(|error| {
            SyntaxBackendBoundaryError::new(
                "source_instance_generated_codec_report",
                format!("failed to parse generated-codec report: {error}"),
            )
        })?;
    report.validate()?;
    Ok(report)
}

fn load_checked_projection_report(
    path: &str,
) -> Result<SyntaxGoSourceInstanceProjectionReport, SyntaxBackendBoundaryError> {
    require_non_empty(
        "projection.generated_codec_report.projection_report_path",
        path,
    )?;
    let contents = fs::read_to_string(path).map_err(|error| {
        SyntaxBackendBoundaryError::new(
            "projection.generated_codec_report.projection_report_path",
            format!("failed to read projection report {path}: {error}"),
        )
    })?;
    parse_checked_report(&contents)
}

fn generated_codec_report_path_from_projection_output(
    projection_output: &str,
) -> Result<String, SyntaxBackendBoundaryError> {
    let file_name = Path::new(projection_output)
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| {
            SyntaxBackendBoundaryError::new(
                "projection_output",
                "projection output must name a source-instance projection JSON file",
            )
        })?;
    let stem = file_name
        .strip_suffix("-source-instance-projection-v1.json")
        .ok_or_else(|| {
            SyntaxBackendBoundaryError::new(
                "projection_output",
                "projection output must end with -source-instance-projection-v1.json",
            )
        })?;
    Ok(format!(
        "{GENERATED_CODEC_REPORT_DIR}/go-{stem}-source-instance-generated-codec-report-v1.json"
    ))
}

fn projection_report_path_from_validation_commands(
    commands: &[String],
) -> Result<String, SyntaxBackendBoundaryError> {
    commands
        .iter()
        .find_map(|command| {
            command
                .split_once("--validate-report ")
                .map(|(_, path)| path)
        })
        .map(|path| path.split_whitespace().next().unwrap_or(path).to_string())
        .ok_or_else(|| {
            SyntaxBackendBoundaryError::new(
                "validation_commands",
                "source-instance projection report must include a --validate-report command",
            )
        })
}

fn generated_codec_report_validation_commands(
    report: &SyntaxGoSourceInstanceProjectionReport,
) -> Vec<String> {
    let mut commands = report.validation_commands.clone();
    commands.push("cargo test --test syntax_go_source_instance_projection_test".to_string());
    commands.push(GENERATED_CODEC_REPORT_COMMAND.to_string());
    commands
}

fn validate_projection_report_commands(
    commands: &[String],
    source_path: &str,
    metadata_path: &str,
    manifest_path: &str,
    candidate_grammar_id: &str,
    projection_output: &str,
) -> Result<(), SyntaxBackendBoundaryError> {
    if candidate_grammar_id == GO_UNIFIED_GRAMMAR_ID {
        // Inventory-as-metadata records validate the metadata input through
        // the inventory self-test instead of the per-file metadata smoke.
        require_command_containing(
            "validation_commands",
            commands,
            "go run scripts/analysis/go_ast_schema_inventory.go --validate",
        )?;
    } else {
        require_command_containing(
            "validation_commands",
            commands,
            "go run scripts/analysis/go_syntax_metadata_smoke.go",
        )?;
    }
    require_command_containing(
        "validation_commands",
        commands,
        "go run scripts/analysis/go_syntax_source_instance_project.go",
    )?;
    require_command_containing("validation_commands", commands, "--source")?;
    require_command_containing("validation_commands", commands, source_path)?;
    require_command_containing("validation_commands", commands, "--metadata")?;
    require_command_containing("validation_commands", commands, metadata_path)?;
    require_command_containing("validation_commands", commands, "--manifest-path")?;
    require_command_containing("validation_commands", commands, manifest_path)?;
    require_command_containing("validation_commands", commands, "--candidate-grammar-id")?;
    require_command_containing("validation_commands", commands, candidate_grammar_id)?;
    require_command_containing("validation_commands", commands, "--projection-output")?;
    require_command_containing("validation_commands", commands, projection_output)?;
    require_command_containing("validation_commands", commands, "--validate-report")?;
    require_command_containing(
        "validation_commands",
        commands,
        "go test scripts/analysis/go_syntax_source_instance_project.go scripts/analysis/go_syntax_source_instance_project_test.go",
    )?;
    require_command_containing("validation_commands", commands, "git diff --check")
}

fn validate_generated_codec_report_commands(
    commands: &[String],
    projection_report: &SyntaxGoSourceInstanceProjectionReport,
) -> Result<(), SyntaxBackendBoundaryError> {
    for required in generated_codec_report_validation_commands(projection_report) {
        if !commands.iter().any(|command| command == &required) {
            return Err(SyntaxBackendBoundaryError::new(
                "projection.generated_codec_report.validation_commands",
                format!("missing {required}"),
            ));
        }
    }
    Ok(())
}

fn validate_go_syntax_grammar_id(
    field: &'static str,
    candidate_grammar_id: &str,
) -> Result<(), SyntaxBackendBoundaryError> {
    require_non_empty(field, candidate_grammar_id)?;
    if candidate_grammar_id == GO_UNIFIED_GRAMMAR_ID
        || (candidate_grammar_id.starts_with("go_") && candidate_grammar_id.ends_with("_syntax_v0"))
    {
        Ok(())
    } else {
        Err(SyntaxBackendBoundaryError::new(
            field,
            "Go source-instance grammar IDs must be the unified inventory grammar id or use go_*_syntax_v0",
        ))
    }
}

/// Forward gate (M1): reject new per-file Go grammar registrations.
///
/// Two-tier policy over the shape check above: the unified inventory grammar
/// id is the only accepted forward id for Go source-instance records; retired
/// per-file `go_*_source_file_syntax_v0` ids stay parseable only when the
/// checked report records an explicit [`SourceInstancePerFileGrammarBlocker`]
/// or the id is one of the frozen historical `SYNTAX_CODEC_REGISTRY` scratch
/// routes (pinned to exactly two Go entries by the registry-freeze test in
/// `tests/syntax_generated_codec_gate_test.rs`), whose source-driven
/// regression reports are synthesized without a blocker field. Per-construct
/// `go_*_syntax_v0` scratch ids are registered scratch routes, not per-file
/// registrations, and remain accepted without a blocker. The nested artifact
/// validators keep the shape-only check because their grammar id is copied
/// from a report that already passed this door.
fn validate_forward_go_grammar_registration(
    candidate_grammar_id: &str,
    per_file_grammar_blocker: Option<&SourceInstancePerFileGrammarBlocker>,
) -> Result<(), SyntaxBackendBoundaryError> {
    if candidate_grammar_id == GO_UNIFIED_GRAMMAR_ID {
        return match per_file_grammar_blocker {
            Some(_) => Err(SyntaxBackendBoundaryError::new(
                "per_file_grammar_blocker",
                format!(
                    "per_file_grammar_blocker must not be recorded for the unified inventory grammar id {GO_UNIFIED_GRAMMAR_ID}"
                ),
            )),
            None => Ok(()),
        };
    }
    if candidate_grammar_id.ends_with("_source_file_syntax_v0") {
        return match per_file_grammar_blocker {
            Some(blocker) => blocker.validate(),
            None if frozen_registered_go_source_file_route(candidate_grammar_id) => Ok(()),
            None => Err(SyntaxBackendBoundaryError::new(
                "candidate_grammar_id",
                format!(
                    "new per-file Go grammar registrations are retired from the forward gate: \
                     {candidate_grammar_id} must use the unified inventory grammar id \
                     {GO_UNIFIED_GRAMMAR_ID} pinned by grammar artifact \
                     {GO_UNIFIED_GRAMMAR_ARTIFACT_ID}; a historical checked fixture may keep \
                     its per-file id only by recording the per_file_grammar_blocker escape hatch"
                ),
            )),
        };
    }
    match per_file_grammar_blocker {
        Some(_) => Err(SyntaxBackendBoundaryError::new(
            "per_file_grammar_blocker",
            "per_file_grammar_blocker applies only to go_*_source_file_syntax_v0 grammar ids",
        )),
        None => Ok(()),
    }
}

/// True for the frozen historical per-file Go scratch routes registered in
/// `SYNTAX_CODEC_REGISTRY`. The registry-freeze test pins the set to exactly
/// two entries, so this exemption cannot silently grow.
fn frozen_registered_go_source_file_route(candidate_grammar_id: &str) -> bool {
    crate::syntax_production_route::syntax_codec_registry_entry(candidate_grammar_id)
        .is_some_and(|entry| entry.target_id == "go")
}

fn source_instance_structural_model_sidecar(
    report: &SyntaxGoSourceInstanceProjectionReport,
) -> Result<GrammarStructuralModelSidecar, SyntaxBackendBoundaryError> {
    if report.candidate_grammar_id == GO_UNIFIED_GRAMMAR_ID {
        // Unified inventory route: every current-plus record validates and
        // emits against the one pinned inventory-derived artifact.
        return Ok(go_unified_grammar_artifact()?.sidecar.clone());
    }
    let grammar = GoSyntaxMetadataAdapter::new(&report.candidate_grammar_id)
        .lower_metadata_file(Path::new(&report.metadata.path))
        .map_err(|error| {
            SyntaxBackendBoundaryError::new(
                "projection.token_layout_mapping.metadata",
                format!("failed to lower Go syntax metadata: {error}"),
            )
        })?;
    lower_grammar_ir_to_structural_model(&grammar, go_syntax_source_file_token_layout()).map_err(
        |error| {
            SyntaxBackendBoundaryError::new(
                "projection.token_layout_mapping.structural_model",
                format!("failed to lower GrammarIR to StructuralModel: {error}"),
            )
        },
    )
}

fn validate_source_instance_sidecar(
    sidecar: &GrammarStructuralModelSidecar,
    report: &SyntaxGoSourceInstanceProjectionReport,
) -> Result<(), SyntaxBackendBoundaryError> {
    require_string(
        "projection.token_layout_mapping.sidecar.grammar_id",
        &sidecar.input.grammar_id,
        &report.candidate_grammar_id,
    )?;
    require_string(
        "projection.token_layout_mapping.sidecar.metadata_path",
        sidecar
            .input
            .provenance
            .metadata_path
            .as_deref()
            .unwrap_or(""),
        &report.metadata.path,
    )?;
    require_string(
        "projection.token_layout_mapping.sidecar.metadata_sha256",
        sidecar
            .input
            .provenance
            .metadata_sha256
            .as_deref()
            .unwrap_or(""),
        &report.metadata.sha256,
    )?;
    require_usize(
        "projection.token_layout_mapping.sidecar.records",
        sidecar.structural_model.records.len(),
        report.metadata.rule_count,
    )?;
    let field_count = sidecar
        .structural_model
        .records
        .iter()
        .map(|record| record.fields.len())
        .sum();
    require_usize(
        "projection.token_layout_mapping.sidecar.fields",
        field_count,
        report.metadata.field_count,
    )
}

fn terminal_kind_name(terminal: GrammarTerminalKind) -> &'static str {
    match terminal {
        GrammarTerminalKind::Nonterminal => "nonterminal",
        GrammarTerminalKind::IdentifierPayload => "identifier_payload",
        GrammarTerminalKind::StringPayload => "string_payload",
        GrammarTerminalKind::IntegerPayload => "integer_payload",
        GrammarTerminalKind::BooleanPayload => "boolean_payload",
    }
}

fn terminal_payload_kind(terminal: &str) -> Result<&'static str, SyntaxBackendBoundaryError> {
    match terminal {
        "identifier_payload" => Ok("identifier"),
        "string_payload" => Ok("string"),
        "integer_payload" => Ok("integer"),
        "boolean_payload" => Ok("boolean"),
        other => Err(SyntaxBackendBoundaryError::new(
            "projection.token_layout_mapping.terminal",
            format!("unsupported terminal kind {other}"),
        )),
    }
}

fn contextual_structural_token(
    context: &ProjectionTokenLayoutContext,
    offset: u32,
    name: &'static str,
) -> Result<u32, SyntaxBackendBoundaryError> {
    let token = context.structural_range.start + offset;
    if token < context.structural_range.end_exclusive {
        Ok(token)
    } else {
        Err(SyntaxBackendBoundaryError::new(
            "projection.raw_token_ids.structural_token",
            format!(
                "contextual structural token {name} offset {offset} exceeds range {}",
                context.structural_range.id
            ),
        ))
    }
}

fn validate_raw_token_segments(
    segments: &[ProjectionRawTokenSegment],
    raw_token_count: usize,
) -> Result<(), SyntaxBackendBoundaryError> {
    let mut expected_offset = 0;
    for (expected_ordinal, segment) in segments.iter().enumerate() {
        require_usize(
            "projection.raw_token_ids.raw_token_segments.event_ordinal",
            segment.event_ordinal,
            expected_ordinal,
        )?;
        require_non_empty(
            "projection.raw_token_ids.raw_token_segments.event_kind",
            &segment.event_kind,
        )?;
        require_non_empty(
            "projection.raw_token_ids.raw_token_segments.primary_token_range_role",
            &segment.primary_token_range_role,
        )?;
        require_usize(
            "projection.raw_token_ids.raw_token_segments.raw_token_offset",
            segment.raw_token_offset,
            expected_offset,
        )?;
        if segment.raw_token_count == 0 {
            return Err(SyntaxBackendBoundaryError::new(
                "projection.raw_token_ids.raw_token_segments.raw_token_count",
                "raw token segments must be non-empty",
            ));
        }
        expected_offset += segment.raw_token_count;
    }
    require_usize(
        "projection.raw_token_ids.raw_token_segments.coverage",
        expected_offset,
        raw_token_count,
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
            format!("expected {expected}, got {actual}"),
        ))
    }
}

fn require_contains(
    field: &'static str,
    actual: &str,
    required: &str,
) -> Result<(), SyntaxBackendBoundaryError> {
    if actual.contains(required) {
        Ok(())
    } else {
        Err(SyntaxBackendBoundaryError::new(
            field,
            format!("missing required text {required}"),
        ))
    }
}

/// Accept a Go source path for the unified inventory grammar without requiring
/// a GOROOT-style `/src/` segment (needed for external module stress corpora).
fn require_go_source_path_suffix(
    field: &'static str,
    path: &str,
) -> Result<(), SyntaxBackendBoundaryError> {
    let normalized = path.replace('\\', "/");
    if normalized.ends_with(".go") {
        Ok(())
    } else {
        Err(SyntaxBackendBoundaryError::new(
            field,
            format!("expected a .go source path, got {path}"),
        ))
    }
}

fn require_non_empty(field: &'static str, actual: &str) -> Result<(), SyntaxBackendBoundaryError> {
    if actual.is_empty() {
        Err(SyntaxBackendBoundaryError::new(
            field,
            "expected non-empty string",
        ))
    } else {
        Ok(())
    }
}

fn require_positive_usize(
    field: &'static str,
    actual: usize,
) -> Result<(), SyntaxBackendBoundaryError> {
    if actual == 0 {
        Err(SyntaxBackendBoundaryError::new(
            field,
            "expected positive integer",
        ))
    } else {
        Ok(())
    }
}

fn require_u32(
    field: &'static str,
    actual: u32,
    expected: u32,
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

fn require_command_containing(
    field: &'static str,
    commands: &[String],
    required: &str,
) -> Result<(), SyntaxBackendBoundaryError> {
    if commands.iter().any(|command| command.contains(required)) {
        Ok(())
    } else {
        Err(SyntaxBackendBoundaryError::new(
            field,
            format!("missing command text {required}"),
        ))
    }
}

fn require_all(
    field: &'static str,
    actual: &[String],
    required: &[&str],
) -> Result<(), SyntaxBackendBoundaryError> {
    let actual = actual.iter().map(String::as_str).collect::<BTreeSet<_>>();
    for expected in required {
        if !actual.contains(expected) {
            return Err(SyntaxBackendBoundaryError::new(
                field,
                format!("missing {expected}"),
            ));
        }
    }
    Ok(())
}

fn require_sorted_unique_strings(
    field: &'static str,
    actual: &[String],
) -> Result<(), SyntaxBackendBoundaryError> {
    require_positive_usize(field, actual.len())?;
    let mut previous = None;
    for value in actual {
        require_non_empty(field, value)?;
        if let Some(previous) = previous
            && previous >= value.as_str()
        {
            return Err(SyntaxBackendBoundaryError::new(
                field,
                "expected sorted unique strings",
            ));
        }
        previous = Some(value.as_str());
    }
    Ok(())
}

fn require_projection_identity<T: PartialEq>(
    field: &'static str,
    actual: &T,
    expected: &T,
) -> Result<(), SyntaxBackendBoundaryError> {
    if actual == expected {
        Ok(())
    } else {
        Err(SyntaxBackendBoundaryError::new(
            field,
            "projection artifact must preserve the checked report identity fields",
        ))
    }
}

fn require_projection_stats_match(
    field: &'static str,
    actual: &SourceInstanceProjectionStats,
    expected: &SourceInstanceProjectionStats,
) -> Result<(), SyntaxBackendBoundaryError> {
    if actual == expected {
        Ok(())
    } else {
        Err(SyntaxBackendBoundaryError::new(
            field,
            "projection artifact stats must match the projected AST value tree and checked report",
        ))
    }
}

fn require_sha256(field: &'static str, actual: &str) -> Result<(), SyntaxBackendBoundaryError> {
    if actual.len() == 64
        && actual
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        Ok(())
    } else {
        Err(SyntaxBackendBoundaryError::new(
            field,
            "expected lowercase SHA-256 hex digest",
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn optional_field(
        name: &str,
        missing: bool,
        values: Vec<SourceInstanceProjectionValue>,
    ) -> SourceInstanceProjectionField {
        SourceInstanceProjectionField {
            name: name.to_string(),
            required: false,
            multiple: false,
            terminal: None,
            values,
            missing,
        }
    }

    fn terminal_value(payload: &str) -> SourceInstanceProjectionValue {
        SourceInstanceProjectionValue {
            kind: "terminal".to_string(),
            node_kind: "Ident".to_string(),
            terminal: Some("ident".to_string()),
            payload: Some(payload.to_string()),
            token_name: None,
            node: None,
        }
    }

    #[test]
    fn optional_empty_present_field_rejected() {
        let field = optional_field("Doc", false, Vec::new());
        let error = field
            .ensure_presence_contract()
            .expect_err("empty present optional field must fail");
        let message = error.to_string();
        assert!(
            message.contains("missing=false") && message.contains("missing=true"),
            "error should tell the caller to set missing=true: {message}"
        );
    }

    #[test]
    fn optional_missing_field_accepted() {
        let field = optional_field("Doc", true, Vec::new());
        field
            .ensure_presence_contract()
            .expect("missing optional field is valid");
    }

    #[test]
    fn optional_present_field_with_value_accepted() {
        let field = optional_field("Doc", false, vec![terminal_value("x")]);
        field
            .ensure_presence_contract()
            .expect("present optional field with a value is valid");
    }

    #[test]
    fn unified_grammar_accepts_external_module_go_source_paths() {
        let dir = std::env::temp_dir().join(format!(
            "tct-external-module-source-path-{}",
            std::process::id()
        ));
        let source_path = dir
            .join("modules")
            .join("example.com__mod__v1.0.0")
            .join("pkg")
            .join("a.go");
        fs::create_dir_all(source_path.parent().expect("parent")).expect("mkdir");
        let source_bytes = b"package pkg\n\nfunc A() int { return 1 }\n";
        fs::write(&source_path, source_bytes).expect("write source");
        let source = SourceInstanceProjectionSource {
            path: source_path.to_string_lossy().into_owned(),
            sha256: sha256_hex(source_bytes),
            bytes: source_bytes.len(),
        };
        source
            .validate_checked_file(GO_UNIFIED_GRAMMAR_ID)
            .expect("external module *.go path must be accepted for unified grammar");

        let non_go = SourceInstanceProjectionSource {
            path: source_path
                .with_extension("txt")
                .to_string_lossy()
                .into_owned(),
            sha256: source.sha256.clone(),
            bytes: source.bytes,
        };
        let error = non_go
            .validate_checked_file(GO_UNIFIED_GRAMMAR_ID)
            .expect_err("non-.go path must still be rejected");
        assert_eq!(error.field(), "source.path");
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn required_empty_field_still_rejected() {
        let field = SourceInstanceProjectionField {
            name: "Name".to_string(),
            required: true,
            multiple: false,
            terminal: None,
            values: Vec::new(),
            missing: false,
        };
        let error = field
            .ensure_presence_contract()
            .expect_err("required empty field must fail");
        assert!(
            error
                .to_string()
                .contains("required fields must carry a projected value"),
            "unexpected error: {error}"
        );
    }

    #[test]
    fn empty_present_optional_does_not_emit_bare_field_span() {
        let field = optional_field("Doc", false, Vec::new());
        let mut builder = ProjectionCodecInputBuilder::default();
        let error = field
            .append_codec_input_events(&mut builder)
            .expect_err("must not emit bare field_start/field_end");
        assert!(
            error.to_string().contains("strict decode rejects"),
            "unexpected error: {error}"
        );
        assert!(
            builder.events.is_empty(),
            "rejected fields must not leave partial codec events"
        );
    }

    /// Build a minimal projection JSON whose nested `root` exceeds serde_json's
    /// default recursion limit (128). Each level is:
    /// node -> fields[0] -> values[0].node -> ...
    fn deep_projection_artifact_json(depth: usize) -> String {
        assert!(depth >= 1);
        let mut node = serde_json::json!({
            "node_kind": "Ident",
            "rule_name": "Ident",
            "fields": []
        });
        for level in (1..depth).rev() {
            node = serde_json::json!({
                "node_kind": "ParenExpr",
                "rule_name": format!("ParenExpr_{level}"),
                "fields": [{
                    "name": "X",
                    "required": true,
                    "multiple": false,
                    "values": [{
                        "kind": "node",
                        "node_kind": node["node_kind"].clone(),
                        "node": node
                    }],
                    "missing": false
                }]
            });
        }
        let artifact = serde_json::json!({
            "artifact_kind": PROJECTION_ARTIFACT_KIND,
            "version": VERSION,
            "tool": REPORT_TOOL,
            "status": PROJECTION_STATUS,
            "target": TARGET,
            "candidate_grammar_id": GO_UNIFIED_GRAMMAR_ID,
            "manifest_path": "deep/nesting.go",
            "source": {
                "path": "tests/fixtures/go_syntax_corpus/src/deep/nesting.go",
                "sha256": "00".repeat(32),
                "bytes": 1
            },
            "metadata": {
                "path": "tests/fixtures/go_ast_schema_inventory/go1.24.3-ast-schema-inventory-v1.json",
                "sha256": "11".repeat(32),
                "artifact_kind": GO_AST_SCHEMA_INVENTORY_METADATA_ARTIFACT_KIND,
                "version": 1,
                "rule_count": 1,
                "field_count": 1
            },
            "parser": {
                "metadata_tool": "go",
                "metadata_version": "go1.24.3",
                "metadata_package": "go/parser",
                "metadata_package_version": "go1.24.3",
                "runtime_go_version": "go1.24.3"
            },
            "root": node,
            "stats": {
                "node_count": depth,
                "field_count": depth.saturating_sub(1).max(1),
                "field_value_count": depth.saturating_sub(1).max(1),
                "terminal_payload_count": 1,
                "optional_missing_count": 0,
                "repeated_empty_count": 0,
                "max_depth": depth,
                "distinct_node_kinds": ["Ident", "ParenExpr"],
                "terminal_payloads_by_kind": {}
            },
            "claim_boundary": "parser-backed source-instance projection evidence only; generated-codec token production and arena-IR and stable-backend claims stay blocked",
            "no_stable_claims": ["no_generated_codec_report_claim"]
        });
        artifact.to_string()
    }

    #[test]
    fn parse_projection_artifact_accepts_trees_deeper_than_serde_default_recursion_limit() {
        // serde_json defaults to a recursion limit of 128. Real stdlib
        // projections such as cmd/cgo/gcc.go nest past that (depth ~156).
        let depth = 140;
        let contents = deep_projection_artifact_json(depth);
        let default_parse = serde_json::from_str::<SourceInstanceProjectionArtifact>(&contents);
        assert!(
            default_parse.is_err(),
            "control: default serde_json recursion limit should reject depth {depth}"
        );
        let artifact = parse_projection_artifact(&contents)
            .expect("projection parser must accept trees deeper than serde_json's default limit");
        assert_eq!(artifact.root.node_kind, "ParenExpr");
        assert_eq!(artifact.stats.max_depth, depth);
    }
}
