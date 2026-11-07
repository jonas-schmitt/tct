//! Shared raw syntax scratch-generation evidence helpers.
//!
//! These helpers describe only the target-neutral raw state-machine boundary:
//! empty raw prefix, finite admissible masks, selected raw-token transitions,
//! strict incomplete-prefix rejection, and token-only decode summaries. Target
//! parser, formatter, renderer, and source metadata facts stay in the Go/C#
//! callers that own those sidecar contracts.

use crate::provenance::json_sha256;
use crate::syntax_backend_boundary::SyntaxBackendBoundaryError;
use crate::syntax_state_machine::{
    CompiledSyntaxModel, RawSyntaxDecodedTree, RawSyntaxError, RawSyntaxSelectedCompletionTrace,
    RawSyntaxState, RawSyntaxStateMachineCore, singleton_admissible_raw_token,
};
use std::collections::BTreeSet;

pub const RAW_SCRATCH_GENERATOR_CORE_API_ID: &str = "raw-syntax-scratch-generator-core-v1";
pub const RAW_SCRATCH_GENERATOR_CORE_API_BOUNDARY: &str = "precompiled syntax artifact + raw token prefix -> state -> finite admissible raw-token IDs -> selected transition -> token-only decode; elided selected-completion via CompiledSyntaxModel begin_elided_stream when the live stream contract is elided";

pub struct RawScratchGeneratorCore<'a> {
    core: &'a dyn RawSyntaxStateMachineCore,
}

impl<'a> RawScratchGeneratorCore<'a> {
    pub fn new(core: &'a dyn RawSyntaxStateMachineCore) -> Self {
        Self { core }
    }

    pub fn api_id(&self) -> &'static str {
        RAW_SCRATCH_GENERATOR_CORE_API_ID
    }

    pub fn api_boundary(&self) -> &'static str {
        RAW_SCRATCH_GENERATOR_CORE_API_BOUNDARY
    }

    pub fn empty_prefix(
        &self,
        error_field: &'static str,
    ) -> Result<RawScratchEmptyPrefixSummary, SyntaxBackendBoundaryError> {
        RawScratchEmptyPrefixSummary::from_core(self.core, error_field)
    }

    /// Target-neutral elided selected-completion walk over a compiled model.
    ///
    /// Elided stream APIs (`begin_elided_stream`, `expand_elided_raw_tokens`,
    /// `decode_complete_tree_elided`) are owned by `CompiledSyntaxModel` rather
    /// than the generic `RawSyntaxStateMachineCore` trait surface. This method
    /// is the reusable generator-core entrypoint for that contract so Go
    /// source-record evidence (and future elided-stream targets) do not embed
    /// the walk in a language megamodule. Expanded selected-completion remains
    /// available via [`Self::selected_completion`].
    pub fn elided_selected_completion(
        model: &CompiledSyntaxModel,
        elided_token_ids: &[u32],
        expected_expanded_token_ids: &[u32],
        error_field: &'static str,
    ) -> Result<RawScratchElidedSelectedCompletionRun, SyntaxBackendBoundaryError> {
        RawScratchElidedSelectedCompletionRun::from_compiled_model(
            model,
            elided_token_ids,
            expected_expanded_token_ids,
            error_field,
        )
    }

    pub fn reject_empty_prefix_candidate(
        &self,
        candidate_raw_token_id: u32,
        error_field: &'static str,
    ) -> Result<RawScratchRejectedTransitionSummary, SyntaxBackendBoundaryError> {
        let transition = self
            .core
            .candidate_raw_token_transition(&[], candidate_raw_token_id);
        if transition.accepted {
            return Err(SyntaxBackendBoundaryError::new(
                error_field,
                format!(
                    "candidate raw token {candidate_raw_token_id} should be rejected from the empty prefix"
                ),
            ));
        }
        let error = transition.error.as_ref().ok_or_else(|| {
            SyntaxBackendBoundaryError::new(
                error_field,
                "rejected empty-prefix candidate transition must carry an error",
            )
        })?;

        Ok(RawScratchRejectedTransitionSummary {
            candidate_raw_token_id,
            rejected: true,
            error_kind: error.kind.to_string(),
        })
    }

    pub fn selected_completion(
        &self,
        raw_token_ids: &[u32],
        sample_steps: &BTreeSet<usize>,
        error_field: &'static str,
    ) -> Result<RawScratchSelectedCompletionRun, SyntaxBackendBoundaryError> {
        let trace = self.selected_completion_trace(raw_token_ids, sample_steps, error_field)?;
        let summary =
            RawScratchSelectedCompletionSummary::from_trace(raw_token_ids, &trace, error_field)?;
        let token_only_decode = self.token_only_decode(
            raw_token_ids,
            "raw_scratch_generator_core.token_only_decode",
        )?;
        let raw_token_ids_sha256 = json_sha256(&raw_token_ids).map_err(|error| {
            SyntaxBackendBoundaryError::new(
                error_field,
                format!("failed to hash selected raw-token stream: {error}"),
            )
        })?;

        Ok(RawScratchSelectedCompletionRun {
            raw_token_ids_sha256,
            summary,
            decoded_tree: token_only_decode.decoded_tree,
        })
    }

    pub fn incomplete_prefix(
        &self,
        raw_token_ids: &[u32],
        prefix_raw_token_count: usize,
        error_field: &'static str,
    ) -> Result<RawScratchIncompletePrefixProbe, SyntaxBackendBoundaryError> {
        if prefix_raw_token_count == 0 {
            return Err(SyntaxBackendBoundaryError::new(
                error_field,
                "incomplete-prefix probe requires at least one prefix raw token",
            ));
        }
        if raw_token_ids.len() <= prefix_raw_token_count {
            return Err(SyntaxBackendBoundaryError::new(
                error_field,
                "incomplete-prefix probe requires at least one continuation raw token",
            ));
        }

        let prefix_raw_token_ids = raw_token_ids[..prefix_raw_token_count].to_vec();
        let prefix_state = self
            .core
            .state_after_raw_prefix(&prefix_raw_token_ids)
            .map_err(|error| {
                SyntaxBackendBoundaryError::new(
                    error_field,
                    format!(
                        "selected raw-token prefix should be accepted as partial: {}",
                        error.message
                    ),
                )
            })?;
        let strict_completion_error = match self.core.strict_decode_complete(&prefix_raw_token_ids)
        {
            Ok(_) => {
                return Err(SyntaxBackendBoundaryError::new(
                    error_field,
                    "selected incomplete prefix must not strict-decode as a complete stream",
                ));
            }
            Err(error) => error,
        };
        let strict_incomplete = RawScratchStrictIncompletePrefixSummary::from_error(
            &strict_completion_error,
            prefix_raw_token_count,
        );
        let selected_next_raw_token_id = raw_token_ids[prefix_raw_token_count];
        let selected_next_raw_token_admitted = prefix_state
            .admissible
            .finite_raw_token_ids
            .contains(&selected_next_raw_token_id);
        let selected_next_transition = self
            .core
            .candidate_raw_token_transition(&prefix_raw_token_ids, selected_next_raw_token_id);
        let selected_next_transition_state =
            selected_next_transition.state.as_ref().ok_or_else(|| {
                let diagnostic = selected_next_transition
                    .error
                    .as_ref()
                    .map_or("missing transition error".to_string(), |error| {
                        error.message.clone()
                    });
                SyntaxBackendBoundaryError::new(
                    error_field,
                    format!("selected continuation raw token should transition: {diagnostic}"),
                )
            })?;
        let final_completion_state =
            self.core.strict_decode_complete(raw_token_ids).map_err(|error| {
                SyntaxBackendBoundaryError::new(
                    error_field,
                    format!(
                        "complete selected raw-token stream should strict-decode after prefix continuation: {}",
                        error.message
                    ),
                )
            })?;
        let prefix_raw_token_ids_sha256 = json_sha256(&prefix_raw_token_ids).map_err(|error| {
            SyntaxBackendBoundaryError::new(
                error_field,
                format!("failed to hash selected incomplete-prefix raw-token IDs: {error}"),
            )
        })?;

        Ok(RawScratchIncompletePrefixProbe {
            prefix_raw_token_ids,
            prefix_raw_token_ids_sha256,
            prefix_state,
            strict_incomplete,
            selected_next_raw_token_id,
            selected_next_raw_token_admitted,
            selected_next_transition_accepted: selected_next_transition.accepted,
            selected_next_transition_state: selected_next_transition_state.clone(),
            final_completion_state,
        })
    }

    pub fn token_only_decode(
        &self,
        raw_token_ids: &[u32],
        error_field: &'static str,
    ) -> Result<RawScratchTokenOnlyDecodeResult, SyntaxBackendBoundaryError> {
        let raw_token_ids_sha256 = json_sha256(&raw_token_ids).map_err(|error| {
            SyntaxBackendBoundaryError::new(
                error_field,
                format!("failed to hash token-only raw-token stream: {error}"),
            )
        })?;
        let decoded_tree = self
            .core
            .decode_complete_tree(raw_token_ids)
            .map_err(|error| {
                SyntaxBackendBoundaryError::new(
                    error_field,
                    format!(
                        "raw-token-only stream should decode to a reusable raw syntax tree: {}",
                        error.message
                    ),
                )
            })?;
        let summary = RawScratchTokenOnlyDecodeSummary::from_tree(
            raw_token_ids.len(),
            raw_token_ids_sha256,
            &decoded_tree,
            error_field,
        )?;

        Ok(RawScratchTokenOnlyDecodeResult {
            summary,
            decoded_tree,
        })
    }

    fn selected_completion_trace(
        &self,
        raw_token_ids: &[u32],
        sample_steps: &BTreeSet<usize>,
        error_field: &'static str,
    ) -> Result<RawSyntaxSelectedCompletionTrace, SyntaxBackendBoundaryError> {
        self.core
            .selected_completion_trace(raw_token_ids, sample_steps)
            .map_err(|error| {
                SyntaxBackendBoundaryError::new(
                    error_field,
                    format!(
                        "selected raw-token stream should trace through admitted state-machine transitions: {}",
                        error.message
                    ),
                )
            })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RawScratchRejectedTransitionSummary {
    pub candidate_raw_token_id: u32,
    pub rejected: bool,
    pub error_kind: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RawScratchSelectedCompletionRun {
    pub raw_token_ids_sha256: String,
    pub summary: RawScratchSelectedCompletionSummary,
    pub decoded_tree: RawSyntaxDecodedTree,
}

/// Result of walking a canonical elided raw-token stream through the shared
/// generator-core surface: admission at non-forced positions, expand parity
/// against the expected expanded stream, and elided-path token-only decode.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RawScratchElidedSelectedCompletionRun {
    pub elided_token_count: usize,
    pub elided_tokens_sha256: String,
    pub expanded_token_count: usize,
    pub forced_token_count: usize,
    pub expansion_matches_expanded_stream: bool,
    pub every_selected_elided_token_admitted: bool,
    pub every_elided_position_non_forced: bool,
    pub transition_count: usize,
    pub admissible_mask_query_count: usize,
    pub first_selected_elided_raw_token_id: Option<u32>,
    pub final_selected_elided_raw_token_id: Option<u32>,
    pub final_state: RawScratchStateSummary,
    pub decoded_tree: RawSyntaxDecodedTree,
}

impl RawScratchElidedSelectedCompletionRun {
    pub fn from_compiled_model(
        model: &CompiledSyntaxModel,
        elided_token_ids: &[u32],
        expected_expanded_token_ids: &[u32],
        error_field: &'static str,
    ) -> Result<Self, SyntaxBackendBoundaryError> {
        let mut walker = model.begin_elided_stream().map_err(|error| {
            SyntaxBackendBoundaryError::new(
                error_field,
                format!(
                    "elided selected-completion must open an elided stream: {}",
                    error.message
                ),
            )
        })?;

        let mut every_selected_elided_token_admitted = true;
        let mut every_elided_position_non_forced = true;
        let mut transition_count = 0usize;
        let mut admissible_mask_query_count = 0usize;

        for &token in elided_token_ids {
            let admissible = walker.admissible_next_raw_tokens();
            admissible_mask_query_count += 1;
            if singleton_admissible_raw_token(&admissible).is_some() {
                every_elided_position_non_forced = false;
            }
            // Successful advance proves admission; track membership for the
            // finite list / range mask surface exposed to model-side samplers.
            let admitted = admissible.finite_raw_token_ids.contains(&token)
                || admissible
                    .raw_token_ranges
                    .iter()
                    .any(|range| token >= range.start && token < range.end_exclusive);
            if !admitted {
                every_selected_elided_token_admitted = false;
            }
            walker.advance(token).map_err(|error| {
                SyntaxBackendBoundaryError::new(
                    error_field,
                    format!(
                        "canonical elided token {token} at step {transition_count} must advance on the elided handle: {}",
                        error.message
                    ),
                )
            })?;
            transition_count += 1;
        }

        if !walker.eof_valid() {
            return Err(SyntaxBackendBoundaryError::new(
                error_field,
                "elided selected-completion stream must end EOF-valid",
            ));
        }
        let final_state = walker.state_snapshot();
        if final_state.consumed_raw_tokens != expected_expanded_token_ids.len() {
            return Err(SyntaxBackendBoundaryError::new(
                error_field,
                format!(
                    "elided selected-completion final consumed raw tokens {} must match expected expanded length {}",
                    final_state.consumed_raw_tokens,
                    expected_expanded_token_ids.len()
                ),
            ));
        }

        let reexpanded = model.expand_elided_raw_tokens(elided_token_ids).map_err(|error| {
            SyntaxBackendBoundaryError::new(
                error_field,
                format!(
                    "canonical elided stream must re-expand to the expected expanded stream: {}",
                    error.message
                ),
            )
        })?;
        let expansion_matches_expanded_stream = reexpanded == expected_expanded_token_ids;
        if !expansion_matches_expanded_stream {
            return Err(SyntaxBackendBoundaryError::new(
                error_field,
                "expand_elided_raw_tokens must reproduce the expected expanded stream",
            ));
        }

        let decoded_tree = model
            .decode_complete_tree_elided(elided_token_ids)
            .map_err(|error| {
                SyntaxBackendBoundaryError::new(
                    error_field,
                    format!(
                        "elided selected-completion stream must decode to a reusable raw syntax tree: {}",
                        error.message
                    ),
                )
            })?;
        if decoded_tree.state.consumed_raw_tokens != expected_expanded_token_ids.len() {
            return Err(SyntaxBackendBoundaryError::new(
                error_field,
                format!(
                    "elided decode consumed raw tokens {} must match expected expanded length {}",
                    decoded_tree.state.consumed_raw_tokens,
                    expected_expanded_token_ids.len()
                ),
            ));
        }

        let elided_tokens_sha256 = json_sha256(&elided_token_ids).map_err(|error| {
            SyntaxBackendBoundaryError::new(
                error_field,
                format!("failed to hash elided selected-completion stream: {error}"),
            )
        })?;
        let expanded_token_count = expected_expanded_token_ids.len();
        let forced_token_count = expanded_token_count.saturating_sub(elided_token_ids.len());

        Ok(Self {
            elided_token_count: elided_token_ids.len(),
            elided_tokens_sha256,
            expanded_token_count,
            forced_token_count,
            expansion_matches_expanded_stream,
            every_selected_elided_token_admitted,
            every_elided_position_non_forced,
            transition_count,
            admissible_mask_query_count,
            first_selected_elided_raw_token_id: elided_token_ids.first().copied(),
            final_selected_elided_raw_token_id: elided_token_ids.last().copied(),
            final_state: RawScratchStateSummary::from_state(&final_state),
            decoded_tree,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RawScratchIncompletePrefixProbe {
    pub prefix_raw_token_ids: Vec<u32>,
    pub prefix_raw_token_ids_sha256: String,
    pub prefix_state: RawSyntaxState,
    pub strict_incomplete: RawScratchStrictIncompletePrefixSummary,
    pub selected_next_raw_token_id: u32,
    pub selected_next_raw_token_admitted: bool,
    pub selected_next_transition_accepted: bool,
    pub selected_next_transition_state: RawSyntaxState,
    pub final_completion_state: RawSyntaxState,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RawScratchTokenOnlyDecodeResult {
    pub summary: RawScratchTokenOnlyDecodeSummary,
    pub decoded_tree: RawSyntaxDecodedTree,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RawScratchClaimBoundaryFlags {
    pub per_output_parser_metadata_required: bool,
    pub source_instance_segment_sidecar_used: bool,
    pub source_prefix_table_used: bool,
    pub arena_ir_used: bool,
    pub route_handoff_provenance_used: bool,
    pub bpe_validity_authority_used: bool,
    pub stable_backend_claimed: bool,
}

impl RawScratchClaimBoundaryFlags {
    pub fn raw_state_machine_only() -> Self {
        Self {
            per_output_parser_metadata_required: false,
            source_instance_segment_sidecar_used: false,
            source_prefix_table_used: false,
            arena_ir_used: false,
            route_handoff_provenance_used: false,
            bpe_validity_authority_used: false,
            stable_backend_claimed: false,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RawScratchStateSummary {
    pub observed_status: String,
    pub consumed_raw_tokens: usize,
    pub total_raw_tokens: usize,
    pub eof_valid: bool,
    pub stack_depth: usize,
    pub active_rule: Option<String>,
    pub active_field: Option<String>,
}

impl RawScratchStateSummary {
    pub fn from_state(state: &RawSyntaxState) -> Self {
        Self {
            observed_status: state.status.as_str().to_string(),
            consumed_raw_tokens: state.consumed_raw_tokens,
            total_raw_tokens: state.total_raw_tokens,
            eof_valid: state.eof_valid,
            stack_depth: state.stack_depth,
            active_rule: state.active_rule.clone(),
            active_field: state.active_field.clone(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RawScratchEmptyPrefixSummary {
    pub raw_state_machine_core_boundary_id: String,
    pub raw_state_machine_core_boundary: String,
    pub token_layout_vocab_size: u32,
    pub pad_token_id: u32,
    pub initial_state: RawScratchStateSummary,
    pub finite_admissible_raw_token_count: usize,
    pub finite_admissible_raw_token_ids: Vec<u32>,
    pub finite_admissible_raw_token_ids_sha256: String,
    pub selected_raw_token_id: u32,
    pub selected_raw_token_admitted: bool,
    pub selected_transition_accepted: bool,
    pub selected_transition_state: RawScratchStateSummary,
}

impl RawScratchEmptyPrefixSummary {
    pub fn from_core(
        core: &(impl RawSyntaxStateMachineCore + ?Sized),
        error_field: &'static str,
    ) -> Result<Self, SyntaxBackendBoundaryError> {
        let initial_state = core.state_after_raw_prefix(&[]).map_err(|error| {
            SyntaxBackendBoundaryError::new(
                error_field,
                format!(
                    "empty raw syntax prefix should construct a live state: {}",
                    error.message
                ),
            )
        })?;
        let finite_admissible_raw_token_ids = initial_state.admissible.finite_raw_token_ids.clone();
        let finite_admissible_raw_token_ids_sha256 = json_sha256(&finite_admissible_raw_token_ids)
            .map_err(|error| {
                SyntaxBackendBoundaryError::new(
                    error_field,
                    format!("failed to hash empty-prefix finite admissible mask: {error}"),
                )
            })?;
        let selected_raw_token_id = finite_admissible_raw_token_ids
            .first()
            .copied()
            .ok_or_else(|| {
                SyntaxBackendBoundaryError::new(
                    error_field,
                    "empty raw syntax prefix must expose at least one finite raw token",
                )
            })?;
        let selected_transition = core.candidate_raw_token_transition(&[], selected_raw_token_id);
        let selected_state = selected_transition.state.as_ref().ok_or_else(|| {
            let diagnostic = selected_transition
                .error
                .as_ref()
                .map_or("missing transition error".to_string(), |error| {
                    error.message.clone()
                });
            SyntaxBackendBoundaryError::new(
                error_field,
                format!("selected empty-prefix raw token should transition: {diagnostic}"),
            )
        })?;

        Ok(Self {
            raw_state_machine_core_boundary_id: core.boundary_id().to_string(),
            raw_state_machine_core_boundary: core.boundary_contract().to_string(),
            token_layout_vocab_size: core.token_roles().vocab_size,
            pad_token_id: core.token_roles().pad_token,
            initial_state: RawScratchStateSummary::from_state(&initial_state),
            finite_admissible_raw_token_count: finite_admissible_raw_token_ids.len(),
            finite_admissible_raw_token_ids,
            finite_admissible_raw_token_ids_sha256,
            selected_raw_token_id,
            selected_raw_token_admitted: initial_state
                .admissible
                .finite_raw_token_ids
                .contains(&selected_raw_token_id),
            selected_transition_accepted: selected_transition.accepted,
            selected_transition_state: RawScratchStateSummary::from_state(selected_state),
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RawScratchSelectedCompletionSummary {
    pub raw_token_count: usize,
    pub transition_count: usize,
    pub admissible_mask_query_count: usize,
    pub finite_admissible_raw_token_mask_available: bool,
    pub every_selected_raw_token_admitted: bool,
    pub selected_transition_accepted_count: usize,
    pub first_selected_raw_token_id: u32,
    pub final_selected_raw_token_id: u32,
    pub final_state: RawScratchStateSummary,
    pub step_samples: Vec<RawScratchSelectedCompletionStepSummary>,
}

impl RawScratchSelectedCompletionSummary {
    pub fn from_trace(
        raw_token_ids: &[u32],
        trace: &RawSyntaxSelectedCompletionTrace,
        error_field: &'static str,
    ) -> Result<Self, SyntaxBackendBoundaryError> {
        let first_selected_raw_token_id = raw_token_ids.first().copied().ok_or_else(|| {
            SyntaxBackendBoundaryError::new(
                error_field,
                "selected completion stream must contain at least one raw token",
            )
        })?;
        let final_selected_raw_token_id = raw_token_ids.last().copied().ok_or_else(|| {
            SyntaxBackendBoundaryError::new(
                error_field,
                "selected completion stream must contain at least one raw token",
            )
        })?;

        Ok(Self {
            raw_token_count: raw_token_ids.len(),
            transition_count: trace.transition_count,
            admissible_mask_query_count: trace.admissible_mask_query_count,
            finite_admissible_raw_token_mask_available: trace
                .finite_admissible_raw_token_mask_available,
            every_selected_raw_token_admitted: trace.every_selected_raw_token_admitted,
            selected_transition_accepted_count: trace.selected_transition_accepted_count,
            first_selected_raw_token_id,
            final_selected_raw_token_id,
            final_state: RawScratchStateSummary::from_state(&trace.final_state),
            step_samples: trace
                .step_samples
                .iter()
                .map(RawScratchSelectedCompletionStepSummary::from_step)
                .collect(),
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RawScratchSelectedCompletionStepSummary {
    pub step_index: usize,
    pub prefix_raw_token_count: usize,
    pub selected_raw_token_id: u32,
    pub finite_admissible_raw_token_count: usize,
    pub selected_raw_token_admitted: bool,
    pub selected_transition_accepted: bool,
    pub transition_state: RawScratchStateSummary,
}

impl RawScratchSelectedCompletionStepSummary {
    fn from_step(step: &crate::syntax_state_machine::RawSyntaxSelectedCompletionStep) -> Self {
        Self {
            step_index: step.step_index,
            prefix_raw_token_count: step.prefix_raw_token_count,
            selected_raw_token_id: step.selected_raw_token_id,
            finite_admissible_raw_token_count: step.finite_admissible_raw_token_count,
            selected_raw_token_admitted: step.selected_raw_token_admitted,
            selected_transition_accepted: step.selected_transition_accepted,
            transition_state: RawScratchStateSummary::from_state(&step.transition_state),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RawScratchStrictIncompletePrefixSummary {
    pub rejected: bool,
    pub error_kind: String,
    pub consumed_raw_tokens: usize,
    pub total_raw_tokens: usize,
    pub failing_token: Option<u32>,
}

impl RawScratchStrictIncompletePrefixSummary {
    pub fn from_error(error: &RawSyntaxError, total_raw_tokens: usize) -> Self {
        Self {
            rejected: true,
            error_kind: error.kind.to_string(),
            consumed_raw_tokens: error.consumed_raw_tokens,
            total_raw_tokens,
            failing_token: error.failing_token,
        }
    }

    pub fn from_parts(
        rejected: bool,
        error_kind: String,
        consumed_raw_tokens: usize,
        total_raw_tokens: usize,
        failing_token: Option<u32>,
    ) -> Self {
        Self {
            rejected,
            error_kind,
            consumed_raw_tokens,
            total_raw_tokens,
            failing_token,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RawScratchTokenOnlyDecodeSummary {
    pub raw_token_count: usize,
    pub raw_token_ids_sha256: String,
    pub root_rule: String,
    pub root_node_kind: String,
    pub node_start_count: usize,
    pub payload_value_count: usize,
}

impl RawScratchTokenOnlyDecodeSummary {
    pub fn from_tree(
        raw_token_count: usize,
        raw_token_ids_sha256: String,
        decoded_tree: &RawSyntaxDecodedTree,
        error_field: &'static str,
    ) -> Result<Self, SyntaxBackendBoundaryError> {
        let root_rule = decoded_tree.root_rule().ok_or_else(|| {
            SyntaxBackendBoundaryError::new(
                error_field,
                "token-only decoded tree must expose a root rule",
            )
        })?;
        let root_node_kind = decoded_tree.root_node_kind().ok_or_else(|| {
            SyntaxBackendBoundaryError::new(
                error_field,
                "token-only decoded tree must expose a root node kind",
            )
        })?;

        Ok(Self {
            raw_token_count,
            raw_token_ids_sha256,
            root_rule: root_rule.to_string(),
            root_node_kind: root_node_kind.to_string(),
            node_start_count: decoded_tree.node_start_count(),
            payload_value_count: decoded_tree.payload_value_count(),
        })
    }
}
