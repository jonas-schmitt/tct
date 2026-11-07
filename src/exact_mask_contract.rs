//! Shared exact admissible-token mask contract.
//!
//! This module defines the target-neutral report shape and validation rules for
//! TCT-native syntax-bounded masking. Target-specific generators provide the
//! oracle evidence; this contract checks that the emitted mask decisions are
//! exact for that evidence.

use crate::provenance::json_sha256;
use crate::syntax_backend_boundary::SyntaxBackendBoundaryError;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

pub const TOOL: &str = "tct_exact_mask_contract";
pub const VERSION: u32 = 1;
pub const ARTIFACT_KIND: &str = "tct_exact_admissible_token_mask_contract_report";
pub const STATUS: &str = "exact_mask_contract_checked";
pub const RAW_TCT_PROJECTION_LEVEL: &str = "raw_tct";
pub const EXACTNESS_DEFINITION: &str =
    "admit_iff_candidate_has_valid_completion_path_and_preserves_prefix";
pub const EOS_EXACTNESS_DEFINITION: &str =
    "admit_eos_iff_strict_decode_and_validation_finalization_is_legal";
pub const RAW_PREFIX_ORACLE_KIND: &str = "finite_raw_prefix_admissibility_oracle_v1";
pub const RAW_PREFIX_STATE_TRANSITION_KIND: &str = "raw_prefix_state_transition_v1";
pub const NEXT_BLOCKER_JSON_PREFIX_ADMISSIBILITY: &str =
    "json_schema_generated_tokenizer_raw_prefix_admissibility_evidence";
pub const NEXT_BLOCKER_BPE_EXACT_LIFTING: &str = "exact_bpe_lift_from_raw_prefix_admissibility";
pub const NEXT_BLOCKER_GENERATED_PREFIX_STATE_API: &str =
    "generated_tokenizer_prefix_state_api_required_for_json_schema_raw_admissibility";

const ALLOWED_NEXT_WORKSTREAM_BLOCKERS: &[&str] = &[
    NEXT_BLOCKER_JSON_PREFIX_ADMISSIBILITY,
    NEXT_BLOCKER_BPE_EXACT_LIFTING,
    NEXT_BLOCKER_GENERATED_PREFIX_STATE_API,
];

const REQUIRED_BLOCKED_CLAIMS: &[&str] = &[
    "no_arena_ir_claim",
    "no_bpe_exactness_claim_until_projection_checked",
    "no_full_go_coverage_claim",
    "no_full_json_schema_dsl_coverage_claim",
    "no_generated_hot_path_claim",
    "no_model_quality_claim",
    "no_paper_facing_claim",
    "no_stable_backend_claim",
    "no_type_correctness_claim",
];

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExactMaskContractReport {
    pub artifact_kind: String,
    pub version: u32,
    pub tool: String,
    pub status: String,
    pub projection_level: String,
    pub exactness_contract: ExactMaskContract,
    pub raw_prefix_cases: Vec<RawPrefixMaskCase>,
    pub raw_prefix_cases_sha256: String,
    pub summary: ExactMaskSummary,
    pub claim_boundary: ExactMaskClaimBoundary,
    pub validation_commands: Vec<String>,
    pub next_missing_workstream_blocker: String,
}

impl ExactMaskContractReport {
    pub fn for_raw_prefix_cases(
        raw_prefix_cases: Vec<RawPrefixMaskCase>,
    ) -> Result<Self, SyntaxBackendBoundaryError> {
        let raw_prefix_cases_sha256 = hash("raw_prefix_cases_sha256", &raw_prefix_cases)?;
        let summary = ExactMaskSummary::from_cases(&raw_prefix_cases)?;
        Ok(Self {
            artifact_kind: ARTIFACT_KIND.to_string(),
            version: VERSION,
            tool: TOOL.to_string(),
            status: STATUS.to_string(),
            projection_level: RAW_TCT_PROJECTION_LEVEL.to_string(),
            exactness_contract: ExactMaskContract::raw_tct(),
            raw_prefix_cases,
            raw_prefix_cases_sha256,
            summary,
            claim_boundary: ExactMaskClaimBoundary::raw_contract_only(),
            validation_commands: vec![
                "cargo test --test exact_mask_contract_test".to_string(),
                "git diff --check".to_string(),
            ],
            next_missing_workstream_blocker: NEXT_BLOCKER_JSON_PREFIX_ADMISSIBILITY.to_string(),
        })
    }

    pub fn with_next_missing_workstream_blocker(mut self, blocker: impl Into<String>) -> Self {
        self.next_missing_workstream_blocker = blocker.into();
        self
    }

    pub fn validate(&self) -> Result<(), SyntaxBackendBoundaryError> {
        require_string("artifact_kind", &self.artifact_kind, ARTIFACT_KIND)?;
        require_u32("version", self.version, VERSION)?;
        require_string("tool", &self.tool, TOOL)?;
        require_string("status", &self.status, STATUS)?;
        require_string(
            "projection_level",
            &self.projection_level,
            RAW_TCT_PROJECTION_LEVEL,
        )?;
        self.exactness_contract.validate()?;
        require_non_empty("raw_prefix_cases", self.raw_prefix_cases.len())?;
        let mut case_ids = BTreeSet::new();
        for case in &self.raw_prefix_cases {
            case.validate()?;
            if !case_ids.insert(case.case_id.as_str()) {
                return Err(err(
                    "raw_prefix_cases.case_id",
                    format!("duplicate case_id {}", case.case_id),
                ));
            }
        }
        require_sha256(
            "raw_prefix_cases_sha256",
            &self.raw_prefix_cases_sha256,
            &self.raw_prefix_cases,
        )?;
        self.summary.validate_against(&self.raw_prefix_cases)?;
        self.claim_boundary.validate(&self.summary)?;
        require_non_empty("validation_commands", self.validation_commands.len())?;
        if ALLOWED_NEXT_WORKSTREAM_BLOCKERS
            .iter()
            .any(|allowed| *allowed == self.next_missing_workstream_blocker)
        {
            Ok(())
        } else {
            Err(err(
                "next_missing_workstream_blocker",
                format!(
                    "expected one of {:?}, got {}",
                    ALLOWED_NEXT_WORKSTREAM_BLOCKERS, self.next_missing_workstream_blocker
                ),
            ))
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExactMaskContract {
    pub raw_token_exactness_definition: String,
    pub eos_exactness_definition: String,
    pub finite_raw_oracle_required: bool,
    pub over_admission_is_error: bool,
    pub under_admission_is_error: bool,
    pub eos_requires_strict_finalization: bool,
    pub bpe_projection_checked: bool,
}

impl ExactMaskContract {
    pub fn raw_tct() -> Self {
        Self {
            raw_token_exactness_definition: EXACTNESS_DEFINITION.to_string(),
            eos_exactness_definition: EOS_EXACTNESS_DEFINITION.to_string(),
            finite_raw_oracle_required: true,
            over_admission_is_error: true,
            under_admission_is_error: true,
            eos_requires_strict_finalization: true,
            bpe_projection_checked: false,
        }
    }

    fn validate(&self) -> Result<(), SyntaxBackendBoundaryError> {
        require_string(
            "exactness_contract.raw_token_exactness_definition",
            &self.raw_token_exactness_definition,
            EXACTNESS_DEFINITION,
        )?;
        require_string(
            "exactness_contract.eos_exactness_definition",
            &self.eos_exactness_definition,
            EOS_EXACTNESS_DEFINITION,
        )?;
        require_bool(
            "exactness_contract.finite_raw_oracle_required",
            self.finite_raw_oracle_required,
            true,
        )?;
        require_bool(
            "exactness_contract.over_admission_is_error",
            self.over_admission_is_error,
            true,
        )?;
        require_bool(
            "exactness_contract.under_admission_is_error",
            self.under_admission_is_error,
            true,
        )?;
        require_bool(
            "exactness_contract.eos_requires_strict_finalization",
            self.eos_requires_strict_finalization,
            true,
        )?;
        require_bool(
            "exactness_contract.bpe_projection_checked",
            self.bpe_projection_checked,
            false,
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RawPrefixMaskCase {
    pub case_id: String,
    pub target_family: String,
    pub target_id: String,
    pub prefix_state_id: String,
    pub prefix_raw_token_count: usize,
    pub prefix_raw_token_ids_sha256: String,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub metadata: BTreeMap<String, String>,
    pub oracle: RawPrefixOracleEvidence,
    pub raw_token_decisions: Vec<RawTokenMaskDecision>,
    pub eos_decision: EosMaskDecision,
}

impl RawPrefixMaskCase {
    pub fn validate(&self) -> Result<(), SyntaxBackendBoundaryError> {
        require_text("raw_prefix_cases.case_id", &self.case_id)?;
        require_text("raw_prefix_cases.target_family", &self.target_family)?;
        require_text("raw_prefix_cases.target_id", &self.target_id)?;
        require_text("raw_prefix_cases.prefix_state_id", &self.prefix_state_id)?;
        require_text(
            "raw_prefix_cases.prefix_raw_token_ids_sha256",
            &self.prefix_raw_token_ids_sha256,
        )?;
        validate_metadata(&self.metadata)?;
        self.oracle.validate()?;
        require_non_empty(
            "raw_prefix_cases.raw_token_decisions",
            self.raw_token_decisions.len(),
        )?;
        let oracle_tokens = self.oracle.admissible_token_set()?;
        let mut decisions_by_token = BTreeMap::new();
        for decision in &self.raw_token_decisions {
            decision.validate_against_oracle(&oracle_tokens)?;
            if decisions_by_token
                .insert(decision.candidate_raw_token_id, decision)
                .is_some()
            {
                return Err(err(
                    "raw_prefix_cases.raw_token_decisions.candidate_raw_token_id",
                    format!("duplicate candidate {}", decision.candidate_raw_token_id),
                ));
            }
        }
        for token in &oracle_tokens {
            match decisions_by_token.get(token) {
                Some(decision) if decision.admitted => {}
                Some(_) => {
                    return Err(err(
                        "raw_prefix_cases.under_admission",
                        format!("oracle token {token} was rejected"),
                    ));
                }
                None => {
                    return Err(err(
                        "raw_prefix_cases.under_admission",
                        format!("oracle token {token} has no mask decision"),
                    ));
                }
            }
        }
        self.eos_decision.validate()
    }

    fn exact_raw_token_count(&self) -> usize {
        self.raw_token_decisions
            .iter()
            .filter(|decision| decision.admitted)
            .count()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RawPrefixOracleEvidence {
    pub oracle_kind: String,
    pub finite_admissible_set_available: bool,
    pub admissible_raw_token_ids: Vec<u32>,
    pub admissible_raw_token_ids_sha256: String,
    pub strict_finalization_legal: bool,
    pub strict_decode_and_validation_run: bool,
}

impl RawPrefixOracleEvidence {
    pub fn finite(
        admissible_raw_token_ids: Vec<u32>,
        strict_finalization_legal: bool,
    ) -> Result<Self, SyntaxBackendBoundaryError> {
        let admissible_raw_token_ids_sha256 = hash(
            "oracle.admissible_raw_token_ids_sha256",
            &admissible_raw_token_ids,
        )?;
        Ok(Self {
            oracle_kind: RAW_PREFIX_ORACLE_KIND.to_string(),
            finite_admissible_set_available: true,
            admissible_raw_token_ids,
            admissible_raw_token_ids_sha256,
            strict_finalization_legal,
            strict_decode_and_validation_run: true,
        })
    }

    fn validate(&self) -> Result<(), SyntaxBackendBoundaryError> {
        require_string(
            "oracle.oracle_kind",
            &self.oracle_kind,
            RAW_PREFIX_ORACLE_KIND,
        )?;
        require_bool(
            "oracle.finite_admissible_set_available",
            self.finite_admissible_set_available,
            true,
        )?;
        require_sha256(
            "oracle.admissible_raw_token_ids_sha256",
            &self.admissible_raw_token_ids_sha256,
            &self.admissible_raw_token_ids,
        )?;
        require_bool(
            "oracle.strict_decode_and_validation_run",
            self.strict_decode_and_validation_run,
            true,
        )?;
        self.admissible_token_set().map(|_| ())
    }

    fn admissible_token_set(&self) -> Result<BTreeSet<u32>, SyntaxBackendBoundaryError> {
        let set = self
            .admissible_raw_token_ids
            .iter()
            .copied()
            .collect::<BTreeSet<_>>();
        if set.len() != self.admissible_raw_token_ids.len() {
            return Err(err(
                "oracle.admissible_raw_token_ids",
                "duplicate oracle token id",
            ));
        }
        Ok(set)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RawTokenMaskDecision {
    pub candidate_raw_token_id: u32,
    pub admitted: bool,
    pub oracle_valid_next_token: bool,
    pub candidate_preserves_prefix: bool,
    pub valid_completion_exists: bool,
    pub rejection_reason: Option<String>,
}

impl RawTokenMaskDecision {
    pub fn admitted(candidate_raw_token_id: u32) -> Self {
        Self {
            candidate_raw_token_id,
            admitted: true,
            oracle_valid_next_token: true,
            candidate_preserves_prefix: true,
            valid_completion_exists: true,
            rejection_reason: None,
        }
    }

    pub fn rejected(candidate_raw_token_id: u32, reason: impl Into<String>) -> Self {
        Self {
            candidate_raw_token_id,
            admitted: false,
            oracle_valid_next_token: false,
            candidate_preserves_prefix: false,
            valid_completion_exists: false,
            rejection_reason: Some(reason.into()),
        }
    }

    fn validate_against_oracle(
        &self,
        oracle_tokens: &BTreeSet<u32>,
    ) -> Result<(), SyntaxBackendBoundaryError> {
        let oracle_allows = oracle_tokens.contains(&self.candidate_raw_token_id);
        let candidate_has_valid_next =
            self.candidate_preserves_prefix && self.valid_completion_exists;
        require_bool(
            "raw_token_decisions.oracle_valid_next_token",
            self.oracle_valid_next_token,
            candidate_has_valid_next,
        )?;
        require_bool(
            "raw_token_decisions.oracle_alignment",
            self.oracle_valid_next_token,
            oracle_allows,
        )?;
        require_bool("raw_token_decisions.admitted", self.admitted, oracle_allows)?;
        if self.admitted && self.rejection_reason.is_some() {
            return Err(err(
                "raw_token_decisions.rejection_reason",
                "admitted token must not carry a rejection reason",
            ));
        }
        if !self.admitted && self.rejection_reason.is_none() {
            return Err(err(
                "raw_token_decisions.rejection_reason",
                "rejected token must carry a rejection reason",
            ));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RawPrefixStateTransition {
    pub transition_kind: String,
    pub source_case_id: String,
    pub source_prefix_state_id: String,
    pub source_prefix_raw_token_count: usize,
    pub source_prefix_raw_token_ids_sha256: String,
    pub candidate_raw_token_id: u32,
    pub admitted: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub next_prefix_state_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub next_prefix_raw_token_count: Option<usize>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub next_prefix_raw_token_ids_sha256: Option<String>,
    pub strict_finalization_legal_after_candidate: bool,
    pub valid_completion_exists_after_candidate: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rejection_reason: Option<String>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub metadata: BTreeMap<String, String>,
}

impl RawPrefixStateTransition {
    pub fn admitted(
        source_case: &RawPrefixMaskCase,
        source_prefix_raw_token_ids: &[u32],
        candidate_raw_token_id: u32,
        next_prefix_state_id: impl Into<String>,
        strict_finalization_legal_after_candidate: bool,
        metadata: BTreeMap<String, String>,
    ) -> Result<Self, SyntaxBackendBoundaryError> {
        let mut next_prefix = source_prefix_raw_token_ids.to_vec();
        next_prefix.push(candidate_raw_token_id);
        Ok(Self {
            transition_kind: RAW_PREFIX_STATE_TRANSITION_KIND.to_string(),
            source_case_id: source_case.case_id.clone(),
            source_prefix_state_id: source_case.prefix_state_id.clone(),
            source_prefix_raw_token_count: source_prefix_raw_token_ids.len(),
            source_prefix_raw_token_ids_sha256: hash(
                "prefix_state_transitions.source_prefix_raw_token_ids_sha256",
                &source_prefix_raw_token_ids,
            )?,
            candidate_raw_token_id,
            admitted: true,
            next_prefix_state_id: Some(next_prefix_state_id.into()),
            next_prefix_raw_token_count: Some(next_prefix.len()),
            next_prefix_raw_token_ids_sha256: Some(hash(
                "prefix_state_transitions.next_prefix_raw_token_ids_sha256",
                &next_prefix,
            )?),
            strict_finalization_legal_after_candidate,
            valid_completion_exists_after_candidate: true,
            rejection_reason: None,
            metadata,
        })
    }

    pub fn rejected(
        source_case: &RawPrefixMaskCase,
        source_prefix_raw_token_ids: &[u32],
        candidate_raw_token_id: u32,
        reason: impl Into<String>,
        metadata: BTreeMap<String, String>,
    ) -> Result<Self, SyntaxBackendBoundaryError> {
        Ok(Self {
            transition_kind: RAW_PREFIX_STATE_TRANSITION_KIND.to_string(),
            source_case_id: source_case.case_id.clone(),
            source_prefix_state_id: source_case.prefix_state_id.clone(),
            source_prefix_raw_token_count: source_prefix_raw_token_ids.len(),
            source_prefix_raw_token_ids_sha256: hash(
                "prefix_state_transitions.source_prefix_raw_token_ids_sha256",
                &source_prefix_raw_token_ids,
            )?,
            candidate_raw_token_id,
            admitted: false,
            next_prefix_state_id: None,
            next_prefix_raw_token_count: None,
            next_prefix_raw_token_ids_sha256: None,
            strict_finalization_legal_after_candidate: false,
            valid_completion_exists_after_candidate: false,
            rejection_reason: Some(reason.into()),
            metadata,
        })
    }

    pub fn validate_against_source(
        &self,
        source_case: &RawPrefixMaskCase,
    ) -> Result<(), SyntaxBackendBoundaryError> {
        require_string(
            "prefix_state_transitions.transition_kind",
            &self.transition_kind,
            RAW_PREFIX_STATE_TRANSITION_KIND,
        )?;
        require_text(
            "prefix_state_transitions.source_case_id",
            &self.source_case_id,
        )?;
        require_string(
            "prefix_state_transitions.source_case_id",
            &self.source_case_id,
            &source_case.case_id,
        )?;
        require_string(
            "prefix_state_transitions.source_prefix_state_id",
            &self.source_prefix_state_id,
            &source_case.prefix_state_id,
        )?;
        require_usize(
            "prefix_state_transitions.source_prefix_raw_token_count",
            self.source_prefix_raw_token_count,
            source_case.prefix_raw_token_count,
        )?;
        require_string(
            "prefix_state_transitions.source_prefix_raw_token_ids_sha256",
            &self.source_prefix_raw_token_ids_sha256,
            &source_case.prefix_raw_token_ids_sha256,
        )?;
        validate_metadata(&self.metadata)?;
        let oracle_tokens = source_case.oracle.admissible_token_set()?;
        let decision = source_case
            .raw_token_decisions
            .iter()
            .find(|decision| decision.candidate_raw_token_id == self.candidate_raw_token_id);
        if let Some(decision) = decision {
            require_bool(
                "prefix_state_transitions.admitted",
                self.admitted,
                decision.admitted,
            )?;
            require_bool(
                "prefix_state_transitions.valid_completion_exists_after_candidate",
                self.valid_completion_exists_after_candidate,
                decision.valid_completion_exists,
            )?;
        } else if self.admitted || oracle_tokens.contains(&self.candidate_raw_token_id) {
            return Err(err(
                "prefix_state_transitions.candidate_raw_token_id",
                format!(
                    "candidate {} has no raw mask decision in source case {}",
                    self.candidate_raw_token_id, source_case.case_id
                ),
            ));
        } else {
            require_bool("prefix_state_transitions.admitted", self.admitted, false)?;
            require_bool(
                "prefix_state_transitions.valid_completion_exists_after_candidate",
                self.valid_completion_exists_after_candidate,
                false,
            )?;
        }
        if self.admitted {
            require_text(
                "prefix_state_transitions.next_prefix_state_id",
                self.next_prefix_state_id.as_deref().ok_or_else(|| {
                    err(
                        "prefix_state_transitions.next_prefix_state_id",
                        "admitted transition must expose a next prefix state",
                    )
                })?,
            )?;
            require_usize(
                "prefix_state_transitions.next_prefix_raw_token_count",
                self.next_prefix_raw_token_count.ok_or_else(|| {
                    err(
                        "prefix_state_transitions.next_prefix_raw_token_count",
                        "admitted transition must expose a next prefix token count",
                    )
                })?,
                self.source_prefix_raw_token_count + 1,
            )?;
            require_text(
                "prefix_state_transitions.next_prefix_raw_token_ids_sha256",
                self.next_prefix_raw_token_ids_sha256
                    .as_deref()
                    .ok_or_else(|| {
                        err(
                            "prefix_state_transitions.next_prefix_raw_token_ids_sha256",
                            "admitted transition must expose a next prefix hash",
                        )
                    })?,
            )?;
            if self.rejection_reason.is_some() {
                return Err(err(
                    "prefix_state_transitions.rejection_reason",
                    "admitted transition must not carry a rejection reason",
                ));
            }
        } else {
            if self.next_prefix_state_id.is_some()
                || self.next_prefix_raw_token_count.is_some()
                || self.next_prefix_raw_token_ids_sha256.is_some()
            {
                return Err(err(
                    "prefix_state_transitions.next_prefix_state",
                    "rejected transition must not expose a next prefix state",
                ));
            }
            require_bool(
                "prefix_state_transitions.strict_finalization_legal_after_candidate",
                self.strict_finalization_legal_after_candidate,
                false,
            )?;
            if self.rejection_reason.is_none() {
                return Err(err(
                    "prefix_state_transitions.rejection_reason",
                    "rejected transition must carry a rejection reason",
                ));
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EosMaskDecision {
    pub admitted: bool,
    pub strict_finalization_legal: bool,
    pub strict_decode_and_validation_run: bool,
    pub rejection_reason: Option<String>,
}

impl EosMaskDecision {
    pub fn from_oracle(oracle: &RawPrefixOracleEvidence) -> Self {
        Self {
            admitted: oracle.strict_finalization_legal,
            strict_finalization_legal: oracle.strict_finalization_legal,
            strict_decode_and_validation_run: oracle.strict_decode_and_validation_run,
            rejection_reason: (!oracle.strict_finalization_legal)
                .then(|| "strict_finalization_not_legal".to_string()),
        }
    }

    fn validate(&self) -> Result<(), SyntaxBackendBoundaryError> {
        require_bool(
            "eos_decision.strict_decode_and_validation_run",
            self.strict_decode_and_validation_run,
            true,
        )?;
        require_bool(
            "eos_decision.admitted",
            self.admitted,
            self.strict_finalization_legal,
        )?;
        if self.admitted && self.rejection_reason.is_some() {
            return Err(err(
                "eos_decision.rejection_reason",
                "admitted EOS must not carry a rejection reason",
            ));
        }
        if !self.admitted && self.rejection_reason.is_none() {
            return Err(err(
                "eos_decision.rejection_reason",
                "rejected EOS must carry a rejection reason",
            ));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExactMaskSummary {
    pub raw_prefix_case_count: usize,
    pub finite_oracle_case_count: usize,
    pub raw_candidate_decision_count: usize,
    pub admitted_raw_token_count: usize,
    pub rejected_raw_token_count: usize,
    pub eos_admitted_count: usize,
    pub exact_raw_prefix_case_count: usize,
    pub over_admission_count: usize,
    pub under_admission_count: usize,
}

impl ExactMaskSummary {
    fn from_cases(
        raw_prefix_cases: &[RawPrefixMaskCase],
    ) -> Result<Self, SyntaxBackendBoundaryError> {
        let mut raw_candidate_decision_count = 0;
        let mut admitted_raw_token_count = 0;
        let mut eos_admitted_count = 0;
        for case in raw_prefix_cases {
            case.validate()?;
            raw_candidate_decision_count += case.raw_token_decisions.len();
            admitted_raw_token_count += case.exact_raw_token_count();
            if case.eos_decision.admitted {
                eos_admitted_count += 1;
            }
        }
        Ok(Self {
            raw_prefix_case_count: raw_prefix_cases.len(),
            finite_oracle_case_count: raw_prefix_cases.len(),
            raw_candidate_decision_count,
            admitted_raw_token_count,
            rejected_raw_token_count: raw_candidate_decision_count - admitted_raw_token_count,
            eos_admitted_count,
            exact_raw_prefix_case_count: raw_prefix_cases.len(),
            over_admission_count: 0,
            under_admission_count: 0,
        })
    }

    fn validate_against(
        &self,
        raw_prefix_cases: &[RawPrefixMaskCase],
    ) -> Result<(), SyntaxBackendBoundaryError> {
        let actual = Self::from_cases(raw_prefix_cases)?;
        require_usize(
            "summary.raw_prefix_case_count",
            self.raw_prefix_case_count,
            actual.raw_prefix_case_count,
        )?;
        require_usize(
            "summary.finite_oracle_case_count",
            self.finite_oracle_case_count,
            actual.finite_oracle_case_count,
        )?;
        require_usize(
            "summary.raw_candidate_decision_count",
            self.raw_candidate_decision_count,
            actual.raw_candidate_decision_count,
        )?;
        require_usize(
            "summary.admitted_raw_token_count",
            self.admitted_raw_token_count,
            actual.admitted_raw_token_count,
        )?;
        require_usize(
            "summary.rejected_raw_token_count",
            self.rejected_raw_token_count,
            actual.rejected_raw_token_count,
        )?;
        require_usize(
            "summary.eos_admitted_count",
            self.eos_admitted_count,
            actual.eos_admitted_count,
        )?;
        require_usize(
            "summary.exact_raw_prefix_case_count",
            self.exact_raw_prefix_case_count,
            actual.exact_raw_prefix_case_count,
        )?;
        require_usize("summary.over_admission_count", self.over_admission_count, 0)?;
        require_usize(
            "summary.under_admission_count",
            self.under_admission_count,
            0,
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExactMaskClaimBoundary {
    pub raw_tct_prefix_exactness_claim_allowed: bool,
    pub bpe_exactness_claim_allowed: bool,
    pub json_schema_output_changed: bool,
    pub arena_ir_proof_used: bool,
    pub blocked_claims: Vec<String>,
}

impl ExactMaskClaimBoundary {
    fn raw_contract_only() -> Self {
        Self {
            raw_tct_prefix_exactness_claim_allowed: true,
            bpe_exactness_claim_allowed: false,
            json_schema_output_changed: false,
            arena_ir_proof_used: false,
            blocked_claims: REQUIRED_BLOCKED_CLAIMS
                .iter()
                .map(|claim| (*claim).to_string())
                .collect(),
        }
    }

    fn validate(&self, summary: &ExactMaskSummary) -> Result<(), SyntaxBackendBoundaryError> {
        require_bool(
            "claim_boundary.raw_tct_prefix_exactness_claim_allowed",
            self.raw_tct_prefix_exactness_claim_allowed,
            summary.raw_prefix_case_count == summary.exact_raw_prefix_case_count
                && summary.over_admission_count == 0
                && summary.under_admission_count == 0,
        )?;
        require_bool(
            "claim_boundary.bpe_exactness_claim_allowed",
            self.bpe_exactness_claim_allowed,
            false,
        )?;
        require_bool(
            "claim_boundary.json_schema_output_changed",
            self.json_schema_output_changed,
            false,
        )?;
        require_bool(
            "claim_boundary.arena_ir_proof_used",
            self.arena_ir_proof_used,
            false,
        )?;
        let blocked = self
            .blocked_claims
            .iter()
            .map(String::as_str)
            .collect::<BTreeSet<_>>();
        for claim in REQUIRED_BLOCKED_CLAIMS {
            if !blocked.contains(claim) {
                return Err(err(
                    "claim_boundary.blocked_claims",
                    format!("missing blocked claim {claim}"),
                ));
            }
        }
        Ok(())
    }
}

fn hash<T: Serialize>(
    field: &'static str,
    value: &T,
) -> Result<String, SyntaxBackendBoundaryError> {
    json_sha256(value).map_err(|error| err(field, format!("failed to hash JSON: {error}")))
}

fn require_sha256<T: Serialize>(
    field: &'static str,
    actual: &str,
    value: &T,
) -> Result<(), SyntaxBackendBoundaryError> {
    let expected = hash(field, value)?;
    require_string(field, actual, &expected)
}

fn require_string(
    field: &'static str,
    actual: &str,
    expected: &str,
) -> Result<(), SyntaxBackendBoundaryError> {
    if actual == expected {
        Ok(())
    } else {
        Err(err(field, format!("expected {expected}, got {actual}")))
    }
}

fn require_text(field: &'static str, value: &str) -> Result<(), SyntaxBackendBoundaryError> {
    if value.is_empty() {
        Err(err(field, "must not be empty"))
    } else {
        Ok(())
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
        Err(err(field, format!("expected {expected}, got {actual}")))
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
        Err(err(field, format!("expected {expected}, got {actual}")))
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
        Err(err(field, format!("expected {expected}, got {actual}")))
    }
}

fn require_non_empty(field: &'static str, count: usize) -> Result<(), SyntaxBackendBoundaryError> {
    if count == 0 {
        Err(err(field, "must not be empty"))
    } else {
        Ok(())
    }
}

fn validate_metadata(
    metadata: &BTreeMap<String, String>,
) -> Result<(), SyntaxBackendBoundaryError> {
    for (key, value) in metadata {
        require_text("raw_prefix_cases.metadata.key", key)?;
        require_text("raw_prefix_cases.metadata.value", value)?;
        if key.ends_with("_sha256") {
            require_hex_sha256("raw_prefix_cases.metadata.sha256", value)?;
        }
    }
    Ok(())
}

fn require_hex_sha256(field: &'static str, value: &str) -> Result<(), SyntaxBackendBoundaryError> {
    let is_hex = value
        .bytes()
        .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase());
    if value.len() == 64 && is_hex {
        Ok(())
    } else {
        Err(err(field, "must be a lowercase SHA-256 hex digest"))
    }
}

fn err(field: &'static str, message: impl Into<String>) -> SyntaxBackendBoundaryError {
    SyntaxBackendBoundaryError::new(field, message)
}
