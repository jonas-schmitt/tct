//! Stepwise admissibility proofs for Go scratch-generation seed streams.

use crate::syntax_backend_boundary::SyntaxBackendBoundaryError;
use crate::syntax_state_machine::{
    RawSyntaxAdmissibleTokens, RawSyntaxState, RawSyntaxStateMachineCore, RawSyntaxStatus,
    SyntaxTokenRoles,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GoScratchSeedAdmissibilityProof {
    pub raw_token_count: usize,
    pub admissible_mask_query_count: usize,
    pub finite_admissible_mask_available: bool,
    pub every_selected_raw_token_admitted: bool,
    pub selected_transition_accepted_count: usize,
    pub final_state: RawSyntaxState,
    pub steps: Vec<GoScratchSeedAdmissibilityStep>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GoScratchSeedAdmissibilityStep {
    pub step_index: usize,
    pub prefix_raw_token_count: usize,
    pub selected_raw_token_id: u32,
    pub finite_admissible_raw_token_count: usize,
    pub admissible_raw_token_range_count: usize,
    pub payload_end_admissible: bool,
    pub selected_raw_token_admitted: bool,
    pub selected_transition_accepted: bool,
    pub transition_state: RawSyntaxState,
}

pub fn verify_go_scratch_seed_admissibility(
    core: &(impl RawSyntaxStateMachineCore + ?Sized),
    raw_token_ids: &[u32],
) -> Result<GoScratchSeedAdmissibilityProof, SyntaxBackendBoundaryError> {
    // One incremental walker pass when the core exposes a stream handle
    // (the compiled model does): mask query plus `advance` per step is
    // O(1)-amortized per token instead of two full replays per token. Cores
    // without incremental access keep the replay-shaped probe loop; the
    // replay-test fallback property pins the two paths as observably
    // identical (equal proofs and equal errors).
    let mut walker = core.begin_raw_stream();
    let mut prefix = Vec::with_capacity(raw_token_ids.len());
    let mut steps = Vec::with_capacity(raw_token_ids.len());
    let mut finite_admissible_mask_available = true;
    let mut selected_transition_accepted_count = 0usize;

    for (step_index, selected_raw_token_id) in raw_token_ids.iter().copied().enumerate() {
        let admissible = match walker.as_ref() {
            Some(stream) => stream.admissible_next_raw_tokens(),
            None => {
                core.state_after_raw_prefix(&prefix)
                    .map_err(|error| {
                        SyntaxBackendBoundaryError::new(
                            "go_scratch_seed.admissibility.prefix_state",
                            format!(
                                "prefix before step {step_index} should be accepted by the raw state machine: {}",
                                error.message
                            ),
                        )
                    })?
                    .admissible
            }
        };
        let finite_admissible_raw_token_count = finite_admissible_raw_token_count(&admissible);
        if finite_admissible_raw_token_count == 0 {
            finite_admissible_mask_available = false;
        }
        let selected_raw_token_admitted =
            admissible_contains_raw_token(&admissible, selected_raw_token_id, core.token_roles());
        if !selected_raw_token_admitted {
            return Err(SyntaxBackendBoundaryError::new(
                "go_scratch_seed.admissibility.selected_token",
                format!(
                    "selected raw token {selected_raw_token_id} at step {step_index} is not admitted by the raw state machine"
                ),
            ));
        }

        let transition_state = match walker.as_mut() {
            Some(stream) => {
                stream.advance(selected_raw_token_id).map_err(|error| {
                    SyntaxBackendBoundaryError::new(
                        "go_scratch_seed.admissibility.transition",
                        format!(
                            "selected raw token {selected_raw_token_id} at step {step_index} was admitted but transition failed: {}",
                            error.message
                        ),
                    )
                })?;
                stream.state_snapshot()
            }
            None => {
                let transition =
                    core.candidate_raw_token_transition(&prefix, selected_raw_token_id);
                if !transition.accepted {
                    let diagnostic = transition
                        .error
                        .as_ref()
                        .map(|error| error.message.as_str())
                        .unwrap_or("transition was rejected without a diagnostic");
                    return Err(SyntaxBackendBoundaryError::new(
                        "go_scratch_seed.admissibility.transition",
                        format!(
                            "selected raw token {selected_raw_token_id} at step {step_index} was admitted but transition failed: {diagnostic}"
                        ),
                    ));
                }
                transition.state.ok_or_else(|| {
                    SyntaxBackendBoundaryError::new(
                        "go_scratch_seed.admissibility.transition",
                        format!(
                            "accepted selected raw token {selected_raw_token_id} at step {step_index} did not return a transition state"
                        ),
                    )
                })?
            }
        };
        selected_transition_accepted_count += 1;
        steps.push(GoScratchSeedAdmissibilityStep {
            step_index,
            prefix_raw_token_count: prefix.len(),
            selected_raw_token_id,
            finite_admissible_raw_token_count,
            admissible_raw_token_range_count: admissible.raw_token_ranges.len(),
            payload_end_admissible: admissible.payload_end_admissible,
            selected_raw_token_admitted,
            selected_transition_accepted: true,
            transition_state,
        });
        prefix.push(selected_raw_token_id);
    }

    let final_state = core.strict_decode_complete(raw_token_ids).map_err(|error| {
        SyntaxBackendBoundaryError::new(
            "go_scratch_seed.admissibility.strict_decode_complete",
            format!(
                "admitted scratch seed stream should strictly decode to a complete GoSourceFile: {}",
                error.message
            ),
        )
    })?;
    if final_state.status != RawSyntaxStatus::Complete || !final_state.eof_valid {
        return Err(SyntaxBackendBoundaryError::new(
            "go_scratch_seed.admissibility.final_state",
            "admitted scratch seed stream did not reach an EOF-valid complete state",
        ));
    }

    Ok(GoScratchSeedAdmissibilityProof {
        raw_token_count: raw_token_ids.len(),
        admissible_mask_query_count: raw_token_ids.len(),
        finite_admissible_mask_available,
        every_selected_raw_token_admitted: true,
        selected_transition_accepted_count,
        final_state,
        steps,
    })
}

fn finite_admissible_raw_token_count(admissible: &RawSyntaxAdmissibleTokens) -> usize {
    admissible.finite_raw_token_ids.len()
        + admissible
            .raw_token_ranges
            .iter()
            .map(|range| (range.end_exclusive - range.start) as usize)
            .sum::<usize>()
}

fn admissible_contains_raw_token(
    admissible: &RawSyntaxAdmissibleTokens,
    token_id: u32,
    roles: &SyntaxTokenRoles,
) -> bool {
    admissible.finite_raw_token_ids.contains(&token_id)
        || admissible
            .raw_token_ranges
            .iter()
            .any(|range| token_id >= range.start && token_id < range.end_exclusive)
        || (admissible.payload_end_admissible && token_id == roles.payload_end)
}
