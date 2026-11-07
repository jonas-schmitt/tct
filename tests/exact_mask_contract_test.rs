use tct::exact_mask_contract::{
    EosMaskDecision, ExactMaskContractReport, NEXT_BLOCKER_BPE_EXACT_LIFTING, RawPrefixMaskCase,
    RawPrefixOracleEvidence, RawPrefixStateTransition, RawTokenMaskDecision,
};
use tct::provenance::json_sha256;

#[test]
fn exact_mask_contract_accepts_exact_raw_prefix_cases() {
    let cases = vec![
        raw_case(
            "json-kubernetes-prefix",
            "json_schema",
            "kubernetes",
            vec![2, 4],
            false,
        ),
        raw_case(
            "go-crypto-header-comment-prefix",
            "go_syntax",
            "go_crypto_sysrand_rand_windows_source_file_syntax_v0",
            vec![14, 129],
            true,
        ),
    ];
    let report = ExactMaskContractReport::for_raw_prefix_cases(cases).unwrap();

    report.validate().unwrap();
    assert_eq!(report.summary.raw_prefix_case_count, 2);
    assert_eq!(report.summary.finite_oracle_case_count, 2);
    assert_eq!(report.summary.admitted_raw_token_count, 4);
    assert_eq!(report.summary.eos_admitted_count, 1);
    assert_eq!(report.summary.over_admission_count, 0);
    assert_eq!(report.summary.under_admission_count, 0);
    assert!(report.claim_boundary.raw_tct_prefix_exactness_claim_allowed);
    assert!(!report.claim_boundary.bpe_exactness_claim_allowed);
    assert!(!report.claim_boundary.arena_ir_proof_used);
    assert!(!report.claim_boundary.json_schema_output_changed);
}

#[test]
fn exact_mask_contract_accepts_next_workstream_blocker_after_json_evidence() {
    let report = ExactMaskContractReport::for_raw_prefix_cases(vec![raw_case(
        "json-mask-ready",
        "json_schema",
        "json_synthetic",
        vec![0, 1],
        false,
    )])
    .unwrap()
    .with_next_missing_workstream_blocker(NEXT_BLOCKER_BPE_EXACT_LIFTING);

    report.validate().unwrap();
    assert_eq!(
        report.next_missing_workstream_blocker,
        NEXT_BLOCKER_BPE_EXACT_LIFTING
    );
}

#[test]
fn exact_mask_contract_accepts_raw_prefix_state_transitions() {
    let case = raw_case(
        "json-eslint-parser-options",
        "json_schema",
        "eslint",
        vec![0, 1],
        false,
    );
    let source_prefix = vec![0_u32, 1, 2];
    let admitted = RawPrefixStateTransition::admitted(
        &case,
        &source_prefix,
        0,
        "json-eslint-parser-options-next-state",
        false,
        Default::default(),
    )
    .expect("admitted transition should hash");
    let rejected = RawPrefixStateTransition::rejected(
        &case,
        &source_prefix,
        511,
        "not_admissible_for_json_schema_prefix_state",
        Default::default(),
    )
    .expect("rejected transition should hash");
    let rejected_oracle_nonmember = RawPrefixStateTransition::rejected(
        &case,
        &source_prefix,
        128,
        "not_admissible_for_json_schema_prefix_state",
        Default::default(),
    )
    .expect("oracle-backed rejected transition should hash");

    admitted.validate_against_source(&case).unwrap();
    rejected.validate_against_source(&case).unwrap();
    rejected_oracle_nonmember
        .validate_against_source(&case)
        .unwrap();
    assert_eq!(admitted.next_prefix_raw_token_count, Some(4));
    assert!(rejected.next_prefix_state_id.is_none());
}

#[test]
fn exact_mask_contract_rejects_admitted_transition_without_raw_decision() {
    let case = raw_case(
        "json-eslint-parser-options",
        "json_schema",
        "eslint",
        vec![0, 1],
        false,
    );
    let transition = RawPrefixStateTransition::admitted(
        &case,
        &[0_u32, 1, 2],
        128,
        "json-eslint-parser-options-next-state",
        false,
        Default::default(),
    )
    .expect("transition fixture should hash");
    let error = transition.validate_against_source(&case).unwrap_err();

    assert_eq!(
        error.field(),
        "prefix_state_transitions.candidate_raw_token_id"
    );
}

#[test]
fn exact_mask_contract_rejects_prefix_state_transition_mask_mismatch() {
    let case = raw_case(
        "json-eslint-parser-options",
        "json_schema",
        "eslint",
        vec![0, 1],
        false,
    );
    let transition = RawPrefixStateTransition::admitted(
        &case,
        &[0_u32, 1, 2],
        511,
        "json-eslint-parser-options-next-state",
        false,
        Default::default(),
    )
    .expect("transition fixture should hash");
    let error = transition.validate_against_source(&case).unwrap_err();

    assert_eq!(error.field(), "prefix_state_transitions.admitted");
}

#[test]
fn exact_mask_contract_rejects_over_admission() {
    let mut case = raw_case("over-admit", "json_schema", "tsconfig", vec![3], false);
    case.raw_token_decisions
        .push(RawTokenMaskDecision::admitted(99));
    let error = case.validate().unwrap_err();

    assert_eq!(error.field(), "raw_token_decisions.oracle_alignment");
}

#[test]
fn exact_mask_contract_rejects_under_admission() {
    let mut case = raw_case("under-admit", "json_schema", "eslint", vec![5, 6], false);
    case.raw_token_decisions
        .retain(|decision| decision.candidate_raw_token_id != 6);
    let error = case.validate().unwrap_err();

    assert_eq!(error.field(), "raw_prefix_cases.under_admission");
}

#[test]
fn exact_mask_contract_accepts_rejection_when_prefix_has_no_completion() {
    let mut case = raw_case(
        "no-completion",
        "go_syntax",
        "go_crypto_sysrand_rand_windows_source_file_syntax_v0",
        vec![14],
        false,
    );
    case.raw_token_decisions.push(RawTokenMaskDecision {
        candidate_raw_token_id: 130,
        admitted: false,
        oracle_valid_next_token: false,
        candidate_preserves_prefix: true,
        valid_completion_exists: false,
        rejection_reason: Some("no_valid_completion_path".to_string()),
    });

    case.validate().unwrap();
}

#[test]
fn exact_mask_contract_rejects_eos_before_strict_finalization() {
    let mut case = raw_case(
        "bad-eos",
        "go_syntax",
        "go_minimal_function_syntax_v0",
        vec![1],
        false,
    );
    case.eos_decision.admitted = true;
    case.eos_decision.rejection_reason = None;
    let error = case.validate().unwrap_err();

    assert_eq!(error.field(), "eos_decision.admitted");
}

fn raw_case(
    case_id: &str,
    target_family: &str,
    target_id: &str,
    admissible: Vec<u32>,
    strict_finalization_legal: bool,
) -> RawPrefixMaskCase {
    let oracle = RawPrefixOracleEvidence::finite(admissible.clone(), strict_finalization_legal)
        .expect("fixture oracle should hash");
    let mut decisions = admissible
        .iter()
        .copied()
        .map(RawTokenMaskDecision::admitted)
        .collect::<Vec<_>>();
    decisions.push(RawTokenMaskDecision::rejected(511, "not admissible"));

    RawPrefixMaskCase {
        case_id: case_id.to_string(),
        target_family: target_family.to_string(),
        target_id: target_id.to_string(),
        prefix_state_id: format!("{case_id}-state"),
        prefix_raw_token_count: 3,
        prefix_raw_token_ids_sha256: json_sha256(&vec![0_u32, 1, 2]).unwrap(),
        metadata: Default::default(),
        oracle,
        raw_token_decisions: decisions,
        eos_decision: EosMaskDecision::from_oracle(
            &RawPrefixOracleEvidence::finite(admissible, strict_finalization_legal)
                .expect("fixture oracle should hash"),
        ),
    }
}
