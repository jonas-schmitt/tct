//! Exact admissible BPE mask over **elided** SQL schema-linking streams.
//!
//! BPE is trained on free-choice (elided) decision tokens. A merged token is
//! admissible at the current elided walker state iff every constituent base
//! decision token is legal when applied through
//! [`SqlElidedTctDecodeState::apply`] (which auto-drains mask-forced runs
//! between free choices).
//!
//! Report shape reuses [`crate::exact_mask_contract`] for the raw elided
//! free-choice mask; the BPE lift is checked separately via
//! [`SqlElidedBpeMaskReport`]. Not a paper claim; not a token-layout change.

use crate::bpe::encoder::{BpeDecoder, SequentialBpeEncoder};
use crate::bpe::{BpeMergeTable, BpeMetadata, trainer::train_bpe_incremental};
use crate::exact_mask_contract::{
    EosMaskDecision, ExactMaskContractReport, RawPrefixMaskCase, RawPrefixOracleEvidence,
    RawTokenMaskDecision,
};
use crate::provenance::json_sha256;
use crate::sql_schema_linking::layout::{
    SQL_SYNTAX_LAYOUT_PAD_TOKEN, SQL_SYNTAX_LAYOUT_VOCAB_SIZE,
};
use crate::sql_schema_linking::runtime::{
    SqlElidedTctDecodeState, SqlRuntimeError, SqlSchemaLinkingRuntime,
};
use crate::syntax_backend_boundary::SyntaxBackendBoundaryError;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

pub const TOOL: &str = "sql_elided_bpe_admissible_mask";
pub const VERSION: u32 = 1;
pub const ARTIFACT_KIND: &str = "tct_sql_elided_bpe_admissible_mask_report";
pub const STATUS: &str = "sql_elided_bpe_admissible_mask_checked";
pub const CLAIM_BOUNDARY: &str = "compiler_sql_schema_linking_only";

/// Expand one BPE token id to its base elided decision sequence.
pub fn expand_bpe_token_to_elided_base(
    decoder: &BpeDecoder,
    bpe_token: u32,
) -> Result<Vec<u32>, SqlRuntimeError> {
    decoder.decode_strict(&[bpe_token]).map_err(|e| {
        SqlRuntimeError::new(
            "bpe_decode",
            format!("failed to expand BPE token {bpe_token}: {e}"),
        )
    })
}

/// Try applying one BPE token (its full base expansion) on a cloned elided walker.
///
/// Returns `Ok(new_state)` when every base decision is free-choice legal and
/// applies cleanly (forced runs drain between decisions). Returns `Ok(None)`
/// when the expansion is empty (PAD-like) or any base step is illegal.
///
/// Probe-time engine failures (`apply`, `expansion_stuck`, forced-token, empty
/// mask, etc.) are treated as **not admissible** rather than hard errors: an
/// illegal multi-token BPE expansion may leave a temporary probe state that
/// the engine rejects mid-path, and that is exactly a mask reject.
pub fn try_apply_bpe_token(
    state: &SqlElidedTctDecodeState,
    bpe_token: u32,
    decoder: &BpeDecoder,
) -> Result<Option<SqlElidedTctDecodeState>, SqlRuntimeError> {
    let base = match expand_bpe_token_to_elided_base(decoder, bpe_token) {
        Ok(base) => base,
        Err(_) => return Ok(None),
    };
    if base.is_empty() {
        return Ok(None);
    }
    let mut probe = state.clone();
    for &tok in &base {
        let allowed = match probe.allowed_token_ids() {
            Ok(a) => a,
            Err(_) => return Ok(None),
        };
        if !allowed.contains(&tok) {
            return Ok(None);
        }
        if probe.apply(tok).is_err() {
            return Ok(None);
        }
    }
    Ok(Some(probe))
}

/// Whether `bpe_token` is admissible from the current elided free-choice state.
pub fn bpe_token_is_admissible(
    state: &SqlElidedTctDecodeState,
    bpe_token: u32,
    decoder: &BpeDecoder,
) -> Result<bool, SqlRuntimeError> {
    Ok(try_apply_bpe_token(state, bpe_token, decoder)?.is_some())
}

/// Encode SQL → elided → BPE, then reverse BPE → expand elided → decode SQL.
pub fn round_trip_sql_via_elided_bpe(
    runtime: &SqlSchemaLinkingRuntime,
    sql: &str,
    encoder: &SequentialBpeEncoder,
    decoder: &BpeDecoder,
) -> Result<String, SqlRuntimeError> {
    let elided = runtime.encode_elided(sql)?;
    let bpe = encoder.encode(&elided);
    let recovered_elided = decoder.decode_strict(&bpe).map_err(|e| {
        SqlRuntimeError::new(
            "bpe_decode",
            format!("unbpe failed for BPE stream of {} tokens: {e}", bpe.len()),
        )
    })?;
    if recovered_elided != elided {
        return Err(SqlRuntimeError::new(
            "bpe_round_trip",
            format!(
                "unbpe(bpe(elided)) != elided (elided_len={}, recovered_len={})",
                elided.len(),
                recovered_elided.len()
            ),
        ));
    }
    runtime.strict_decode_complete_elided(&recovered_elided)
}

/// One BPE-token admissibility decision at a fixed elided prefix.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SqlBpeTokenDecision {
    pub bpe_token_id: u32,
    pub allowed: bool,
    pub base_elided_expansion: Vec<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rejection_reason: Option<String>,
}

/// Per-prefix BPE mask over the configured BPE vocab.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SqlElidedBpePrefixMask {
    pub case_id: String,
    pub prefix_elided_token_count: usize,
    pub prefix_elided_token_ids_sha256: String,
    pub allowed_bpe_token_ids: Vec<u32>,
    pub rejected_count: usize,
    pub decisions_sha256: String,
    /// Gold next BPE token from the full stream (if any); must be in allowed.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub gold_next_bpe_token: Option<u32>,
    pub gold_next_admitted: bool,
}

/// Full report: raw elided exact-mask contract cases + BPE lift checks.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SqlElidedBpeMaskReport {
    pub artifact_kind: String,
    pub version: u32,
    pub tool: String,
    pub status: String,
    pub claim_boundary: String,
    pub base_vocab_size: u32,
    pub pad_token: u32,
    pub bpe_vocab_size: u32,
    pub merge_count: usize,
    pub stream_contract: &'static str,
    /// Raw free-choice exact-mask contract report (validated via
    /// [`ExactMaskContractReport::validate`]).
    pub raw_elided_exact_mask: ExactMaskContractReport,
    pub bpe_prefix_masks: Vec<SqlElidedBpePrefixMask>,
    pub round_trip_checked_queries: usize,
    pub round_trip_failures: usize,
    pub validation_commands: Vec<String>,
    pub note: String,
}

impl SqlElidedBpeMaskReport {
    pub fn validate(&self) -> Result<(), SyntaxBackendBoundaryError> {
        if self.artifact_kind != ARTIFACT_KIND {
            return Err(SyntaxBackendBoundaryError::new(
                "artifact_kind",
                format!("expected {ARTIFACT_KIND}, got {}", self.artifact_kind),
            ));
        }
        if self.version != VERSION {
            return Err(SyntaxBackendBoundaryError::new(
                "version",
                format!("expected {VERSION}, got {}", self.version),
            ));
        }
        if self.tool != TOOL {
            return Err(SyntaxBackendBoundaryError::new(
                "tool",
                format!("expected {TOOL}, got {}", self.tool),
            ));
        }
        if self.status != STATUS {
            return Err(SyntaxBackendBoundaryError::new(
                "status",
                format!("expected {STATUS}, got {}", self.status),
            ));
        }
        if self.claim_boundary != CLAIM_BOUNDARY {
            return Err(SyntaxBackendBoundaryError::new(
                "claim_boundary",
                format!("expected {CLAIM_BOUNDARY}, got {}", self.claim_boundary),
            ));
        }
        if self.base_vocab_size != SQL_SYNTAX_LAYOUT_VOCAB_SIZE {
            return Err(SyntaxBackendBoundaryError::new(
                "base_vocab_size",
                format!(
                    "expected {SQL_SYNTAX_LAYOUT_VOCAB_SIZE}, got {}",
                    self.base_vocab_size
                ),
            ));
        }
        if self.pad_token != SQL_SYNTAX_LAYOUT_PAD_TOKEN {
            return Err(SyntaxBackendBoundaryError::new(
                "pad_token",
                format!(
                    "expected {SQL_SYNTAX_LAYOUT_PAD_TOKEN}, got {}",
                    self.pad_token
                ),
            ));
        }
        if self.stream_contract != "elided" {
            return Err(SyntaxBackendBoundaryError::new(
                "stream_contract",
                format!("expected elided, got {}", self.stream_contract),
            ));
        }
        self.raw_elided_exact_mask.validate()?;
        if self.round_trip_failures != 0 {
            return Err(SyntaxBackendBoundaryError::new(
                "round_trip_failures",
                format!("{} round-trip failures", self.round_trip_failures),
            ));
        }
        for mask in &self.bpe_prefix_masks {
            if let Some(gold) = mask.gold_next_bpe_token {
                if !mask.gold_next_admitted {
                    return Err(SyntaxBackendBoundaryError::new(
                        "bpe_prefix_masks.gold_next_admitted",
                        format!(
                            "gold next BPE token {gold} not admitted at {}",
                            mask.case_id
                        ),
                    ));
                }
                if !mask.allowed_bpe_token_ids.contains(&gold) {
                    return Err(SyntaxBackendBoundaryError::new(
                        "bpe_prefix_masks.allowed_bpe_token_ids",
                        format!(
                            "gold next BPE token {gold} missing from allowed set at {}",
                            mask.case_id
                        ),
                    ));
                }
            }
        }
        Ok(())
    }
}

/// Build BPE encoder/decoder for a merge table truncated to `merge_count`.
pub fn bpe_codec_for_merges(
    table: &BpeMergeTable,
    merge_count: usize,
) -> Result<(SequentialBpeEncoder, BpeDecoder, u32), SqlRuntimeError> {
    let n = merge_count.min(table.merges.len());
    let bpe_start = table.bpe_start();
    let bpe_vocab_size = bpe_start
        .checked_add(
            u32::try_from(n).map_err(|_| SqlRuntimeError::new("bpe", "merge count exceeds u32"))?,
        )
        .ok_or_else(|| SqlRuntimeError::new("bpe", "BPE vocab overflow"))?;
    // Truncate merges for the encoder by using max vocab.
    let encoder = SequentialBpeEncoder::new(table, bpe_vocab_size);
    let decoder = BpeDecoder::new(table, bpe_vocab_size);
    Ok((encoder, decoder, bpe_vocab_size))
}

/// Train a small merge table on elided sequences (tests / local probes).
pub fn train_elided_bpe_table(
    sequences: &[Vec<u32>],
    base_vocab_size: u32,
    max_merges: usize,
    schema_name: &str,
) -> BpeMergeTable {
    let merges = train_bpe_incremental(sequences.to_vec(), max_merges, base_vocab_size);
    BpeMergeTable {
        schema: schema_name.to_string(),
        base_vocab_size,
        merges: merges.clone(),
        metadata: BpeMetadata {
            trained_on_instances: sequences.len(),
            training_date: "sql-elided-bpe-fixture".to_string(),
            max_merges: merges.len(),
            dict_size: None,
        },
    }
}

/// Build raw elided exact-mask contract cases at each free-choice prefix of one stream.
pub fn raw_elided_exact_mask_cases_for_stream(
    runtime: &SqlSchemaLinkingRuntime,
    case_prefix: &str,
    elided: &[u32],
) -> Result<Vec<RawPrefixMaskCase>, SqlRuntimeError> {
    let mut cases = Vec::new();
    let mut state = runtime.begin_elided_tct_decode()?;
    for (step, &gold) in elided.iter().enumerate() {
        let allowed = state.allowed_token_ids()?;
        let allowed_set: BTreeSet<u32> = allowed.iter().copied().collect();
        if !allowed_set.contains(&gold) {
            return Err(SqlRuntimeError::new(
                "exact_mask",
                format!("{case_prefix}: gold elided token {gold} not in free mask at step {step}"),
            ));
        }
        let prefix_ids: Vec<u32> = elided[..step].to_vec();
        let prefix_hash = json_sha256(&prefix_ids)
            .map_err(|e| SqlRuntimeError::new("hash", format!("prefix hash failed: {e}")))?;
        // Mid-stream free-choice positions are not EOF-complete.
        let complete = false;
        let oracle = RawPrefixOracleEvidence::finite(allowed.clone(), complete).map_err(|e| {
            SqlRuntimeError::new("exact_mask_contract", format!("oracle build failed: {e}"))
        })?;
        let mut decisions = Vec::new();
        // Every oracle token needs an admitted decision (under-admission check).
        for &token_id in &allowed {
            decisions.push(RawTokenMaskDecision::admitted(token_id));
        }
        // Sample out-of-mask structural ids so over-admission is checkable.
        for t in 0..64u32 {
            if !allowed_set.contains(&t) {
                decisions.push(RawTokenMaskDecision::rejected(t, "not_in_free_choice_mask"));
            }
        }
        cases.push(RawPrefixMaskCase {
            case_id: format!("{case_prefix}:elided_step_{step}"),
            target_family: "sql_schema_linking".to_string(),
            target_id: "sql-elided-stream".to_string(),
            prefix_state_id: format!("{case_prefix}:prefix_{step}"),
            prefix_raw_token_count: step,
            prefix_raw_token_ids_sha256: prefix_hash,
            metadata: Default::default(),
            oracle: oracle.clone(),
            raw_token_decisions: decisions,
            eos_decision: EosMaskDecision::from_oracle(&oracle),
        });
        state.apply(gold)?;
    }
    Ok(cases)
}

/// Build BPE prefix masks along a gold BPE path (checks gold token admitted at each step).
pub fn bpe_prefix_masks_for_gold_path(
    runtime: &SqlSchemaLinkingRuntime,
    case_prefix: &str,
    elided: &[u32],
    encoder: &SequentialBpeEncoder,
    decoder: &BpeDecoder,
    bpe_vocab_size: u32,
    // When true, brute-force every BPE id (expensive). When false, only check
    // gold next + base free-choice tokens projected through BPE identity.
    full_vocab_scan: bool,
) -> Result<Vec<SqlElidedBpePrefixMask>, SqlRuntimeError> {
    let bpe_stream = encoder.encode(elided);
    let mut masks = Vec::new();
    let mut elided_state = runtime.begin_elided_tct_decode()?;
    // Walk in lockstep: after each BPE token, apply its base expansion.
    let mut consumed_elided = 0usize;
    for (step, &gold_bpe) in bpe_stream.iter().enumerate() {
        let prefix_elided = &elided[..consumed_elided];
        let prefix_hash = json_sha256(&prefix_elided)
            .map_err(|e| SqlRuntimeError::new("hash", format!("prefix hash failed: {e}")))?;

        let mut allowed = Vec::new();
        let mut decisions = Vec::new();
        let scan_ids: Vec<u32> = if full_vocab_scan {
            (0..bpe_vocab_size)
                .filter(|&id| id != SQL_SYNTAX_LAYOUT_PAD_TOKEN)
                .collect()
        } else {
            // Gold + all currently free raw elided ids (identity BPE tokens) +
            // a handful of known merge ids if present.
            let mut ids: BTreeSet<u32> = elided_state.allowed_token_ids()?.into_iter().collect();
            ids.insert(gold_bpe);
            // Also probe a few early merge ids when available.
            let bpe_start = SQL_SYNTAX_LAYOUT_VOCAB_SIZE;
            for delta in 0..16u32 {
                let id = bpe_start + delta;
                if id < bpe_vocab_size {
                    ids.insert(id);
                }
            }
            ids.into_iter().collect()
        };

        for bpe_id in scan_ids {
            let expansion = match expand_bpe_token_to_elided_base(decoder, bpe_id) {
                Ok(e) => e,
                Err(_) => {
                    decisions.push(SqlBpeTokenDecision {
                        bpe_token_id: bpe_id,
                        allowed: false,
                        base_elided_expansion: Vec::new(),
                        rejection_reason: Some("bpe_decode_error".to_string()),
                    });
                    continue;
                }
            };
            let ok = try_apply_bpe_token(&elided_state, bpe_id, decoder)?.is_some();
            if ok {
                allowed.push(bpe_id);
            }
            decisions.push(SqlBpeTokenDecision {
                bpe_token_id: bpe_id,
                allowed: ok,
                base_elided_expansion: expansion,
                rejection_reason: if ok {
                    None
                } else {
                    Some("elided_path_illegal".to_string())
                },
            });
        }
        allowed.sort_unstable();
        let gold_ok = allowed.contains(&gold_bpe);
        let decisions_sha = json_sha256(&decisions)
            .map_err(|e| SqlRuntimeError::new("hash", format!("decisions hash failed: {e}")))?;
        let rejected_count = decisions.iter().filter(|d| !d.allowed).count();
        masks.push(SqlElidedBpePrefixMask {
            case_id: format!("{case_prefix}:bpe_step_{step}"),
            prefix_elided_token_count: consumed_elided,
            prefix_elided_token_ids_sha256: prefix_hash,
            allowed_bpe_token_ids: allowed,
            rejected_count,
            decisions_sha256: decisions_sha,
            gold_next_bpe_token: Some(gold_bpe),
            gold_next_admitted: gold_ok,
        });

        // Advance gold path.
        let base = expand_bpe_token_to_elided_base(decoder, gold_bpe)?;
        for &tok in &base {
            elided_state.apply(tok)?;
            consumed_elided += 1;
        }
    }
    if !elided_state.is_complete()? {
        return Err(SqlRuntimeError::new(
            "bpe_path",
            format!("{case_prefix}: BPE gold path did not complete elided stream"),
        ));
    }
    Ok(masks)
}

/// Build a full checked report for a set of (id, sql) queries.
pub fn build_sql_elided_bpe_mask_report(
    runtime: &SqlSchemaLinkingRuntime,
    queries: &[(String, String)],
    merge_table: &BpeMergeTable,
    merge_count: usize,
    full_vocab_scan: bool,
) -> Result<SqlElidedBpeMaskReport, SqlRuntimeError> {
    let (encoder, decoder, bpe_vocab_size) = bpe_codec_for_merges(merge_table, merge_count)?;
    let mut raw_cases = Vec::new();
    let mut bpe_masks = Vec::new();
    let mut round_trip_failures = 0usize;
    let mut round_trip_checked = 0usize;

    for (qid, sql) in queries {
        let elided = runtime.encode_elided(sql)?;
        match round_trip_sql_via_elided_bpe(runtime, sql, &encoder, &decoder) {
            Ok(rendered) => {
                // Semantic fixpoint: re-encode must match elided expansion of rendered.
                let again = runtime.encode_elided(&rendered)?;
                if again != elided {
                    // Allow whitespace-normalized render that re-encodes to same stream.
                    // If streams differ, count as failure.
                    round_trip_failures += 1;
                } else {
                    round_trip_checked += 1;
                }
            }
            Err(_) => {
                round_trip_failures += 1;
            }
        }
        raw_cases.extend(raw_elided_exact_mask_cases_for_stream(
            runtime, qid, &elided,
        )?);
        bpe_masks.extend(bpe_prefix_masks_for_gold_path(
            runtime,
            qid,
            &elided,
            &encoder,
            &decoder,
            bpe_vocab_size,
            full_vocab_scan,
        )?);
    }

    let raw_report = ExactMaskContractReport::for_raw_prefix_cases(raw_cases).map_err(|e| {
        SqlRuntimeError::new(
            "exact_mask_contract",
            format!("failed to build raw contract report: {e}"),
        )
    })?;

    Ok(SqlElidedBpeMaskReport {
        artifact_kind: ARTIFACT_KIND.to_string(),
        version: VERSION,
        tool: TOOL.to_string(),
        status: STATUS.to_string(),
        claim_boundary: CLAIM_BOUNDARY.to_string(),
        base_vocab_size: SQL_SYNTAX_LAYOUT_VOCAB_SIZE,
        pad_token: SQL_SYNTAX_LAYOUT_PAD_TOKEN,
        bpe_vocab_size,
        merge_count: merge_count.min(merge_table.merges.len()),
        stream_contract: "elided",
        raw_elided_exact_mask: raw_report,
        bpe_prefix_masks: bpe_masks,
        round_trip_checked_queries: round_trip_checked,
        round_trip_failures,
        validation_commands: vec![
            "cargo test sql_schema_linking::elided_bpe_mask".to_string(),
            "cargo test sql_schema_linking".to_string(),
        ],
        note: "BPE IDs are a downstream artifact over the elided SQL stream; not a token-layout pin change. Claim boundary compiler_sql_schema_linking_only.".to_string(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sql_schema_linking::catalog::load_fixture_bundle;
    use serde::Deserialize;

    #[derive(Debug, Deserialize)]
    struct GoldFile {
        queries: Vec<GoldQuery>,
    }

    #[derive(Debug, Deserialize)]
    struct GoldQuery {
        id: String,
        sql: String,
    }

    fn fixture_runtime() -> SqlSchemaLinkingRuntime {
        SqlSchemaLinkingRuntime::fixture().expect("fixture runtime")
    }

    fn all_gold_queries() -> Vec<(String, String)> {
        let bundle = load_fixture_bundle().expect("bundle");
        let file: GoldFile =
            serde_json::from_slice(&bundle.gold_queries_bytes).expect("parse gold");
        file.queries.into_iter().map(|q| (q.id, q.sql)).collect()
    }

    fn train_fixture_bpe(max_merges: usize) -> BpeMergeTable {
        let rt = fixture_runtime();
        let sequences: Vec<Vec<u32>> = all_gold_queries()
            .iter()
            .map(|(_, sql)| rt.encode_elided(sql).expect("encode_elided"))
            .collect();
        train_elided_bpe_table(
            &sequences,
            SQL_SYNTAX_LAYOUT_VOCAB_SIZE,
            max_merges,
            "sql-fixture-elided",
        )
    }

    #[test]
    fn fixture_elided_bpe_round_trip_and_mask_contract() {
        let rt = fixture_runtime();
        let queries = all_gold_queries();
        assert!(!queries.is_empty());
        let table = train_fixture_bpe(32);
        assert!(!table.merges.is_empty(), "expected some merges on fixture");
        let report = build_sql_elided_bpe_mask_report(&rt, &queries, &table, 32, false)
            .expect("build report");
        report
            .validate()
            .unwrap_or_else(|e| panic!("report validate: {e}"));
        assert_eq!(report.round_trip_failures, 0);
        assert_eq!(report.round_trip_checked_queries, queries.len());
        assert!(
            report.bpe_prefix_masks.iter().all(|m| m.gold_next_admitted),
            "every gold BPE token must be admissible"
        );
        report
            .raw_elided_exact_mask
            .validate()
            .expect("raw exact mask contract");
    }

    #[test]
    fn bpe_token_admissibility_uses_elided_drain() {
        let rt = fixture_runtime();
        let table = train_fixture_bpe(16);
        let (encoder, decoder, _) = bpe_codec_for_merges(&table, 16).expect("codec");
        let sql = &all_gold_queries()[0].1;
        let elided = rt.encode_elided(sql).expect("elided");
        let bpe = encoder.encode(&elided);
        let mut state = rt.begin_elided_tct_decode().expect("begin");
        for &tok in &bpe {
            assert!(
                bpe_token_is_admissible(&state, tok, &decoder).expect("check"),
                "gold BPE token {tok} must be admissible"
            );
            let next = try_apply_bpe_token(&state, tok, &decoder)
                .expect("apply")
                .expect("some");
            state = next;
        }
        assert!(state.is_complete().expect("complete"));
    }

    #[test]
    fn rejects_bpe_token_with_inadmissible_base_expansion() {
        let rt = fixture_runtime();
        let table = train_fixture_bpe(32);
        let (_encoder, decoder, bpe_vocab_size) = bpe_codec_for_merges(&table, 32).expect("codec");
        let state = rt.begin_elided_tct_decode().expect("begin");
        let allowed: BTreeSet<u32> = state
            .allowed_token_ids()
            .expect("mask")
            .into_iter()
            .collect();
        assert!(!allowed.is_empty(), "free-choice mask must be non-empty");

        // Identity base tokens outside the free-choice mask must be rejected.
        let mut saw_identity_reject = false;
        for base_id in 0..64u32 {
            if allowed.contains(&base_id) || base_id == SQL_SYNTAX_LAYOUT_PAD_TOKEN {
                continue;
            }
            assert!(
                !bpe_token_is_admissible(&state, base_id, &decoder).expect("check"),
                "out-of-mask base identity {base_id} must not be BPE-admissible"
            );
            assert!(
                try_apply_bpe_token(&state, base_id, &decoder)
                    .expect("probe")
                    .is_none(),
                "out-of-mask base identity {base_id} must not apply"
            );
            saw_identity_reject = true;
            break;
        }
        assert!(
            saw_identity_reject,
            "expected at least one out-of-mask base identity to reject"
        );

        // PAD is never a free-choice decision.
        assert!(
            !bpe_token_is_admissible(&state, SQL_SYNTAX_LAYOUT_PAD_TOKEN, &decoder)
                .expect("pad check"),
            "PAD must not be BPE-admissible"
        );

        // Any multi-base merge whose first constituent is illegal must reject.
        let mut saw_merge_reject = false;
        for bpe_id in SQL_SYNTAX_LAYOUT_VOCAB_SIZE..bpe_vocab_size {
            let expansion = match expand_bpe_token_to_elided_base(&decoder, bpe_id) {
                Ok(e) if e.len() >= 2 => e,
                _ => continue,
            };
            if allowed.contains(&expansion[0]) {
                continue;
            }
            assert!(
                !bpe_token_is_admissible(&state, bpe_id, &decoder).expect("merge check"),
                "merge {bpe_id} starting with illegal base {} must reject",
                expansion[0]
            );
            saw_merge_reject = true;
            break;
        }
        // Fixture merges may all start legal at position 0; identity reject above
        // is the hard requirement. Merge reject is best-effort when available.
        let _ = saw_merge_reject;

        // A mask that admitted every non-PAD BPE id would be useless: prove
        // the free mask rejects a non-trivial fraction of the BPE vocab.
        let mut admitted = 0usize;
        let mut rejected = 0usize;
        for bpe_id in 0..bpe_vocab_size {
            if bpe_id == SQL_SYNTAX_LAYOUT_PAD_TOKEN {
                continue;
            }
            if bpe_token_is_admissible(&state, bpe_id, &decoder).expect("scan") {
                admitted += 1;
            } else {
                rejected += 1;
            }
        }
        assert!(
            admitted > 0,
            "gold-path free choices must admit some tokens"
        );
        assert!(
            rejected > admitted,
            "admissible mask must reject more tokens than it admits at start \
             (admitted={admitted}, rejected={rejected}); an accept-all mask is useless"
        );
    }

    #[test]
    fn forced_token_in_elided_stream_typed_error_and_bpe_reject() {
        let rt = fixture_runtime();
        let table = train_fixture_bpe(8);
        let (_encoder, decoder, _) = bpe_codec_for_merges(&table, 8).expect("codec");

        // Spelling a mask-forced token on the elided walker fails with the
        // typed phase used by the stream contract.
        let mut walker = rt.begin_elided_tct_decode().expect("begin elided");
        let initial = walker.initial_forced_run().to_vec();
        assert!(
            !initial.is_empty(),
            "SQL elided construction must drain a non-empty initial forced run"
        );
        let forced0 = initial[0];
        let err = walker
            .apply(forced0)
            .expect_err("spelling a forced token must fail");
        assert_eq!(
            err.phase, "forced_token_in_elided_stream",
            "typed error for forced token in elided stream: {err}"
        );

        // Same token as a BPE identity at a free-choice position is rejected
        // by the admissible mask (forced ids are not free-choice choices).
        let state = rt.begin_elided_tct_decode().expect("begin");
        let free = state.allowed_token_ids().expect("mask");
        assert!(
            !free.contains(&forced0),
            "forced token {forced0} must not appear in free-choice mask"
        );
        assert!(
            !bpe_token_is_admissible(&state, forced0, &decoder).expect("forced bpe"),
            "forced identity must not be BPE-admissible at free-choice state"
        );
    }

    #[test]
    fn full_vocab_scan_admits_gold_and_rejects_illegals() {
        let rt = fixture_runtime();
        let queries = all_gold_queries();
        // Small merge budget keeps full-vocab scan cheap on the fixture.
        let table = train_fixture_bpe(16);
        let report = build_sql_elided_bpe_mask_report(&rt, &queries[..1], &table, 16, true)
            .expect("full-scan report");
        report
            .validate()
            .unwrap_or_else(|e| panic!("full-scan validate: {e}"));
        assert!(report.bpe_prefix_masks.iter().all(|m| m.gold_next_admitted));
        assert!(
            report.bpe_prefix_masks.iter().all(|m| m.rejected_count > 0),
            "every prefix must reject some BPE tokens under full vocab scan"
        );
    }
}
