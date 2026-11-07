//! Random admissible-walk validity gate for the unified Go syntax artifact.
//!
//! The sampler starts from the compiled unified Go grammar artifact and the
//! empty `GoSourceFile` elided-stream prefix. It samples only tokens admitted
//! by the raw state machine, expands mask-forced tokens through the elided
//! walker, strict-decodes complete walks, renders Go from the decoded raw tree,
//! and runs `gofmt` plus `go/parser` as sidecar validation.

use crate::go_ast_schema_inventory::GO_AST_SCHEMA_INVENTORY_FIXTURE_PATH;
use crate::provenance::{json_sha256, sha256_hex};
use crate::syntax_backend_boundary::SyntaxBackendBoundaryError;
use crate::syntax_go_raw_tree_renderer::render_go_source_from_raw_tree;
use crate::syntax_go_unified_artifact::{
    GO_UNIFIED_GRAMMAR_ARTIFACT_ID, GO_UNIFIED_GRAMMAR_ID, go_unified_grammar_artifact,
};
use crate::syntax_scratch_generation::{
    RAW_SCRATCH_GENERATOR_CORE_API_BOUNDARY, RAW_SCRATCH_GENERATOR_CORE_API_ID,
};
use crate::syntax_state_machine::{
    RawSyntaxAdmissibleTokens, RawSyntaxError, RawSyntaxState, RawSyntaxStateMachineCore,
    SyntaxTokenRoles,
};
use crate::syntax_token_layout_diff::LIVE_RAW_SYNTAX_STREAM_CONTRACT;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use std::fs;
use std::io::Write;
use std::path::PathBuf;
use std::process::{Command as ProcessCommand, Stdio};

pub const ARTIFACT_KIND: &str = "tct_go_random_admissible_walk_validity_report_v1";
pub const VERSION: u32 = 1;
pub const TOOL: &str = "syntax_go_random_admissible_walk";
pub const TARGET: &str = "go";
pub const ROUTE: &str = "unified_go_artifact_empty_prefix_random_admissible_walk";
pub const STATUS_PASSED: &str = "go_random_admissible_walk_validity_passed";
pub const STATUS_BLOCKED: &str = "go_random_admissible_walk_validity_blocked";
pub const DEFAULT_OUTPUT_PATH: &str =
    "target/tct-syntax-go-random-admissible-walks/random-admissible-walk-report-v1.json";
pub const DEFAULT_WALK_COUNT: usize = 16;
pub const DEFAULT_MAX_ELIDED_STEPS: usize = 512;
pub const DEFAULT_SEED: u64 = 0x5443_542d_474f_4d33;
pub const NEXT_READY_TASK: &str = "start the BPE-over-raw-tokens compression parity gate after preserving the random-admissible-walk report";
pub const BLOCKED_NEXT_READY_TASK: &str =
    "fix the first unified Go random-admissible-walk blocker named by first_blocker";
pub const CLAIM_BOUNDARY: &str = "syntax-level Go random admissible-walk validity gate: generation starts from the unified pinned Go syntax tokenizer artifact and an empty GoSourceFile elided-stream prefix, samples only raw tokens admitted by the raw syntax state machine, decodes token-only streams into reusable raw syntax trees, renders Go, and checks gofmt plus go/parser as sidecar validation. This is not a total Go language claim, not per-output parser metadata, not source-instance segment sidecar authority, not source-prefix table evidence, not Arena IR or GoIR evidence, not BPE validity authority, not type correctness, and not stable-backend promotion.";
pub const NO_STABLE_CLAIMS: &[&str] = &[
    "no_total_go_language_claim",
    "no_per_output_parser_metadata",
    "no_source_instance_segment_sidecar",
    "no_source_prefix_table",
    "no_arena_ir_claim",
    "no_go_ir_claim",
    "no_bpe_layer_validity_claim",
    "no_type_correctness_claim",
    "no_stable_backend_claim",
];

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GoRandomAdmissibleWalkConfig {
    pub seed: u64,
    pub walk_count: usize,
    pub max_elided_steps: usize,
    pub output_path: String,
}

impl Default for GoRandomAdmissibleWalkConfig {
    fn default() -> Self {
        Self {
            seed: DEFAULT_SEED,
            walk_count: DEFAULT_WALK_COUNT,
            max_elided_steps: DEFAULT_MAX_ELIDED_STEPS,
            output_path: DEFAULT_OUTPUT_PATH.to_string(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GoRandomAdmissibleWalkReport {
    pub artifact_kind: String,
    pub version: u32,
    pub tool: String,
    pub target: String,
    pub route: String,
    pub status: String,
    pub grammar_artifact_id: String,
    pub grammar_artifact_content_sha256: String,
    pub candidate_grammar_id: String,
    pub metadata_path: String,
    pub structural_model_id: String,
    pub stream_contract: String,
    pub raw_state_machine_core_boundary_id: String,
    pub raw_state_machine_core_boundary: String,
    pub generator_core_api_id: String,
    pub generator_core_api_boundary: String,
    pub seed: u64,
    pub walk_count: usize,
    pub max_elided_steps: usize,
    pub completed_walk_count: usize,
    pub decoded_walk_count: usize,
    pub rendered_walk_count: usize,
    pub gofmt_passed_count: usize,
    pub go_parser_passed_count: usize,
    pub validity_passed_count: usize,
    pub first_blocker: Option<GoRandomAdmissibleWalkBlocker>,
    pub walks: Vec<GoRandomAdmissibleWalkRecord>,
    pub next_ready_task: String,
    pub validation_commands: Vec<String>,
    pub claim_boundary: String,
    pub no_stable_claims: Vec<String>,
}

impl GoRandomAdmissibleWalkReport {
    pub fn validate(&self) -> Result<(), SyntaxBackendBoundaryError> {
        require_string("artifact_kind", &self.artifact_kind, ARTIFACT_KIND)?;
        require_u32("version", self.version, VERSION)?;
        require_string("tool", &self.tool, TOOL)?;
        require_string("target", &self.target, TARGET)?;
        require_string("route", &self.route, ROUTE)?;
        require_one_of("status", &self.status, &[STATUS_PASSED, STATUS_BLOCKED])?;
        let artifact = go_unified_grammar_artifact()?;
        require_string(
            "grammar_artifact_id",
            &self.grammar_artifact_id,
            artifact.artifact_id,
        )?;
        require_string(
            "grammar_artifact_content_sha256",
            &self.grammar_artifact_content_sha256,
            &artifact.content_sha256,
        )?;
        require_string(
            "candidate_grammar_id",
            &self.candidate_grammar_id,
            GO_UNIFIED_GRAMMAR_ID,
        )?;
        require_string(
            "metadata_path",
            &self.metadata_path,
            GO_AST_SCHEMA_INVENTORY_FIXTURE_PATH,
        )?;
        require_string(
            "structural_model_id",
            &self.structural_model_id,
            &artifact.sidecar.structural_model.model_id,
        )?;
        require_string(
            "stream_contract",
            &self.stream_contract,
            LIVE_RAW_SYNTAX_STREAM_CONTRACT,
        )?;
        require_string(
            "raw_state_machine_core_boundary_id",
            &self.raw_state_machine_core_boundary_id,
            artifact.compiled_model.boundary_id(),
        )?;
        require_string(
            "raw_state_machine_core_boundary",
            &self.raw_state_machine_core_boundary,
            artifact.compiled_model.boundary_contract(),
        )?;
        require_string(
            "generator_core_api_id",
            &self.generator_core_api_id,
            RAW_SCRATCH_GENERATOR_CORE_API_ID,
        )?;
        require_string(
            "generator_core_api_boundary",
            &self.generator_core_api_boundary,
            RAW_SCRATCH_GENERATOR_CORE_API_BOUNDARY,
        )?;
        require_nonzero("walk_count", self.walk_count)?;
        require_nonzero("max_elided_steps", self.max_elided_steps)?;
        require_usize("walks.len", self.walks.len(), self.walk_count)?;
        let completed = self
            .walks
            .iter()
            .filter(|walk| walk.completed && walk.final_state_eof_valid)
            .count();
        let decoded = self.walks.iter().filter(|walk| walk.decoded).count();
        let rendered = self.walks.iter().filter(|walk| walk.rendered).count();
        let gofmt = self
            .walks
            .iter()
            .filter(|walk| walk.gofmt_validation == "passed")
            .count();
        let go_parser = self
            .walks
            .iter()
            .filter(|walk| walk.go_parser_validation == "passed")
            .count();
        let validity = self
            .walks
            .iter()
            .filter(|walk| walk.status == "passed")
            .count();
        require_usize("completed_walk_count", self.completed_walk_count, completed)?;
        require_usize("decoded_walk_count", self.decoded_walk_count, decoded)?;
        require_usize("rendered_walk_count", self.rendered_walk_count, rendered)?;
        require_usize("gofmt_passed_count", self.gofmt_passed_count, gofmt)?;
        require_usize(
            "go_parser_passed_count",
            self.go_parser_passed_count,
            go_parser,
        )?;
        require_usize(
            "validity_passed_count",
            self.validity_passed_count,
            validity,
        )?;
        let expected_status = if self.validity_passed_count == self.walk_count {
            STATUS_PASSED
        } else {
            STATUS_BLOCKED
        };
        require_string("status", &self.status, expected_status)?;
        match (self.status.as_str(), &self.first_blocker) {
            (STATUS_PASSED, None) => {}
            (STATUS_PASSED, Some(_)) => {
                return Err(SyntaxBackendBoundaryError::new(
                    "first_blocker",
                    "passed random-admissible-walk reports must not carry a first blocker",
                ));
            }
            (STATUS_BLOCKED, Some(blocker)) => blocker.validate()?,
            (STATUS_BLOCKED, None) => {
                return Err(SyntaxBackendBoundaryError::new(
                    "first_blocker",
                    "blocked random-admissible-walk reports must carry a first blocker",
                ));
            }
            _ => unreachable!("status is checked by require_one_of"),
        }
        let mut walk_ids = BTreeSet::new();
        for walk in &self.walks {
            walk.validate(self.max_elided_steps)?;
            if !walk_ids.insert(walk.walk_id.clone()) {
                return Err(SyntaxBackendBoundaryError::new(
                    "walks.walk_id",
                    format!("duplicate walk id {}", walk.walk_id),
                ));
            }
        }
        require_string(
            "next_ready_task",
            &self.next_ready_task,
            if self.status == STATUS_PASSED {
                NEXT_READY_TASK
            } else {
                BLOCKED_NEXT_READY_TASK
            },
        )?;
        require_contains(
            "validation_commands",
            &self.validation_commands,
            "tct-syntax-go-tokenizer-generator",
        )?;
        require_contains(
            "validation_commands",
            &self.validation_commands,
            "--random-admissible-walks",
        )?;
        require_str_contains("claim_boundary", &self.claim_boundary, "syntax-level Go")?;
        require_str_contains("claim_boundary", &self.claim_boundary, "empty GoSourceFile")?;
        require_str_contains("claim_boundary", &self.claim_boundary, "not Arena IR")?;
        require_str_contains("claim_boundary", &self.claim_boundary, "not BPE")?;
        for claim in NO_STABLE_CLAIMS {
            require_contains("no_stable_claims", &self.no_stable_claims, claim)?;
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GoRandomAdmissibleWalkRecord {
    pub walk_id: String,
    pub seed: u64,
    pub status: String,
    pub completed: bool,
    pub decoded: bool,
    pub rendered: bool,
    pub gofmt_validation: String,
    pub go_parser_validation: String,
    pub elided_token_count: usize,
    pub expanded_token_count: usize,
    pub forced_token_count: usize,
    pub elided_tokens_sha256: String,
    pub expanded_tokens_sha256: String,
    pub final_state_observed_status: String,
    pub final_state_consumed_raw_tokens: usize,
    pub final_state_eof_valid: bool,
    pub final_state_stack_depth: usize,
    pub decoded_tree_event_count: usize,
    pub decoded_tree_node_start_count: usize,
    pub rendered_source_sha256: Option<String>,
    pub rendered_source_byte_count: Option<usize>,
    pub formatted_source_sha256: Option<String>,
    pub formatted_source_byte_count: Option<usize>,
    pub blocker: Option<GoRandomAdmissibleWalkBlocker>,
    pub step_samples: Vec<GoRandomAdmissibleWalkStepSample>,
}

impl GoRandomAdmissibleWalkRecord {
    fn validate(&self, max_elided_steps: usize) -> Result<(), SyntaxBackendBoundaryError> {
        require_non_empty("walk.walk_id", &self.walk_id)?;
        require_one_of("walk.status", &self.status, &["passed", "blocked"])?;
        if self.elided_token_count > max_elided_steps {
            return Err(SyntaxBackendBoundaryError::new(
                "walk.elided_token_count",
                "walk exceeded the configured elided-step bound",
            ));
        }
        if self.expanded_token_count < self.elided_token_count {
            return Err(SyntaxBackendBoundaryError::new(
                "walk.expanded_token_count",
                "expanded token count cannot be smaller than elided token count",
            ));
        }
        require_usize(
            "walk.forced_token_count",
            self.forced_token_count,
            self.expanded_token_count - self.elided_token_count,
        )?;
        require_sha256("walk.elided_tokens_sha256", &self.elided_tokens_sha256)?;
        require_sha256("walk.expanded_tokens_sha256", &self.expanded_tokens_sha256)?;
        if self.status == "passed" {
            require_bool("walk.completed", self.completed, true)?;
            require_bool("walk.decoded", self.decoded, true)?;
            require_bool("walk.rendered", self.rendered, true)?;
            require_string("walk.gofmt_validation", &self.gofmt_validation, "passed")?;
            require_string(
                "walk.go_parser_validation",
                &self.go_parser_validation,
                "passed",
            )?;
            require_string(
                "walk.final_state_observed_status",
                &self.final_state_observed_status,
                "complete",
            )?;
            require_bool(
                "walk.final_state_eof_valid",
                self.final_state_eof_valid,
                true,
            )?;
            require_usize(
                "walk.final_state_stack_depth",
                self.final_state_stack_depth,
                0,
            )?;
            require_nonzero(
                "walk.decoded_tree_event_count",
                self.decoded_tree_event_count,
            )?;
            require_nonzero(
                "walk.decoded_tree_node_start_count",
                self.decoded_tree_node_start_count,
            )?;
            require_some_sha256(
                "walk.rendered_source_sha256",
                self.rendered_source_sha256.as_deref(),
            )?;
            require_some_sha256(
                "walk.formatted_source_sha256",
                self.formatted_source_sha256.as_deref(),
            )?;
            require_none("walk.blocker", &self.blocker)?;
        } else if self.blocker.is_none() {
            return Err(SyntaxBackendBoundaryError::new(
                "walk.blocker",
                "blocked walks must carry a blocker",
            ));
        }
        for sample in &self.step_samples {
            sample.validate()?;
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GoRandomAdmissibleWalkStepSample {
    pub step_index: usize,
    pub expanded_prefix_token_count: usize,
    pub selected_elided_raw_token_id: u32,
    pub finite_admissible_raw_token_count: usize,
    pub admissible_raw_token_range_count: usize,
    pub payload_end_admissible: bool,
    pub forced_token_count_after: usize,
    pub transition_observed_status: String,
    pub transition_eof_valid: bool,
    pub transition_stack_depth: usize,
    pub transition_active_rule: Option<String>,
    pub transition_active_field: Option<String>,
}

impl GoRandomAdmissibleWalkStepSample {
    fn validate(&self) -> Result<(), SyntaxBackendBoundaryError> {
        require_nonzero(
            "walk.step.finite_admissible_raw_token_count",
            self.finite_admissible_raw_token_count,
        )?;
        require_one_of(
            "walk.step.transition_observed_status",
            &self.transition_observed_status,
            &["partial", "complete"],
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GoRandomAdmissibleWalkBlocker {
    pub walk_id: String,
    pub phase: String,
    pub reason: String,
    pub elided_token_count: usize,
    pub expanded_token_count: usize,
    pub active_rule: Option<String>,
    pub active_field: Option<String>,
    pub next_ready_task: String,
}

impl GoRandomAdmissibleWalkBlocker {
    fn validate(&self) -> Result<(), SyntaxBackendBoundaryError> {
        require_non_empty("blocker.walk_id", &self.walk_id)?;
        require_non_empty("blocker.phase", &self.phase)?;
        require_non_empty("blocker.reason", &self.reason)?;
        require_string(
            "blocker.next_ready_task",
            &self.next_ready_task,
            BLOCKED_NEXT_READY_TASK,
        )
    }
}

pub fn build_go_random_admissible_walk_report(
    config: &GoRandomAdmissibleWalkConfig,
    go_bin: &str,
) -> Result<GoRandomAdmissibleWalkReport, SyntaxBackendBoundaryError> {
    if config.walk_count == 0 {
        return Err(SyntaxBackendBoundaryError::new(
            "walk_count",
            "random-admissible-walk report requires at least one walk",
        ));
    }
    if config.max_elided_steps == 0 {
        return Err(SyntaxBackendBoundaryError::new(
            "max_elided_steps",
            "random-admissible-walk report requires a positive elided-step bound",
        ));
    }

    let artifact = go_unified_grammar_artifact()?;
    let walks = (0..config.walk_count)
        .map(|walk_index| sample_walk(walk_index, config, go_bin))
        .collect::<Result<Vec<_>, _>>()?;
    let completed_walk_count = walks
        .iter()
        .filter(|walk| walk.completed && walk.final_state_eof_valid)
        .count();
    let decoded_walk_count = walks.iter().filter(|walk| walk.decoded).count();
    let rendered_walk_count = walks.iter().filter(|walk| walk.rendered).count();
    let gofmt_passed_count = walks
        .iter()
        .filter(|walk| walk.gofmt_validation == "passed")
        .count();
    let go_parser_passed_count = walks
        .iter()
        .filter(|walk| walk.go_parser_validation == "passed")
        .count();
    let validity_passed_count = walks.iter().filter(|walk| walk.status == "passed").count();
    let first_blocker = walks.iter().find_map(|walk| walk.blocker.clone());
    let status = if validity_passed_count == config.walk_count {
        STATUS_PASSED
    } else {
        STATUS_BLOCKED
    };
    let report = GoRandomAdmissibleWalkReport {
        artifact_kind: ARTIFACT_KIND.to_string(),
        version: VERSION,
        tool: TOOL.to_string(),
        target: TARGET.to_string(),
        route: ROUTE.to_string(),
        status: status.to_string(),
        grammar_artifact_id: GO_UNIFIED_GRAMMAR_ARTIFACT_ID.to_string(),
        grammar_artifact_content_sha256: artifact.content_sha256.clone(),
        candidate_grammar_id: GO_UNIFIED_GRAMMAR_ID.to_string(),
        metadata_path: GO_AST_SCHEMA_INVENTORY_FIXTURE_PATH.to_string(),
        structural_model_id: artifact.sidecar.structural_model.model_id.clone(),
        stream_contract: LIVE_RAW_SYNTAX_STREAM_CONTRACT.to_string(),
        raw_state_machine_core_boundary_id: artifact.compiled_model.boundary_id().to_string(),
        raw_state_machine_core_boundary: artifact.compiled_model.boundary_contract().to_string(),
        generator_core_api_id: RAW_SCRATCH_GENERATOR_CORE_API_ID.to_string(),
        generator_core_api_boundary: RAW_SCRATCH_GENERATOR_CORE_API_BOUNDARY.to_string(),
        seed: config.seed,
        walk_count: config.walk_count,
        max_elided_steps: config.max_elided_steps,
        completed_walk_count,
        decoded_walk_count,
        rendered_walk_count,
        gofmt_passed_count,
        go_parser_passed_count,
        validity_passed_count,
        first_blocker,
        walks,
        next_ready_task: if status == STATUS_PASSED {
            NEXT_READY_TASK.to_string()
        } else {
            BLOCKED_NEXT_READY_TASK.to_string()
        },
        validation_commands: validation_commands(config),
        claim_boundary: CLAIM_BOUNDARY.to_string(),
        no_stable_claims: NO_STABLE_CLAIMS
            .iter()
            .map(|claim| (*claim).to_string())
            .collect(),
    };
    report.validate()?;
    Ok(report)
}

pub fn parse_go_random_admissible_walk_report(
    contents: &str,
) -> Result<GoRandomAdmissibleWalkReport, SyntaxBackendBoundaryError> {
    let report: GoRandomAdmissibleWalkReport = serde_json::from_str(contents).map_err(|error| {
        SyntaxBackendBoundaryError::new(
            "random_admissible_walk_report",
            format!("failed to parse random-admissible-walk report: {error}"),
        )
    })?;
    report.validate()?;
    Ok(report)
}

fn sample_walk(
    walk_index: usize,
    config: &GoRandomAdmissibleWalkConfig,
    go_bin: &str,
) -> Result<GoRandomAdmissibleWalkRecord, SyntaxBackendBoundaryError> {
    let artifact = go_unified_grammar_artifact()?;
    let roles = artifact.compiled_model.token_roles();
    let walk_seed =
        splitmix64(config.seed ^ (walk_index as u64 + 1).wrapping_mul(0x9e37_79b9_7f4a_7c15));
    let walk_id = format!("go-random-admissible-walk-{walk_index:04}");
    let mut rng = DeterministicRng::new(walk_seed);
    let mut walker = artifact
        .compiled_model
        .begin_elided_stream()
        .map_err(|error| {
            SyntaxBackendBoundaryError::new(
                "random_admissible_walk.begin_elided_stream",
                format!(
                    "failed to construct empty GoSourceFile elided stream: {}",
                    error.message
                ),
            )
        })?;
    let mut elided_tokens = Vec::<u32>::new();
    let mut expanded_tokens = walker
        .initial_forced_run()
        .iter()
        .map(|forced| forced.raw_token_id)
        .collect::<Vec<_>>();
    let mut step_samples = Vec::new();
    let mut payload_plan = PayloadPlan::default();

    while !walker.eof_valid() && elided_tokens.len() < config.max_elided_steps {
        let state = walker.state_snapshot();
        let admissible = walker.admissible_next_raw_tokens();
        let selected =
            select_admissible_token(&mut rng, &state, &admissible, roles, &mut payload_plan)
                .ok_or_else(|| {
                    SyntaxBackendBoundaryError::new(
                        "random_admissible_walk.selection",
                        format!(
                            "{} reached a non-EOF state with no selectable admissible raw token",
                            walk_id
                        ),
                    )
                })?;
        let expanded_prefix_token_count = walker.expanded_raw_token_count();
        let finite_admissible_raw_token_count =
            finite_admissible_raw_token_count(&admissible, roles);
        let advance = match walker.advance(selected) {
            Ok(advance) => advance,
            Err(error) => {
                let blocker = blocker_from_error(
                    &walk_id,
                    "transition",
                    &error,
                    elided_tokens.len(),
                    expanded_tokens.len(),
                );
                elided_tokens.push(selected);
                return blocked_walk(
                    walk_id,
                    walk_seed,
                    elided_tokens,
                    expanded_tokens,
                    walker.state_snapshot(),
                    Some(blocker),
                    step_samples,
                );
            }
        };
        elided_tokens.push(selected);
        expanded_tokens.push(selected);
        expanded_tokens.extend(
            advance
                .forced_tokens_applied
                .iter()
                .map(|forced| forced.raw_token_id),
        );
        if should_sample_step(elided_tokens.len() - 1, config.max_elided_steps) {
            let transition = walker.state_snapshot();
            step_samples.push(GoRandomAdmissibleWalkStepSample {
                step_index: elided_tokens.len() - 1,
                expanded_prefix_token_count,
                selected_elided_raw_token_id: selected,
                finite_admissible_raw_token_count,
                admissible_raw_token_range_count: admissible.raw_token_ranges.len(),
                payload_end_admissible: admissible.payload_end_admissible,
                forced_token_count_after: advance.forced_tokens_applied.len(),
                transition_observed_status: transition.status.as_str().to_string(),
                transition_eof_valid: transition.eof_valid,
                transition_stack_depth: transition.stack_depth,
                transition_active_rule: transition.active_rule,
                transition_active_field: transition.active_field,
            });
        }
    }

    if !walker.eof_valid() {
        let state = walker.state_snapshot();
        let blocker = GoRandomAdmissibleWalkBlocker {
            walk_id: walk_id.clone(),
            phase: "completion".to_string(),
            reason: format!(
                "walk hit the max_elided_steps bound {} before EOF validity",
                config.max_elided_steps
            ),
            elided_token_count: elided_tokens.len(),
            expanded_token_count: expanded_tokens.len(),
            active_rule: state.active_rule.clone(),
            active_field: state.active_field.clone(),
            next_ready_task: BLOCKED_NEXT_READY_TASK.to_string(),
        };
        return blocked_walk(
            walk_id,
            walk_seed,
            elided_tokens,
            expanded_tokens,
            state,
            Some(blocker),
            step_samples,
        );
    }

    let decoded = match artifact
        .compiled_model
        .decode_complete_tree_elided(&elided_tokens)
    {
        Ok(decoded) => decoded,
        Err(error) => {
            let blocker = blocker_from_error(
                &walk_id,
                "decode",
                &error,
                elided_tokens.len(),
                expanded_tokens.len(),
            );
            return blocked_walk(
                walk_id,
                walk_seed,
                elided_tokens,
                expanded_tokens,
                walker.state_snapshot(),
                Some(blocker),
                step_samples,
            );
        }
    };
    let rendered = match render_go_source_from_raw_tree(&decoded) {
        Ok(rendered) => rendered,
        Err(error) => {
            let blocker = GoRandomAdmissibleWalkBlocker {
                walk_id: walk_id.clone(),
                phase: "render".to_string(),
                reason: error.to_string(),
                elided_token_count: elided_tokens.len(),
                expanded_token_count: expanded_tokens.len(),
                active_rule: decoded.state.active_rule.clone(),
                active_field: decoded.state.active_field.clone(),
                next_ready_task: BLOCKED_NEXT_READY_TASK.to_string(),
            };
            return blocked_walk(
                walk_id,
                walk_seed,
                elided_tokens,
                expanded_tokens,
                decoded.state,
                Some(blocker),
                step_samples,
            );
        }
    };
    let formatted = match gofmt_source(&rendered.source) {
        Ok(formatted) => formatted,
        Err(error) => {
            let blocker = GoRandomAdmissibleWalkBlocker {
                walk_id: walk_id.clone(),
                phase: "gofmt".to_string(),
                reason: error.to_string(),
                elided_token_count: elided_tokens.len(),
                expanded_token_count: expanded_tokens.len(),
                active_rule: decoded.state.active_rule.clone(),
                active_field: decoded.state.active_field.clone(),
                next_ready_task: BLOCKED_NEXT_READY_TASK.to_string(),
            };
            return blocked_walk(
                walk_id,
                walk_seed,
                elided_tokens,
                expanded_tokens,
                decoded.state,
                Some(blocker),
                step_samples,
            );
        }
    };
    if let Err(error) = go_parser_validate(go_bin, &formatted, walk_index, walk_seed) {
        let blocker = GoRandomAdmissibleWalkBlocker {
            walk_id: walk_id.clone(),
            phase: "go_parser".to_string(),
            reason: error.to_string(),
            elided_token_count: elided_tokens.len(),
            expanded_token_count: expanded_tokens.len(),
            active_rule: decoded.state.active_rule.clone(),
            active_field: decoded.state.active_field.clone(),
            next_ready_task: BLOCKED_NEXT_READY_TASK.to_string(),
        };
        return blocked_walk(
            walk_id,
            walk_seed,
            elided_tokens,
            expanded_tokens,
            decoded.state,
            Some(blocker),
            step_samples,
        );
    }

    let record = GoRandomAdmissibleWalkRecord {
        walk_id,
        seed: walk_seed,
        status: "passed".to_string(),
        completed: true,
        decoded: true,
        rendered: true,
        gofmt_validation: "passed".to_string(),
        go_parser_validation: "passed".to_string(),
        elided_token_count: elided_tokens.len(),
        expanded_token_count: expanded_tokens.len(),
        forced_token_count: expanded_tokens.len() - elided_tokens.len(),
        elided_tokens_sha256: json_sha256(&elided_tokens).map_err(|error| {
            SyntaxBackendBoundaryError::new(
                "random_admissible_walk.elided_tokens_sha256",
                format!("failed to hash elided tokens: {error}"),
            )
        })?,
        expanded_tokens_sha256: json_sha256(&expanded_tokens).map_err(|error| {
            SyntaxBackendBoundaryError::new(
                "random_admissible_walk.expanded_tokens_sha256",
                format!("failed to hash expanded tokens: {error}"),
            )
        })?,
        final_state_observed_status: decoded.state.status.as_str().to_string(),
        final_state_consumed_raw_tokens: decoded.state.consumed_raw_tokens,
        final_state_eof_valid: decoded.state.eof_valid,
        final_state_stack_depth: decoded.state.stack_depth,
        decoded_tree_event_count: decoded.events.len(),
        decoded_tree_node_start_count: decoded.node_start_count(),
        rendered_source_sha256: Some(sha256_hex(rendered.source.as_bytes())),
        rendered_source_byte_count: Some(rendered.source.len()),
        formatted_source_sha256: Some(sha256_hex(formatted.as_bytes())),
        formatted_source_byte_count: Some(formatted.len()),
        blocker: None,
        step_samples,
    };
    record.validate(config.max_elided_steps)?;
    Ok(record)
}

fn blocked_walk(
    walk_id: String,
    walk_seed: u64,
    elided_tokens: Vec<u32>,
    expanded_tokens: Vec<u32>,
    state: RawSyntaxState,
    blocker: Option<GoRandomAdmissibleWalkBlocker>,
    step_samples: Vec<GoRandomAdmissibleWalkStepSample>,
) -> Result<GoRandomAdmissibleWalkRecord, SyntaxBackendBoundaryError> {
    let record = GoRandomAdmissibleWalkRecord {
        walk_id,
        seed: walk_seed,
        status: "blocked".to_string(),
        completed: state.eof_valid,
        decoded: false,
        rendered: false,
        gofmt_validation: "not_run".to_string(),
        go_parser_validation: "not_run".to_string(),
        elided_token_count: elided_tokens.len(),
        expanded_token_count: expanded_tokens.len(),
        forced_token_count: expanded_tokens.len().saturating_sub(elided_tokens.len()),
        elided_tokens_sha256: json_sha256(&elided_tokens).map_err(|error| {
            SyntaxBackendBoundaryError::new(
                "random_admissible_walk.elided_tokens_sha256",
                format!("failed to hash blocked elided tokens: {error}"),
            )
        })?,
        expanded_tokens_sha256: json_sha256(&expanded_tokens).map_err(|error| {
            SyntaxBackendBoundaryError::new(
                "random_admissible_walk.expanded_tokens_sha256",
                format!("failed to hash blocked expanded tokens: {error}"),
            )
        })?,
        final_state_observed_status: state.status.as_str().to_string(),
        final_state_consumed_raw_tokens: state.consumed_raw_tokens,
        final_state_eof_valid: state.eof_valid,
        final_state_stack_depth: state.stack_depth,
        decoded_tree_event_count: 0,
        decoded_tree_node_start_count: 0,
        rendered_source_sha256: None,
        rendered_source_byte_count: None,
        formatted_source_sha256: None,
        formatted_source_byte_count: None,
        blocker,
        step_samples,
    };
    record.validate(usize::MAX)?;
    Ok(record)
}

fn blocker_from_error(
    walk_id: &str,
    phase: &str,
    error: &RawSyntaxError,
    elided_token_count: usize,
    expanded_token_count: usize,
) -> GoRandomAdmissibleWalkBlocker {
    GoRandomAdmissibleWalkBlocker {
        walk_id: walk_id.to_string(),
        phase: phase.to_string(),
        reason: error.message.clone(),
        elided_token_count,
        expanded_token_count,
        active_rule: error.active_rule.clone(),
        active_field: error.active_field.clone(),
        next_ready_task: BLOCKED_NEXT_READY_TASK.to_string(),
    }
}

#[derive(Debug, Default, Clone, PartialEq, Eq)]
struct PayloadPlan {
    active_key: Option<String>,
    bytes: Vec<u8>,
}

fn select_admissible_token(
    rng: &mut DeterministicRng,
    state: &RawSyntaxState,
    admissible: &RawSyntaxAdmissibleTokens,
    roles: &SyntaxTokenRoles,
    payload_plan: &mut PayloadPlan,
) -> Option<u32> {
    if state.active_payload_kind.is_some() {
        return select_payload_token(rng, state, admissible, roles, payload_plan);
    }
    payload_plan.active_key = None;
    payload_plan.bytes.clear();

    let mut candidates = admissible.finite_raw_token_ids.clone();
    candidates.sort_unstable();
    candidates.dedup();
    let source_required_field = matches!(
        (state.active_rule.as_deref(), state.active_field.as_deref()),
        (Some("File"), Some("Name" | "GoVersion"))
    );
    if source_required_field {
        candidates.retain(|candidate| *candidate != roles.field_missing);
    }
    if candidates.is_empty() {
        return None;
    }

    if !source_required_field
        && candidates.contains(&roles.field_missing)
        && rng.next_bounded(100) < 82
    {
        return Some(roles.field_missing);
    }
    if candidates.contains(&roles.repeated_empty) && rng.next_bounded(100) < 78 {
        return Some(roles.repeated_empty);
    }
    if candidates.contains(&roles.field_end) && rng.next_bounded(100) < 58 {
        return Some(roles.field_end);
    }
    if candidates.contains(&roles.node_end) && rng.next_bounded(100) < 70 {
        return Some(roles.node_end);
    }
    let index = rng.next_bounded(candidates.len() as u64) as usize;
    candidates.get(index).copied()
}

fn select_payload_token(
    rng: &mut DeterministicRng,
    state: &RawSyntaxState,
    admissible: &RawSyntaxAdmissibleTokens,
    roles: &SyntaxTokenRoles,
    payload_plan: &mut PayloadPlan,
) -> Option<u32> {
    let key = format!(
        "{}:{}:{}",
        state.active_rule.as_deref().unwrap_or(""),
        state.active_field.as_deref().unwrap_or(""),
        state.active_payload_kind.as_deref().unwrap_or("")
    );
    if payload_plan.active_key.as_deref() != Some(key.as_str())
        || state.active_payload_byte_count == 0
    {
        payload_plan.active_key = Some(key);
        payload_plan.bytes = payload_value_for_state(rng, state).into_bytes();
    }
    if state.active_payload_byte_count < payload_plan.bytes.len() {
        let byte = payload_plan.bytes[state.active_payload_byte_count];
        if let Some(token) = payload_byte_token(byte, state, admissible, roles) {
            return Some(token);
        }
    }
    if admissible.payload_end_admissible {
        return Some(roles.payload_end);
    }
    first_range_token(admissible).or_else(|| admissible.finite_raw_token_ids.first().copied())
}

fn payload_value_for_state(rng: &mut DeterministicRng, state: &RawSyntaxState) -> String {
    let field = state.active_field.as_deref().unwrap_or("");
    let rule = state.active_rule.as_deref().unwrap_or("");
    let kind = state.active_payload_kind.as_deref().unwrap_or("");
    match field {
        "Name" if rule == "File" => "p".to_string(),
        "GoVersion" => "go1.24".to_string(),
        "Kind" => "INT".to_string(),
        "Value" => "0".to_string(),
        "Path" => "\"fmt\"".to_string(),
        "Text" => "// x".to_string(),
        "Dir" => "SEND_RECV".to_string(),
        "Tok" if rule == "GenDecl" => "var".to_string(),
        "Tok" if rule == "AssignStmt" => "=".to_string(),
        "Tok" if rule == "BranchStmt" => "return".to_string(),
        "Op" => "+".to_string(),
        _ if kind.contains("integer") => (rng.next_bounded(9)).to_string(),
        _ if kind.contains("identifier") => {
            const IDENTIFIERS: &[&str] = &["x", "y", "f", "v", "n", "xs", "int", "nil"];
            IDENTIFIERS[rng.next_bounded(IDENTIFIERS.len() as u64) as usize].to_string()
        }
        _ => "x".to_string(),
    }
}

fn payload_byte_token(
    byte: u8,
    state: &RawSyntaxState,
    admissible: &RawSyntaxAdmissibleTokens,
    roles: &SyntaxTokenRoles,
) -> Option<u32> {
    let preferred_role = if state
        .active_payload_kind
        .as_deref()
        .unwrap_or("")
        .contains("integer")
    {
        roles.integer_range.role.as_str()
    } else {
        roles.utf8_range.role.as_str()
    };
    admissible
        .raw_token_ranges
        .iter()
        .find(|range| range.role == preferred_role)
        .and_then(|range| {
            let token = range.start + u32::from(byte);
            if token < range.end_exclusive {
                Some(token)
            } else {
                None
            }
        })
        .or_else(|| first_range_token(admissible))
}

fn first_range_token(admissible: &RawSyntaxAdmissibleTokens) -> Option<u32> {
    admissible.raw_token_ranges.first().map(|range| range.start)
}

fn finite_admissible_raw_token_count(
    admissible: &RawSyntaxAdmissibleTokens,
    roles: &SyntaxTokenRoles,
) -> usize {
    admissible.finite_raw_token_ids.len()
        + admissible
            .raw_token_ranges
            .iter()
            .map(|range| (range.end_exclusive - range.start) as usize)
            .sum::<usize>()
        + usize::from(admissible.payload_end_admissible && roles.payload_end < roles.vocab_size)
}

fn should_sample_step(step_index: usize, max_elided_steps: usize) -> bool {
    step_index < 3
        || step_index + 3 >= max_elided_steps
        || step_index.is_multiple_of((max_elided_steps / 4).max(1))
}

fn gofmt_source(source: &str) -> Result<String, SyntaxBackendBoundaryError> {
    let gofmt_bin = std::env::var_os("TCT_GOFMT_BIN")
        .or_else(|| crate::go_toolchain::gofmt_bin_os_string().ok())
        .unwrap_or_else(|| std::ffi::OsString::from("gofmt"));
    let mut child = ProcessCommand::new(&gofmt_bin)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|error| {
            SyntaxBackendBoundaryError::new(
                "random_admissible_walk.gofmt",
                format!("failed to start gofmt: {error}"),
            )
        })?;
    child
        .stdin
        .as_mut()
        .ok_or_else(|| {
            SyntaxBackendBoundaryError::new("random_admissible_walk.gofmt", "failed to open stdin")
        })?
        .write_all(source.as_bytes())
        .map_err(|error| {
            SyntaxBackendBoundaryError::new(
                "random_admissible_walk.gofmt",
                format!("failed to write source to gofmt: {error}"),
            )
        })?;
    let output = child.wait_with_output().map_err(|error| {
        SyntaxBackendBoundaryError::new(
            "random_admissible_walk.gofmt",
            format!("failed to wait for gofmt: {error}"),
        )
    })?;
    if !output.status.success() {
        return Err(SyntaxBackendBoundaryError::new(
            "random_admissible_walk.gofmt",
            format!(
                "gofmt rejected rendered random walk source: {}",
                String::from_utf8_lossy(&output.stderr)
            ),
        ));
    }
    String::from_utf8(output.stdout).map_err(|error| {
        SyntaxBackendBoundaryError::new(
            "random_admissible_walk.gofmt",
            format!("gofmt output was not UTF-8: {error}"),
        )
    })
}

fn go_parser_validate(
    go_bin: &str,
    source: &str,
    walk_index: usize,
    walk_seed: u64,
) -> Result<(), SyntaxBackendBoundaryError> {
    let dir = PathBuf::from("target/tct-syntax-go-random-admissible-walks/parser-sidecar");
    fs::create_dir_all(&dir).map_err(|error| {
        SyntaxBackendBoundaryError::new(
            "random_admissible_walk.go_parser",
            format!(
                "failed to create parser sidecar directory {}: {error}",
                dir.display()
            ),
        )
    })?;
    let program_path = dir.join(format!(
        "parse-random-walk-{}-{walk_index}-{walk_seed:016x}.go",
        std::process::id()
    ));
    fs::write(&program_path, GO_PARSER_STDIN_PROGRAM).map_err(|error| {
        SyntaxBackendBoundaryError::new(
            "random_admissible_walk.go_parser",
            format!(
                "failed to write parser sidecar {}: {error}",
                program_path.display()
            ),
        )
    })?;
    let mut child = ProcessCommand::new(go_bin)
        .arg("run")
        .arg(&program_path)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|error| {
            SyntaxBackendBoundaryError::new(
                "random_admissible_walk.go_parser",
                format!("failed to start go parser sidecar: {error}"),
            )
        })?;
    child
        .stdin
        .as_mut()
        .ok_or_else(|| {
            SyntaxBackendBoundaryError::new(
                "random_admissible_walk.go_parser",
                "failed to open parser sidecar stdin",
            )
        })?
        .write_all(source.as_bytes())
        .map_err(|error| {
            SyntaxBackendBoundaryError::new(
                "random_admissible_walk.go_parser",
                format!("failed to write source to parser sidecar: {error}"),
            )
        })?;
    let output = child.wait_with_output().map_err(|error| {
        SyntaxBackendBoundaryError::new(
            "random_admissible_walk.go_parser",
            format!("failed to wait for parser sidecar: {error}"),
        )
    })?;
    let _ = fs::remove_file(&program_path);
    if output.status.success() {
        Ok(())
    } else {
        Err(SyntaxBackendBoundaryError::new(
            "random_admissible_walk.go_parser",
            format!(
                "go/parser rejected rendered random walk source: {}",
                String::from_utf8_lossy(&output.stderr)
            ),
        ))
    }
}

const GO_PARSER_STDIN_PROGRAM: &str = r#"package main

import (
	"fmt"
	"go/parser"
	"go/token"
	"io"
	"os"
)

func main() {
	source, err := io.ReadAll(os.Stdin)
	if err != nil {
		fmt.Fprintln(os.Stderr, err)
		os.Exit(1)
	}
	if _, err := parser.ParseFile(token.NewFileSet(), "random_walk.go", source, parser.ParseComments); err != nil {
		fmt.Fprintln(os.Stderr, err)
		os.Exit(1)
	}
}
"#;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct DeterministicRng {
    state: u64,
}

impl DeterministicRng {
    fn new(seed: u64) -> Self {
        Self { state: seed }
    }

    fn next_u64(&mut self) -> u64 {
        self.state = self
            .state
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        splitmix64(self.state)
    }

    fn next_bounded(&mut self, bound: u64) -> u64 {
        if bound == 0 {
            0
        } else {
            self.next_u64() % bound
        }
    }
}

fn splitmix64(mut value: u64) -> u64 {
    value = value.wrapping_add(0x9e37_79b9_7f4a_7c15);
    value = (value ^ (value >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
    value = (value ^ (value >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
    value ^ (value >> 31)
}

fn validation_commands(config: &GoRandomAdmissibleWalkConfig) -> Vec<String> {
    vec![
        format!(
            "cargo run --bin tct-syntax-go-tokenizer-generator -- --random-admissible-walks --random-walk-count {} --random-walk-seed {} --random-walk-max-elided-steps {} --output {}",
            config.walk_count, config.seed, config.max_elided_steps, config.output_path
        ),
        format!(
            "cargo run --bin tct-syntax-go-tokenizer-generator -- --validate-report {}",
            config.output_path
        ),
        "cargo test --test syntax_go_unified_artifact_test unified_go_random_admissible_walk_report_is_deterministic -- --exact".to_string(),
        "git diff --check".to_string(),
    ]
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

fn require_one_of(
    field: &'static str,
    actual: &str,
    expected: &[&str],
) -> Result<(), SyntaxBackendBoundaryError> {
    if expected.contains(&actual) {
        Ok(())
    } else {
        Err(SyntaxBackendBoundaryError::new(
            field,
            format!("expected one of {expected:?}, got {actual:?}"),
        ))
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

fn require_non_empty(field: &'static str, actual: &str) -> Result<(), SyntaxBackendBoundaryError> {
    if actual.is_empty() {
        Err(SyntaxBackendBoundaryError::new(field, "value is empty"))
    } else {
        Ok(())
    }
}

fn require_nonzero(field: &'static str, actual: usize) -> Result<(), SyntaxBackendBoundaryError> {
    if actual == 0 {
        Err(SyntaxBackendBoundaryError::new(
            field,
            "value must be greater than zero",
        ))
    } else {
        Ok(())
    }
}

fn require_sha256(field: &'static str, actual: &str) -> Result<(), SyntaxBackendBoundaryError> {
    if actual.len() == 64 && actual.chars().all(|ch| ch.is_ascii_hexdigit()) {
        Ok(())
    } else {
        Err(SyntaxBackendBoundaryError::new(
            field,
            "value must be a hex-encoded sha256",
        ))
    }
}

fn require_some_sha256(
    field: &'static str,
    actual: Option<&str>,
) -> Result<(), SyntaxBackendBoundaryError> {
    require_sha256(
        field,
        actual.ok_or_else(|| SyntaxBackendBoundaryError::new(field, "missing sha256 value"))?,
    )
}

fn require_none<T>(
    field: &'static str,
    actual: &Option<T>,
) -> Result<(), SyntaxBackendBoundaryError> {
    if actual.is_none() {
        Ok(())
    } else {
        Err(SyntaxBackendBoundaryError::new(
            field,
            "expected field to be absent",
        ))
    }
}

fn require_contains(
    field: &'static str,
    actual: &[String],
    expected: &str,
) -> Result<(), SyntaxBackendBoundaryError> {
    if actual
        .iter()
        .any(|value| value == expected || value.contains(expected))
    {
        Ok(())
    } else {
        Err(SyntaxBackendBoundaryError::new(
            field,
            format!("missing required value containing {expected:?}"),
        ))
    }
}

fn require_str_contains(
    field: &'static str,
    actual: &str,
    expected: &str,
) -> Result<(), SyntaxBackendBoundaryError> {
    if actual.contains(expected) {
        Ok(())
    } else {
        Err(SyntaxBackendBoundaryError::new(
            field,
            format!("missing required text {expected:?}"),
        ))
    }
}
