//! Raw syntax-token replay and application engine.
//!
//! This private module owns the mutable raw-prefix state, admissible-token
//! masks, transition application, payload materialization, and decoded event
//! emission behind the public `CompiledSyntaxModel` API.

use super::factored_menu_codec::{
    FACTORED_COLUMN_MENU_DIGITS, factored_column_digit_admissible, factored_flat_ordinal,
    factored_table_digit_admissible, factored_table_slice,
};
use super::menu_codec::{
    menu_digit_admissible, menu_digit_admissible_in_range, menu_digit_end, menu_digit_width,
};
use super::model_facts::{
    AlternativeBodyKind, ChildSpec, DiscriminatedAlternative, FieldSpec, ModelFacts, PayloadSpec,
    RecordSpec,
};
use super::{
    ELIDED_FORCED_RUN_LIMIT, ElidedForcedRawToken, ElidedRawSyntaxAdvance, ElidedRawSyntaxStream,
    MenuSiteBindings, RawSyntaxAdmissibleTokens, RawSyntaxAdvanceDelta, RawSyntaxDecodedEvent,
    RawSyntaxDecodedEventKind, RawSyntaxError, RawSyntaxSelectedCompletionStep,
    RawSyntaxSelectedCompletionTrace, RawSyntaxState, RawSyntaxStatus, RawSyntaxStream,
    SyntaxTokenRoles,
};
use crate::syntax_go_source_instance_projection::ProjectionTokenLayoutRangeSummary;
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum CompletionMode {
    RequireComplete,
    AllowPrefix,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct RawSyntaxDecodedEventContext {
    rule: Option<String>,
    field: Option<String>,
    node_kind: Option<String>,
    payload_kind: Option<String>,
}

impl RawSyntaxDecodedEventContext {
    fn new(
        rule: Option<String>,
        field: Option<String>,
        node_kind: Option<String>,
        payload_kind: Option<String>,
    ) -> Self {
        Self {
            rule,
            field,
            node_kind,
            payload_kind,
        }
    }
}

#[derive(Debug, Default, Clone, PartialEq, Eq)]
struct RawSyntaxDecodedPayloadMaterialization {
    payload_byte_count: usize,
    payload_token_byte: Option<u8>,
    payload_value: Option<String>,
}

/// Owned mutable raw-prefix decoder state (no model borrows).
///
/// Advances against an immutable [`super::CompiledSyntaxModel`] / model facts in
/// O(one transition) per accepted token. Used by the borrowed
/// [`super::RawSyntaxStream`] handle and by owned incremental cursors (e.g. SQL
/// A5 `SqlTctDecodeState`) without self-referential structs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RawSyntaxLiveState {
    stack: Vec<FrameState>,
    payload: Option<PayloadState>,
    consumed: usize,
    root_closed: bool,
    record_events: bool,
    last_event_kind: Option<RawSyntaxDecodedEventKind>,
    decoded_events: Vec<RawSyntaxDecodedEvent>,
    /// Successful `apply_token` transitions executed on this state.
    /// Used to prove live incremental advance (exactly one transition per step)
    /// rather than full-prefix replay.
    applied_transitions: u64,
}

impl Default for RawSyntaxLiveState {
    fn default() -> Self {
        Self::without_event_recording()
    }
}

impl RawSyntaxLiveState {
    pub fn new_recording_events() -> Self {
        Self {
            stack: Vec::new(),
            payload: None,
            consumed: 0,
            root_closed: false,
            record_events: true,
            last_event_kind: None,
            decoded_events: Vec::new(),
            applied_transitions: 0,
        }
    }

    pub fn without_event_recording() -> Self {
        Self {
            record_events: false,
            ..Self::new_recording_events()
        }
    }

    pub fn applied_transitions(&self) -> u64 {
        self.applied_transitions
    }

    pub fn consumed_raw_tokens(&self) -> usize {
        self.consumed
    }

    pub fn decoded_events(&self) -> &[RawSyntaxDecodedEvent] {
        &self.decoded_events
    }

    pub(super) fn take_decoded_events(&mut self) -> Vec<RawSyntaxDecodedEvent> {
        std::mem::take(&mut self.decoded_events)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct RawSyntaxStateMachine<'a> {
    facts: &'a ModelFacts,
    roles: &'a SyntaxTokenRoles,
    /// Empty-binding fast-out keeps menu paths unreachable (**P10**).
    menu_bindings: &'a MenuSiteBindings,
    pub(super) live: RawSyntaxLiveState,
}

impl<'a> RawSyntaxStateMachine<'a> {
    pub(super) fn new(
        facts: &'a ModelFacts,
        roles: &'a SyntaxTokenRoles,
        menu_bindings: &'a MenuSiteBindings,
    ) -> Self {
        Self {
            facts,
            roles,
            menu_bindings,
            live: RawSyntaxLiveState::new_recording_events(),
        }
    }

    /// Engine for the incremental `RawSyntaxStream` handle: identical token
    /// application, but `decoded_events` stays empty so long-lived handles
    /// do not grow O(stream) decoded-event context.
    pub(super) fn without_event_recording(
        facts: &'a ModelFacts,
        roles: &'a SyntaxTokenRoles,
        menu_bindings: &'a MenuSiteBindings,
    ) -> Self {
        Self {
            facts,
            roles,
            menu_bindings,
            live: RawSyntaxLiveState::without_event_recording(),
        }
    }

    pub(super) fn from_live(
        facts: &'a ModelFacts,
        roles: &'a SyntaxTokenRoles,
        menu_bindings: &'a MenuSiteBindings,
        live: RawSyntaxLiveState,
    ) -> Self {
        Self {
            facts,
            roles,
            menu_bindings,
            live,
        }
    }

    pub(super) fn into_live(self) -> RawSyntaxLiveState {
        self.live
    }

    /// Same transition as [`super::RawSyntaxStream::advance`]: one token, count
    /// applied transitions, error-atomic.
    pub(super) fn advance_one(
        &mut self,
        raw_token_id: u32,
    ) -> Result<super::RawSyntaxAdvanceDelta, RawSyntaxError> {
        self.live.last_event_kind = None;
        self.apply_token(raw_token_id)?;
        let Some(event_kind) = self.live.last_event_kind else {
            return Err(self.error(
                "missing_state_metadata",
                format!("accepted raw token {raw_token_id} recorded no decoded event kind"),
                Some(raw_token_id),
            ));
        };
        self.live.consumed += 1;
        self.live.applied_transitions += 1;
        let eof_valid = self.eof_valid();
        Ok(super::RawSyntaxAdvanceDelta {
            applied_raw_token_id: raw_token_id,
            consumed_raw_tokens: self.live.consumed,
            event_kind,
            status: if eof_valid {
                super::RawSyntaxStatus::Complete
            } else {
                super::RawSyntaxStatus::Partial
            },
            eof_valid,
            stack_depth: self.live.stack.len(),
            payload_active: self.live.payload.is_some(),
            payload_byte_count: self
                .live
                .payload
                .as_ref()
                .map_or(0, |payload| payload.byte_count),
        })
    }

    pub(super) fn replay(
        mut self,
        raw_token_ids: &[u32],
        completion: CompletionMode,
    ) -> Result<(Self, RawSyntaxState), RawSyntaxError> {
        for token in raw_token_ids.iter().copied() {
            self.apply_token(token)?;
            self.live.consumed += 1;
            self.live.applied_transitions += 1;
        }

        match completion {
            CompletionMode::RequireComplete if !self.eof_valid() => {
                let message = match self.active_payload_kind() {
                    Some(class) => format!(
                        "EOF reached mid-payload (expected class {class}) before root closed"
                    ),
                    None => "EOF reached before root closed".to_string(),
                };
                Err(self.error_at("premature_eof", message, self.live.consumed, None))
            }
            CompletionMode::RequireComplete => {
                let state = self.state(
                    RawSyntaxStatus::Complete,
                    raw_token_ids.len(),
                    "complete raw syntax-token stream accepted",
                );
                Ok((self, state))
            }
            CompletionMode::AllowPrefix => {
                let state = self.prefix_state();
                Ok((self, state))
            }
        }
    }

    pub(super) fn eof_valid(&self) -> bool {
        self.live.stack.is_empty() && self.live.payload.is_none() && self.live.root_closed
    }

    /// `AllowPrefix`-classified state for the consumed prefix. Shared by
    /// `replay` and the incremental `RawSyntaxStream::state_snapshot` so the
    /// two paths cannot diverge on status classification or diagnostics;
    /// `total_raw_tokens` equals the consumed-token count on both paths.
    pub(super) fn prefix_state(&self) -> RawSyntaxState {
        if self.eof_valid() {
            self.state(
                RawSyntaxStatus::Complete,
                self.live.consumed,
                "prefix replay reached complete EOF-valid state",
            )
        } else {
            self.state(
                RawSyntaxStatus::Partial,
                self.live.consumed,
                "valid raw syntax-token prefix accepted as partial state",
            )
        }
    }

    pub(super) fn selected_completion_trace(
        mut self,
        raw_token_ids: &[u32],
        sample_steps: &BTreeSet<usize>,
    ) -> Result<RawSyntaxSelectedCompletionTrace, RawSyntaxError> {
        let mut admissible_mask_query_count = 0usize;
        let mut selected_transition_accepted_count = 0usize;
        let mut finite_admissible_raw_token_mask_available = true;
        let every_selected_raw_token_admitted = true;
        let mut step_samples = Vec::with_capacity(sample_steps.len());

        for (step_index, token) in raw_token_ids.iter().copied().enumerate() {
            let admissible = self.admissible_next_raw_tokens();
            admissible_mask_query_count += 1;
            let finite_admissible_raw_token_count = finite_admissible_raw_token_count(&admissible);
            if finite_admissible_raw_token_count == 0 {
                finite_admissible_raw_token_mask_available = false;
            }
            let selected_raw_token_admitted = admissible_contains_raw_token(&admissible, token);
            if !selected_raw_token_admitted {
                return Err(self.error(
                    "unexpected_structural_token",
                    format!(
                        "selected raw token {token} at step {step_index} is not admissible for the active syntax state"
                    ),
                    Some(token),
                ));
            }

            self.apply_token(token)?;
            self.live.consumed += 1;
            selected_transition_accepted_count += 1;
            let transition_state = self.state(
                if self.eof_valid() {
                    RawSyntaxStatus::Complete
                } else {
                    RawSyntaxStatus::Partial
                },
                self.live.consumed,
                "selected raw token transition accepted",
            );
            if sample_steps.contains(&step_index) {
                step_samples.push(RawSyntaxSelectedCompletionStep {
                    step_index,
                    prefix_raw_token_count: step_index,
                    selected_raw_token_id: token,
                    finite_admissible_raw_token_count,
                    selected_raw_token_admitted,
                    selected_transition_accepted: true,
                    transition_state,
                });
            }
        }

        if !self.eof_valid() {
            let message = match self.active_payload_kind() {
                Some(class) => {
                    format!("EOF reached mid-payload (expected class {class}) before root closed")
                }
                None => "EOF reached before root closed".to_string(),
            };
            return Err(self.error_at("premature_eof", message, self.live.consumed, None));
        }
        let final_state = self.state(
            RawSyntaxStatus::Complete,
            raw_token_ids.len(),
            "selected completion raw syntax-token stream accepted",
        );
        Ok(RawSyntaxSelectedCompletionTrace {
            transition_count: raw_token_ids.len(),
            admissible_mask_query_count,
            finite_admissible_raw_token_mask_available,
            every_selected_raw_token_admitted,
            selected_transition_accepted_count,
            final_state,
            step_samples,
        })
    }

    pub(super) fn apply_token(&mut self, token: u32) -> Result<(), RawSyntaxError> {
        if token >= self.roles.vocab_size {
            return Err(self.error(
                "token_id_out_of_vocab",
                format!(
                    "token {token} at offset {} is >= VOCAB_SIZE",
                    self.live.consumed
                ),
                Some(token),
            ));
        }
        if token == self.roles.pad_token {
            return Err(self.error(
                "pad_token_in_raw_stream",
                format!(
                    "PAD_TOKEN {token} appears at raw offset {}",
                    self.live.consumed
                ),
                Some(token),
            ));
        }
        if self.live.root_closed {
            return Err(self.error(
                "extra_trailing_token",
                "raw token appears after the root node closed",
                Some(token),
            ));
        }
        if self.live.payload.is_some() {
            return self.apply_payload_token(token);
        }
        // Mixed fields: alternative-specific structural choice tokens select
        // Nonterminal descent vs immediate payload entry without look-ahead.
        if let Some(alt) = self.mixed_field_choice_for_token(token).cloned() {
            return self.apply_mixed_field_choice(token, &alt);
        }
        if token == self.roles.node_start || self.roles.is_alternative_choice_token(token) {
            return self.apply_node_start(token);
        }
        if token == self.roles.node_end {
            return self.apply_node_end(token);
        }
        if token == self.roles.field_start {
            return self.apply_field_start(token);
        }
        if token == self.roles.field_end {
            return self.apply_field_end(token);
        }
        if token == self.roles.field_missing {
            return self.apply_field_missing(token);
        }
        if token == self.roles.repeated_empty {
            return self.apply_field_repeated_empty(token);
        }
        if token == self.roles.terminal {
            return self.apply_terminal_start(token);
        }
        Err(self.error(
            "unexpected_structural_token",
            format!("token {token} is not admissible for the active syntax state"),
            Some(token),
        ))
    }

    fn apply_node_start(&mut self, token: u32) -> Result<(), RawSyntaxError> {
        if self.live.stack.is_empty() {
            if token != self.roles.node_start {
                return Err(self.error(
                    "unexpected_structural_token",
                    format!(
                        "root node_start expected token {}, got {token}",
                        self.roles.node_start
                    ),
                    Some(token),
                ));
            }
            let record = self
                .facts
                .root_record()
                .map_err(|message| self.error("missing_state_metadata", message, Some(token)))?;
            let rule = record.rule.clone();
            let node_kind = record.node_kind.clone();
            let is_scope_root = self.menu_bindings.is_scope_root(&rule);
            self.live
                .stack
                .push(FrameState::from_record(record, is_scope_root));
            self.push_decoded_event(
                token,
                RawSyntaxDecodedEventKind::NodeStart,
                RawSyntaxDecodedEventContext::new(Some(rule), None, Some(node_kind), None),
                RawSyntaxDecodedPayloadMaterialization::default(),
            );
            return Ok(());
        }

        let parent_index = self.live.stack.len() - 1;
        let parent = &self.live.stack[parent_index];
        let Some(open_field) = parent.open_field.as_ref() else {
            return Err(self.error(
                "unexpected_rule_or_node_kind",
                "node_start appeared outside an open field",
                Some(token),
            ));
        };
        if open_field.missing_seen || open_field.repeated_empty_seen {
            return Err(self.error(
                "unexpected_structural_token",
                format!(
                    "field {}.{} already carries a missing or repeated-empty marker; only field_end is admissible",
                    parent.rule, open_field.name
                ),
                Some(token),
            ));
        }
        if !open_field.multiple && open_field.value_count > 0 {
            return Err(self.error(
                "repeated_field_cardinality",
                format!(
                    "field {}.{} is single-value but received another node",
                    parent.rule, open_field.name
                ),
                Some(token),
            ));
        }

        let child = self.child_for_node_start_token(&parent.rule, &open_field.name, token)?;
        if !self.facts.references.contains(&(
            parent.rule.clone(),
            open_field.name.clone(),
            child.node_kind.clone(),
        )) {
            return Err(self.error(
                "unexpected_rule_or_node_kind",
                format!(
                    "field {}.{} cannot accept node kind {}",
                    parent.rule, open_field.name, child.node_kind
                ),
                Some(token),
            ));
        }
        let record = self.facts.records.get(&child.target_rule).ok_or_else(|| {
            self.error(
                "unexpected_rule_or_node_kind",
                format!(
                    "target rule {} for node kind {} is missing from StructuralModel records",
                    child.target_rule, child.node_kind
                ),
                Some(token),
            )
        })?;
        if record.node_kind != child.node_kind {
            return Err(self.error(
                "unexpected_rule_or_node_kind",
                format!(
                    "target rule {} has node kind {}, expected {}",
                    child.target_rule, record.node_kind, child.node_kind
                ),
                Some(token),
            ));
        }

        let Some(open_field) = self.live.stack[parent_index].open_field.as_mut() else {
            return Err(self.error(
                "missing_state_metadata",
                "open field disappeared before node_start value accounting",
                Some(token),
            ));
        };
        let parent_field = open_field.name.clone();
        open_field.value_count += 1;
        let rule = record.rule.clone();
        let node_kind = record.node_kind.clone();
        let is_scope_root = self.menu_bindings.is_scope_root(&rule);
        let join_on_cmp = self.join_on_cmp_init_for_push(&rule);
        let mut frame = FrameState::from_record(record, is_scope_root);
        frame.join_on_cmp = join_on_cmp;
        self.live.stack.push(frame);
        self.push_decoded_event(
            token,
            RawSyntaxDecodedEventKind::NodeStart,
            RawSyntaxDecodedEventContext::new(
                Some(rule),
                Some(parent_field),
                Some(node_kind),
                None,
            ),
            RawSyntaxDecodedPayloadMaterialization::default(),
        );
        Ok(())
    }

    fn child_for_node_start_token(
        &self,
        owner_rule: &str,
        owner_field: &str,
        token: u32,
    ) -> Result<&ChildSpec, RawSyntaxError> {
        let children = self
            .facts
            .children_by_field
            .get(&(owner_rule.to_string(), owner_field.to_string()))
            .ok_or_else(|| {
                self.error(
                    "unexpected_rule_or_node_kind",
                    format!("field {owner_rule}.{owner_field} has no nonterminal children"),
                    Some(token),
                )
            })?;
        let multi_alternative = self
            .facts
            .multi_alternative_nonterminal_fields
            .contains(&(owner_rule.to_string(), owner_field.to_string()));
        if multi_alternative {
            // SharedAlphabet: I-DECODE-SHARED field-relative reverse (arity +
            // ordinal into A_F). PerGrammar: historical global reverse map.
            let Some(node_kind) =
                self.roles
                    .node_kind_for_field_choice_token(owner_rule, owner_field, token)
            else {
                return Err(self.shared_or_legacy_choice_resolve_error(
                    owner_rule,
                    owner_field,
                    token,
                ));
            };
            return children
                .iter()
                .find(|child| child.node_kind == node_kind)
                .ok_or_else(|| {
                    self.error(
                        "unexpected_rule_or_node_kind",
                        format!("field {owner_rule}.{owner_field} cannot accept {node_kind}"),
                        Some(token),
                    )
                });
        }
        if token != self.roles.node_start {
            return Err(self.error(
                "unexpected_structural_token",
                format!(
                    "field {owner_rule}.{owner_field} expected generic node_start token {}, got {token}",
                    self.roles.node_start
                ),
                Some(token),
            ));
        }
        if children.len() != 1 {
            return Err(self.error(
                "missing_state_metadata",
                format!(
                    "field {owner_rule}.{owner_field} has {} nonterminal children but no alternative token in raw stream",
                    children.len()
                ),
                Some(token),
            ));
        }
        Ok(&children[0])
    }

    /// Contextual error when multi-alt choice resolution fails.
    ///
    /// Under SharedAlphabet, prefer arity-mismatch / unknown-ordinal messages
    /// (I-DECODE-SHARED); otherwise the historical structural-token message.
    fn shared_or_legacy_choice_resolve_error(
        &self,
        owner_rule: &str,
        owner_field: &str,
        token: u32,
    ) -> RawSyntaxError {
        use crate::shared_alphabet_layout::SyntaxTokenLayoutMode;
        if self.roles.layout_mode == SyntaxTokenLayoutMode::SharedAlphabet {
            if let Some(row) = self.roles.field_choice_row(owner_rule, owner_field) {
                let field_arity = row.len() as u32;
                if let Some((token_arity, ordinal)) = self.roles.unpack_shared_choice_token(token) {
                    if token_arity != field_arity {
                        return self.error(
                            "shared_alphabet_choice_arity_mismatch",
                            format!(
                                "field {owner_rule}.{owner_field} has arity {field_arity} but \
                                 token {token} decodes to CHOICE({token_arity}, {ordinal}); \
                                 arity mismatch hard-rejects (I-DECODE-SHARED)"
                            ),
                            Some(token),
                        );
                    }
                    return self.error(
                        "shared_alphabet_choice_unknown_ordinal",
                        format!(
                            "field {owner_rule}.{owner_field} arity {field_arity} has no \
                             alternative at ordinal {ordinal} for token {token} \
                             (I-DECODE-SHARED)"
                        ),
                        Some(token),
                    );
                }
            }
            return self.error(
                "unexpected_structural_token",
                format!(
                    "field {owner_rule}.{owner_field} requires shared CHOICE token under \
                     SharedAlphabet, got {token}"
                ),
                Some(token),
            );
        }
        self.error(
            "unexpected_structural_token",
            format!(
                "field {owner_rule}.{owner_field} requires alternative-specific node_start token, got {token}"
            ),
            Some(token),
        )
    }

    fn apply_node_end(&mut self, token: u32) -> Result<(), RawSyntaxError> {
        let Some(frame) = self.live.stack.last() else {
            return Err(self.error(
                "unexpected_structural_token",
                "node_end appeared with no active node",
                Some(token),
            ));
        };
        if frame.open_field.is_some() {
            return Err(self.error(
                "premature_eof",
                "node_end appeared while a field is still open",
                Some(token),
            ));
        }
        if let Some(required_field) = frame
            .fields
            .iter()
            .skip(frame.field_index)
            .find(|field| field.required)
        {
            return Err(self.error(
                "missing_required_field",
                format!(
                    "required field {}.{} was not consumed",
                    frame.rule, required_field.name
                ),
                Some(token),
            ));
        }
        if self.menu_bindings.visit_all_remaining_fields()
            && let Some(field) = frame.fields.get(frame.field_index)
        {
            return Err(self.error(
                "unvisited_field",
                format!(
                    "node_end appeared before field {}.{} was consumed",
                    frame.rule, field.name
                ),
                Some(token),
            ));
        }
        let rule = frame.rule.clone();
        let node_kind = frame.node_kind.clone();
        // Structural join-ON: closing an operator alternative under a tracked
        // comparison frame records whether it is the equality op.
        let join_on_op_parent = self.menu_bindings.join_on().and_then(|constraint| {
            let parent_index = self.live.stack.len().checked_sub(2)?;
            let parent = self.live.stack.get(parent_index)?;
            let open = parent.open_field.as_ref()?;
            (parent.rule == constraint.comparison_rule
                && parent.join_on_cmp.is_some()
                && open.name == constraint.comparison_op_field)
                .then_some((parent_index, rule == constraint.eq_op_rule))
        });
        self.live.stack.pop();
        if let Some((parent_index, is_eq)) = join_on_op_parent
            && let Some(cmp) = self
                .live
                .stack
                .get_mut(parent_index)
                .and_then(|frame| frame.join_on_cmp.as_mut())
        {
            cmp.op_is_eq = Some(is_eq);
        }
        if self.live.stack.is_empty() {
            self.live.root_closed = true;
        }
        self.push_decoded_event(
            token,
            RawSyntaxDecodedEventKind::NodeEnd,
            RawSyntaxDecodedEventContext::new(Some(rule), None, Some(node_kind), None),
            RawSyntaxDecodedPayloadMaterialization::default(),
        );
        Ok(())
    }

    fn apply_field_start(&mut self, token: u32) -> Result<(), RawSyntaxError> {
        let frame_index = self.live.stack.len().checked_sub(1).ok_or_else(|| {
            self.error(
                "unexpected_field",
                "field_start appeared with no active node",
                Some(token),
            )
        })?;
        let frame = &self.live.stack[frame_index];
        if frame.open_field.is_some() {
            return Err(self.error(
                "unexpected_field",
                "field_start appeared while another field is open",
                Some(token),
            ));
        }
        let Some(expected) = frame.fields.get(frame.field_index) else {
            return Err(self.error(
                "unexpected_field",
                format!("field_start appeared after all fields for {}", frame.rule),
                Some(token),
            ));
        };
        let owner_rule = frame.rule.clone();
        let field_name = expected.name.clone();
        self.live.stack[frame_index].open_field = Some(FieldState {
            name: field_name.clone(),
            required: expected.required,
            multiple: expected.multiple,
            value_count: 0,
            missing_seen: false,
            repeated_empty_seen: false,
        });
        self.push_decoded_event(
            token,
            RawSyntaxDecodedEventKind::FieldStart,
            RawSyntaxDecodedEventContext::new(Some(owner_rule), Some(field_name), None, None),
            RawSyntaxDecodedPayloadMaterialization::default(),
        );
        Ok(())
    }

    fn apply_field_end(&mut self, token: u32) -> Result<(), RawSyntaxError> {
        let frame_index = self.live.stack.len().checked_sub(1).ok_or_else(|| {
            self.error(
                "unexpected_field",
                "field_end appeared with no active node",
                Some(token),
            )
        })?;
        let Some(field) = self.live.stack[frame_index].open_field.clone() else {
            return Err(self.error(
                "unexpected_field",
                "field_end appeared with no open field",
                Some(token),
            ));
        };
        let owner_rule = self.live.stack[frame_index].rule.clone();
        if !field.multiple && field.value_count > 1 {
            return Err(self.error(
                "repeated_field_cardinality",
                format!(
                    "single-value field {owner_rule}.{} consumed more than one value",
                    field.name
                ),
                Some(token),
            ));
        }
        if field.required && field.value_count == 0 {
            let kind = if field.multiple {
                "repeated_field_cardinality"
            } else {
                "missing_required_field"
            };
            return Err(self.error(
                kind,
                format!(
                    "required field {owner_rule}.{} consumed no value",
                    field.name
                ),
                Some(token),
            ));
        }
        if field.value_count == 0 && !field.missing_seen && !field.repeated_empty_seen {
            return Err(self.error(
                "unexpected_structural_token",
                format!(
                    "field {owner_rule}.{} closed without a value or missing/repeated-empty marker",
                    field.name
                ),
                Some(token),
            ));
        }
        let field_name = field.name.clone();
        self.live.stack[frame_index].open_field = None;
        self.live.stack[frame_index].field_index += 1;
        self.push_decoded_event(
            token,
            RawSyntaxDecodedEventKind::FieldEnd,
            RawSyntaxDecodedEventContext::new(Some(owner_rule), Some(field_name), None, None),
            RawSyntaxDecodedPayloadMaterialization::default(),
        );
        Ok(())
    }

    fn apply_field_missing(&mut self, token: u32) -> Result<(), RawSyntaxError> {
        let frame_index = self.live.stack.len().checked_sub(1).ok_or_else(|| {
            self.error(
                "unexpected_field",
                "field_missing appeared with no active node",
                Some(token),
            )
        })?;
        let Some(field) = self.live.stack[frame_index].open_field.as_ref() else {
            return Err(self.error(
                "unexpected_field",
                "field_missing appeared with no open field",
                Some(token),
            ));
        };
        if field.missing_seen || field.repeated_empty_seen {
            return Err(self.error(
                "unexpected_structural_token",
                format!(
                    "field {}.{} already carries a missing or repeated-empty marker; only field_end is admissible",
                    self.live.stack[frame_index].rule, field.name
                ),
                Some(token),
            ));
        }
        if field.required || field.value_count > 0 {
            return Err(self.error(
                "missing_required_field",
                format!(
                    "field {}.{} cannot be marked missing",
                    self.live.stack[frame_index].rule, field.name
                ),
                Some(token),
            ));
        }
        let owner_rule = self.live.stack[frame_index].rule.clone();
        let field_name = self.live.stack[frame_index]
            .open_field
            .as_ref()
            .map(|field| field.name.clone())
            .ok_or_else(|| {
                self.error(
                    "missing_state_metadata",
                    "open field disappeared before field_missing accounting",
                    Some(token),
                )
            })?;
        let Some(open_field) = self.live.stack[frame_index].open_field.as_mut() else {
            return Err(self.error(
                "missing_state_metadata",
                "open field disappeared before field_missing accounting",
                Some(token),
            ));
        };
        open_field.missing_seen = true;
        self.push_decoded_event(
            token,
            RawSyntaxDecodedEventKind::FieldMissing,
            RawSyntaxDecodedEventContext::new(Some(owner_rule), Some(field_name), None, None),
            RawSyntaxDecodedPayloadMaterialization::default(),
        );
        Ok(())
    }

    fn apply_field_repeated_empty(&mut self, token: u32) -> Result<(), RawSyntaxError> {
        let frame_index = self.live.stack.len().checked_sub(1).ok_or_else(|| {
            self.error(
                "unexpected_field",
                "field_repeated_empty appeared with no active node",
                Some(token),
            )
        })?;
        let Some(field) = self.live.stack[frame_index].open_field.as_ref() else {
            return Err(self.error(
                "unexpected_field",
                "field_repeated_empty appeared with no open field",
                Some(token),
            ));
        };
        if field.missing_seen || field.repeated_empty_seen {
            return Err(self.error(
                "unexpected_structural_token",
                format!(
                    "field {}.{} already carries a missing or repeated-empty marker; only field_end is admissible",
                    self.live.stack[frame_index].rule, field.name
                ),
                Some(token),
            ));
        }
        if !field.multiple || field.required || field.value_count > 0 {
            return Err(self.error(
                "repeated_field_cardinality",
                format!(
                    "field {}.{} cannot use repeated-empty marker",
                    self.live.stack[frame_index].rule, field.name
                ),
                Some(token),
            ));
        }
        let owner_rule = self.live.stack[frame_index].rule.clone();
        let field_name = self.live.stack[frame_index]
            .open_field
            .as_ref()
            .map(|field| field.name.clone())
            .ok_or_else(|| {
                self.error(
                    "missing_state_metadata",
                    "open field disappeared before repeated-empty accounting",
                    Some(token),
                )
            })?;
        let Some(open_field) = self.live.stack[frame_index].open_field.as_mut() else {
            return Err(self.error(
                "missing_state_metadata",
                "open field disappeared before repeated-empty accounting",
                Some(token),
            ));
        };
        open_field.repeated_empty_seen = true;
        self.push_decoded_event(
            token,
            RawSyntaxDecodedEventKind::RepeatedEmpty,
            RawSyntaxDecodedEventContext::new(Some(owner_rule), Some(field_name), None, None),
            RawSyntaxDecodedPayloadMaterialization::default(),
        );
        Ok(())
    }

    fn apply_terminal_start(&mut self, token: u32) -> Result<(), RawSyntaxError> {
        let frame_index = self.live.stack.len().checked_sub(1).ok_or_else(|| {
            self.error(
                "unexpected_rule_or_node_kind",
                "terminal appeared with no active node",
                Some(token),
            )
        })?;
        let frame = &self.live.stack[frame_index];
        let Some(field) = frame.open_field.as_ref() else {
            return Err(self.error(
                "unexpected_field",
                "terminal appeared with no open field",
                Some(token),
            ));
        };
        if field.missing_seen || field.repeated_empty_seen {
            return Err(self.error(
                "unexpected_structural_token",
                format!(
                    "field {}.{} already carries a missing or repeated-empty marker; only field_end is admissible",
                    frame.rule, field.name
                ),
                Some(token),
            ));
        }
        if !field.multiple && field.value_count > 0 {
            return Err(self.error(
                "repeated_field_cardinality",
                format!(
                    "field {}.{} is single-value but received another terminal",
                    frame.rule, field.name
                ),
                Some(token),
            ));
        }
        // Mixed fields require alternative-specific choice tokens; generic
        // `terminal` is reserved for homogeneous payload fields only.
        if self
            .facts
            .mixed_payload_fields
            .contains(&(frame.rule.clone(), field.name.clone()))
        {
            return Err(self.error(
                "unexpected_structural_token",
                format!(
                    "field {}.{} is a mixed-payload field; generic terminal is not admissible (use an alternative-specific choice token)",
                    frame.rule, field.name
                ),
                Some(token),
            ));
        }
        let payloads = self
            .facts
            .payloads_by_field
            .get(&(frame.rule.clone(), field.name.clone()))
            .cloned()
            .unwrap_or_default();
        if payloads.is_empty() {
            return Err(self.error(
                "unexpected_rule_or_node_kind",
                format!(
                    "field {}.{} cannot accept a terminal",
                    frame.rule, field.name
                ),
                Some(token),
            ));
        }
        let owner_rule = frame.rule.clone();
        let owner_field = field.name.clone();
        let payload_node_kinds = join_payload_node_kinds(&payloads);
        let payload_kinds = join_payload_kinds(&payloads);
        // Menu binding keyed by (owner_rule, owner_field); empty binding → None (**P10**).
        // Column sites resolve against live scope_tables (**P4**/**P5**).
        let menu_cardinality = self.resolve_menu_cardinality(&owner_rule, &owner_field, token)?;
        self.live.payload = Some(PayloadState {
            owner_frame_index: frame_index,
            owner_rule: owner_rule.clone(),
            owner_field: owner_field.clone(),
            candidates: payloads,
            bytes: Vec::new(),
            byte_count: 0,
            menu_cardinality,
            menu_width: None,
            menu_digits_seen: 0,
            menu_partial: 0,
            menu_index: None,
        });
        self.push_decoded_event(
            token,
            RawSyntaxDecodedEventKind::TerminalStart,
            RawSyntaxDecodedEventContext::new(
                Some(owner_rule),
                Some(owner_field),
                Some(payload_node_kinds),
                Some(payload_kinds),
            ),
            RawSyntaxDecodedPayloadMaterialization::default(),
        );
        Ok(())
    }

    fn apply_payload_token(&mut self, token: u32) -> Result<(), RawSyntaxError> {
        let Some(payload) = self.live.payload.as_ref() else {
            return Err(self.error(
                "missing_state_metadata",
                "payload token appeared without active payload state",
                Some(token),
            ));
        };

        // Menu-index payload branch (**P4**): never touches payload.bytes or the
        // shared u8/UTF-8 ranges (**P9**). Unreachable when menu_cardinality is None (**P10**).
        if let Some(cardinality) = payload.menu_cardinality {
            return self.apply_menu_payload_token(token, cardinality);
        }

        // Re-borrow mutably after the menu early-return (payload still present).
        let Some(payload) = self.live.payload.as_mut() else {
            return Err(self.error(
                "missing_state_metadata",
                "payload token appeared without active payload state",
                Some(token),
            ));
        };

        let boolean_payload = payload
            .candidates
            .iter()
            .all(|candidate| is_boolean_payload_kind(&candidate.payload_kind));
        if boolean_payload {
            if token != self.roles.boolean_false && token != self.roles.boolean_true {
                return Err(self.error(
                    "invalid_payload_token",
                    format!("boolean payload token {token} is not a valid finite boolean token"),
                    Some(token),
                ));
            }
            let owner_rule = payload.owner_rule.clone();
            let owner_field = payload.owner_field.clone();
            let node_kind = join_payload_node_kinds(&payload.candidates);
            let payload_kind = join_payload_kinds(&payload.candidates);
            let payload_value = if token == self.roles.boolean_true {
                "true"
            } else {
                "false"
            };
            self.finish_payload_value(token)?;
            self.push_decoded_event(
                token,
                RawSyntaxDecodedEventKind::BooleanPayload,
                RawSyntaxDecodedEventContext::new(
                    Some(owner_rule),
                    Some(owner_field),
                    Some(node_kind),
                    Some(payload_kind),
                ),
                RawSyntaxDecodedPayloadMaterialization {
                    payload_byte_count: 1,
                    payload_token_byte: None,
                    payload_value: Some(payload_value.to_string()),
                },
            );
            return Ok(());
        }

        if token == self.roles.payload_end {
            let owner_rule = payload.owner_rule.clone();
            let owner_field = payload.owner_field.clone();
            let node_kind = join_payload_node_kinds(&payload.candidates);
            let payload_kind = join_payload_kinds(&payload.candidates);
            let payload_byte_count = payload.byte_count;
            let all_integer = payload
                .candidates
                .iter()
                .all(|candidate| is_integer_payload_kind(&candidate.payload_kind));
            let payload_value = if all_integer && is_u8_integer_range(self.roles) {
                // Variable-width integer on the 256 band (canonical escape form).
                // Materialize decimal so high digits round-trip without UTF-8.
                if !super::integer_codec::integer_digits_complete(&payload.bytes) {
                    let observed = payload.bytes.len();
                    return Err(self.error(
                        "invalid_payload_continuation",
                        format!(
                            "integer payload incomplete at payload_end ({observed} digit(s); need full canonical form)"
                        ),
                        Some(token),
                    ));
                }
                let value = super::integer_codec::decode_integer_digits(&payload.bytes)
                    .map_err(|msg| self.error("invalid_payload_continuation", msg, Some(token)))?;
                value.to_string()
            } else {
                String::from_utf8(payload.bytes.clone()).map_err(|error| {
                    self.error(
                        "invalid_payload_continuation",
                        format!("payload bytes are not valid UTF-8: {error}"),
                        Some(token),
                    )
                })?
            };
            self.finish_payload_value(token)?;
            self.push_decoded_event(
                token,
                RawSyntaxDecodedEventKind::PayloadEnd,
                RawSyntaxDecodedEventContext::new(
                    Some(owner_rule),
                    Some(owner_field),
                    Some(node_kind),
                    Some(payload_kind),
                ),
                RawSyntaxDecodedPayloadMaterialization {
                    payload_byte_count,
                    payload_token_byte: None,
                    payload_value: Some(payload_value),
                },
            );
            return Ok(());
        }

        // Validate before mutating: a rejected payload byte must leave the
        // candidate set and byte buffer untouched so incremental `advance`
        // stays error-atomic. `roles` is copied out as the engine's borrowed
        // reference (no clone) so the retain closure does not capture `self`.
        let roles = self.roles;
        let all_integer = payload
            .candidates
            .iter()
            .all(|candidate| is_integer_payload_kind(&candidate.payload_kind));
        if all_integer && is_u8_integer_range(roles) {
            // Variable-width integer: reject further value digits once complete;
            // reject non-canonical continuations before mutating state.
            if super::integer_codec::integer_digits_complete(&payload.bytes) {
                let observed = payload.byte_count;
                return Err(self.error(
                    "invalid_payload_token",
                    format!(
                        "integer payload already complete with {observed} digit(s); only payload_end is legal"
                    ),
                    Some(token),
                ));
            }
            let Some(payload_byte) =
                payload_token_byte_for_candidates(&payload.candidates, token, roles)
            else {
                return Err(self.error(
                    "invalid_payload_token",
                    format!("payload token {token} is outside every active payload range"),
                    Some(token),
                ));
            };
            if !super::integer_codec::integer_digit_admissible(&payload.bytes, payload_byte) {
                let prefix = payload.bytes.clone();
                return Err(self.error(
                    "invalid_payload_token",
                    format!(
                        "non-canonical integer digit {payload_byte} after {prefix:?}; short-form-expressible or over-length continuation"
                    ),
                    Some(token),
                ));
            }
            // Fall through to shared retain/push path below with payload_byte set.
            // A payload byte was resolved above, so at least one candidate range
            // contains the token and the narrowed candidate set is non-empty.
            payload.candidates.retain(|candidate| {
                payload_token_range_for_kind(&candidate.payload_kind, roles)
                    .is_some_and(|range| token >= range.start && token < range.end_exclusive)
            });
            debug_assert!(
                !payload.candidates.is_empty(),
                "payload_token_byte_for_candidates implies a matching candidate range"
            );
            payload.bytes.push(payload_byte);
            payload.byte_count += 1;
            let owner_rule = payload.owner_rule.clone();
            let owner_field = payload.owner_field.clone();
            let node_kind = join_payload_node_kinds(&payload.candidates);
            let payload_kind = join_payload_kinds(&payload.candidates);
            let payload_byte_count = payload.byte_count;
            self.push_decoded_event(
                token,
                RawSyntaxDecodedEventKind::PayloadToken,
                RawSyntaxDecodedEventContext::new(
                    Some(owner_rule),
                    Some(owner_field),
                    Some(node_kind),
                    Some(payload_kind),
                ),
                RawSyntaxDecodedPayloadMaterialization {
                    payload_byte_count,
                    payload_token_byte: Some(payload_byte),
                    payload_value: None,
                },
            );
            return Ok(());
        }
        let Some(payload_byte) =
            payload_token_byte_for_candidates(&payload.candidates, token, roles)
        else {
            return Err(self.error(
                "invalid_payload_token",
                format!("payload token {token} is outside every active payload range"),
                Some(token),
            ));
        };
        // A payload byte was resolved above, so at least one candidate range
        // contains the token and the narrowed candidate set is non-empty.
        payload.candidates.retain(|candidate| {
            payload_token_range_for_kind(&candidate.payload_kind, roles)
                .is_some_and(|range| token >= range.start && token < range.end_exclusive)
        });
        payload.bytes.push(payload_byte);
        payload.byte_count += 1;
        let owner_rule = payload.owner_rule.clone();
        let owner_field = payload.owner_field.clone();
        let node_kind = join_payload_node_kinds(&payload.candidates);
        let payload_kind = join_payload_kinds(&payload.candidates);
        let payload_byte_count = payload.byte_count;
        self.push_decoded_event(
            token,
            RawSyntaxDecodedEventKind::PayloadToken,
            RawSyntaxDecodedEventContext::new(
                Some(owner_rule),
                Some(owner_field),
                Some(node_kind),
                Some(payload_kind),
            ),
            RawSyntaxDecodedPayloadMaterialization {
                payload_byte_count,
                payload_token_byte: Some(payload_byte),
                payload_value: None,
            },
        );
        Ok(())
    }

    /// Accept menu-band digit token(s) forming a complete ordinal, then
    /// `payload_end` (**P4**). Width is per-site from live cardinality (W=1
    /// when N ≤ band width; W>1 when `max_menu_digits` permits).
    fn apply_menu_payload_token(
        &mut self,
        token: u32,
        cardinality: u32,
    ) -> Result<(), RawSyntaxError> {
        let Some(menu_range) = self.roles.menu_range.as_ref() else {
            return Err(self.error(
                "missing_state_metadata",
                "menu payload token appeared without menu_index_payload layout role",
                Some(token),
            ));
        };
        let menu_start = menu_range.start;
        let menu_end = menu_range.end_exclusive;

        let Some(payload) = self.live.payload.as_ref() else {
            return Err(self.error(
                "missing_state_metadata",
                "menu payload token appeared without active payload state",
                Some(token),
            ));
        };
        let owner_rule = payload.owner_rule.clone();
        let owner_field = payload.owner_field.clone();
        let node_kind = join_payload_node_kinds(&payload.candidates);
        let payload_kind = join_payload_kinds(&payload.candidates);

        if token == self.roles.payload_end {
            // payload_end only after a *complete* ordinal (menu_index set).
            let Some(ordinal) = payload.menu_index else {
                return Err(self.error(
                    "invalid_payload_continuation",
                    "menu payload_end requires a complete menu ordinal",
                    Some(token),
                ));
            };
            let payload_value = ordinal.to_string();
            self.finish_payload_value(token)?;
            self.push_decoded_event(
                token,
                RawSyntaxDecodedEventKind::PayloadEnd,
                RawSyntaxDecodedEventContext::new(
                    Some(owner_rule),
                    Some(owner_field),
                    Some(node_kind),
                    Some(payload_kind),
                ),
                RawSyntaxDecodedPayloadMaterialization {
                    payload_byte_count: 1,
                    payload_token_byte: None,
                    payload_value: Some(payload_value),
                },
            );
            return Ok(());
        }

        // Complete ordinal already formed: only payload_end is legal.
        if payload.menu_index.is_some() {
            return Err(self.error(
                "invalid_payload_token",
                "menu payload already has a complete menu ordinal; only payload_end is legal",
                Some(token),
            ));
        }

        if token < menu_start || token >= menu_end {
            return Err(self.error(
                "invalid_payload_token",
                format!("menu payload token {token} is outside the menu_index_payload band"),
                Some(token),
            ));
        }
        // Mask and apply share menu_digit_* so mask==accept holds by construction.
        let base = menu_end.saturating_sub(menu_start);
        let digits_seen = payload.menu_digits_seen;
        let partial = payload.menu_partial;
        let digit = token - menu_start;

        // Factored column addressing: two positional digits (catalog table,
        // then column inside that table) rather than a radix code over the
        // flat scope concatenation. The digit filters live in
        // `factored_menu_codec` and are shared with `admissible_next_raw_tokens` and
        // `emit_factored_menu_index_payload`, so mask == accept == encode holds
        // by construction. Dead without a factored binding (**P10**).
        if self
            .menu_bindings
            .is_factored_column_site(&owner_rule, &owner_field)
        {
            let (new_digits_seen, new_partial, completed) = self.factored_menu_digit_step(
                digit,
                digits_seen,
                partial,
                &owner_rule,
                &owner_field,
                token,
            )?;
            let Some(payload) = self.live.payload.as_mut() else {
                return Err(self.error(
                    "missing_state_metadata",
                    "menu payload token appeared without active payload state",
                    Some(token),
                ));
            };
            payload.menu_width = Some(FACTORED_COLUMN_MENU_DIGITS);
            payload.menu_digits_seen = new_digits_seen;
            payload.menu_partial = new_partial;
            payload.menu_index = completed;
            payload.byte_count = new_digits_seen as usize;
            self.push_decoded_event(
                token,
                RawSyntaxDecodedEventKind::PayloadToken,
                RawSyntaxDecodedEventContext::new(
                    Some(owner_rule),
                    Some(owner_field),
                    Some(node_kind),
                    Some(payload_kind),
                ),
                RawSyntaxDecodedPayloadMaterialization {
                    payload_byte_count: new_digits_seen as usize,
                    payload_token_byte: None,
                    payload_value: None,
                },
            );
            return Ok(());
        }
        // Width cap is defensive only under a healthy binding: bind time
        // (`CompiledSyntaxModel::with_menu_bindings`) rejects sites whose band
        // limit exceeds `band_width.pow(max_menu_digits)`, so
        // `menu_digit_width(N, base) <= max_menu_digits` for live N. Encode
        // (`emit_menu_index_payload`) independently fail-closes on the same
        // capacity slip so this `.min(cap)` is a no-op for valid models.
        let width = payload.menu_width.unwrap_or_else(|| {
            let computed = menu_digit_width(cardinality, base);
            let cap = self.menu_bindings.max_menu_digits.max(1);
            computed.min(cap)
        });
        if !menu_digit_admissible(digit, cardinality, base, width, digits_seen, partial, None) {
            return Err(self.error(
                "invalid_payload_token",
                format!(
                    "menu digit {digit} (partial={partial}, digits_seen={digits_seen}, width={width}) is out of bound menu [0, {cardinality}) for {owner_rule}.{owner_field}"
                ),
                Some(token),
            ));
        }
        let new_partial_u64 = (partial as u64) * (base as u64) + (digit as u64);
        let new_partial = u32::try_from(new_partial_u64).unwrap_or(u32::MAX);
        let new_digits_seen = digits_seen.saturating_add(1);
        let completes = new_digits_seen >= width;
        // Pre-M6: reject table digits with no surviving residual completion
        // outside current scope (at W=1 this is exact-ordinal exclusion).
        if matches!(
            self.menu_bindings.site_kind(&owner_rule, &owner_field),
            Some(crate::syntax_state_machine::MenuSiteKind::Fixed {
                pushes_scope: true,
                ..
            })
        ) {
            let scope = self.current_scope_tables();
            if !menu_digit_admissible(
                digit,
                cardinality,
                base,
                width,
                digits_seen,
                partial,
                Some(scope.as_slice()),
            ) {
                let message = if completes {
                    format!(
                        "table menu ordinal {new_partial} is already in scope; repeated table ordinals are rejected until occurrence aliases (M6)"
                    )
                } else {
                    format!(
                        "table menu digit {digit} has no surviving residual completion outside scope for {owner_rule}.{owner_field}"
                    )
                };
                return Err(self.error("menu_duplicate_scope_table", message, Some(token)));
            }
        }
        // Structural join-ON (**join-key masking**): the RIGHT side of an
        // equality comparison under a join-ON field must pair the just-joined
        // table with an earlier in-scope table. Same digit filter as the mask
        // so mask==accept holds by construction.
        if let Some(allowed) = self.join_on_allowed_column_range()
            && !menu_digit_admissible_in_range(
                digit,
                cardinality,
                base,
                width,
                digits_seen,
                partial,
                Some(allowed),
            )
        {
            return Err(self.error(
                    "join_on_structural_violation",
                    format!(
                        "join-ON column digit {digit} (partial={partial}, digits_seen={digits_seen}) violates the structural join constraint for {owner_rule}.{owner_field}: an equality join key must pair one column of the just-joined table with one column of an earlier in-scope table (allowed scope-relative ordinals [{}, {}))",
                        allowed.0, allowed.1
                    ),
                    Some(token),
                ));
        }

        let Some(payload) = self.live.payload.as_mut() else {
            return Err(self.error(
                "missing_state_metadata",
                "menu payload token appeared without active payload state",
                Some(token),
            ));
        };
        payload.menu_width = Some(width);
        payload.menu_digits_seen = new_digits_seen;
        payload.menu_partial = new_partial;
        // menu_index means *complete* ordinal only; intermediate digits leave it None.
        if completes {
            payload.menu_index = Some(new_partial);
        }
        // byte_count tracks materialization progress (digits accepted so far).
        payload.byte_count = new_digits_seen as usize;
        self.push_decoded_event(
            token,
            RawSyntaxDecodedEventKind::PayloadToken,
            RawSyntaxDecodedEventContext::new(
                Some(owner_rule),
                Some(owner_field),
                Some(node_kind),
                Some(payload_kind),
            ),
            RawSyntaxDecodedPayloadMaterialization {
                payload_byte_count: new_digits_seen as usize,
                payload_token_byte: None,
                payload_value: None,
            },
        );
        Ok(())
    }

    fn finish_payload_value(&mut self, token: u32) -> Result<(), RawSyntaxError> {
        // Validate every completion precondition before clearing the payload
        // state: a rejected completion must leave the machine untouched so
        // incremental `advance` stays error-atomic.
        let Some(payload) = self.live.payload.as_ref() else {
            return Err(self.error(
                "missing_state_metadata",
                "payload completion appeared without active payload state",
                Some(token),
            ));
        };
        let Some(frame) = self.live.stack.get_mut(payload.owner_frame_index) else {
            return Err(self.error(
                "missing_state_metadata",
                "payload owner frame disappeared before terminal value accounting",
                Some(token),
            ));
        };
        let Some(open_field) = frame.open_field.as_mut() else {
            return Err(self.error(
                "missing_state_metadata",
                "open field disappeared before terminal value accounting",
                Some(token),
            ));
        };
        if open_field.name != payload.owner_field || frame.rule != payload.owner_rule {
            return Err(self.error(
                "missing_state_metadata",
                "active field changed before terminal value accounting",
                Some(token),
            ));
        }
        open_field.value_count += 1;
        // Table menu completion pushes onto scope before clearing payload (**P5**).
        let push_scope_ordinal = match (
            payload.menu_index,
            self.menu_bindings
                .site_kind(&payload.owner_rule, &payload.owner_field),
        ) {
            (
                Some(ordinal),
                Some(crate::syntax_state_machine::MenuSiteKind::Fixed {
                    pushes_scope: true, ..
                }),
            ) => Some(ordinal),
            _ => None,
        };
        // Structural join-ON: a completed LEFT-side plain column records its
        // catalog table on the comparison frame so the right-side column menu
        // can be pair-constrained.
        let join_on_left = match (payload.menu_index, self.menu_bindings.join_on()) {
            (Some(ordinal), Some(constraint))
                if payload.owner_rule == constraint.column_rule
                    && payload.owner_field == constraint.column_field =>
            {
                self.join_on_comparison_ancestor(payload.owner_frame_index, constraint)
                    .and_then(|parent_index| {
                        let parent = self.live.stack.get(parent_index)?;
                        let open = parent.open_field.as_ref()?;
                        (parent.join_on_cmp.is_some()
                            && open.name == constraint.comparison_left_field)
                            .then_some(parent_index)
                    })
                    .and_then(|parent_index| {
                        let columns_per_table = self.menu_bindings.column_site_columns_per_table(
                            &payload.owner_rule,
                            &payload.owner_field,
                        )?;
                        let scope = self.current_scope_tables();
                        let mut prefix = 0u32;
                        for &table in &scope {
                            let count = columns_per_table.get(table as usize).copied().unwrap_or(0);
                            if ordinal < prefix.saturating_add(count) {
                                return Some((parent_index, table));
                            }
                            prefix = prefix.saturating_add(count);
                        }
                        None
                    })
            }
            _ => None,
        };
        self.live.payload = None;
        if let Some(ordinal) = push_scope_ordinal {
            self.push_scope_table(ordinal, token)?;
        }
        if let Some((parent_index, table)) = join_on_left
            && let Some(cmp) = self
                .live
                .stack
                .get_mut(parent_index)
                .and_then(|frame| frame.join_on_cmp.as_mut())
        {
            cmp.left_table = Some(table);
        }
        Ok(())
    }

    fn state(
        &self,
        status: RawSyntaxStatus,
        total_raw_tokens: usize,
        diagnostic: &str,
    ) -> RawSyntaxState {
        RawSyntaxState {
            status,
            consumed_raw_tokens: self.live.consumed,
            total_raw_tokens,
            eof_valid: self.eof_valid(),
            stack_depth: self.live.stack.len(),
            active_rule: self.active_rule(),
            active_field: self.active_field(),
            active_payload_kind: self.active_payload_kind(),
            active_payload_node_kind: self.active_payload_node_kind(),
            active_payload_byte_count: self
                .live
                .payload
                .as_ref()
                .map_or(0, |payload| payload.byte_count),
            admissible: self.admissible_next_raw_tokens(),
            diagnostic: diagnostic.to_string(),
        }
    }

    pub(super) fn admissible_next_raw_tokens(&self) -> RawSyntaxAdmissibleTokens {
        let mut finite = BTreeSet::new();
        let mut ranges = BTreeMap::new();
        let mut payload_end_admissible = false;

        if self.live.root_closed {
            return RawSyntaxAdmissibleTokens {
                finite_raw_token_ids: Vec::new(),
                raw_token_ranges: Vec::new(),
                payload_end_admissible,
            };
        }

        if let Some(payload) = self.live.payload.as_ref() {
            // Menu-index mask (**P4**): admit exactly the bound catalog slice of
            // the menu band, then payload_end. Unreachable without binding (**P10**).
            // Scope-pushing table sites exclude ordinals already in the nearest
            // scope root so generated streams cannot enter a non-renderable
            // repeated-table state before occurrence aliases (M6).
            if let Some(cardinality) = payload.menu_cardinality {
                // Incomplete multi-digit ordinal: admit next digit band slice.
                // Complete ordinal (menu_index set): only payload_end.
                if payload.menu_index.is_none() {
                    if let Some(menu_range) = self.roles.menu_range.as_ref() {
                        // Same menu_digit_* helpers as apply_menu_payload_token.
                        // `.min(cap)` is defensive; see apply_menu_payload_token
                        // for the bind-time + encode capacity invariant.
                        let base = menu_range.end_exclusive.saturating_sub(menu_range.start);
                        let digits_seen = payload.menu_digits_seen;
                        let partial = payload.menu_partial;
                        let width = payload.menu_width.unwrap_or_else(|| {
                            let computed = menu_digit_width(cardinality, base);
                            let cap = self.menu_bindings.max_menu_digits.max(1);
                            computed.min(cap)
                        });
                        let end_digit =
                            menu_digit_end(cardinality, base, width, digits_seen, partial);
                        let pushes_scope = matches!(
                            self.menu_bindings
                                .site_kind(&payload.owner_rule, &payload.owner_field),
                            Some(crate::syntax_state_machine::MenuSiteKind::Fixed {
                                pushes_scope: true,
                                ..
                            })
                        );
                        if self
                            .menu_bindings
                            .is_factored_column_site(&payload.owner_rule, &payload.owner_field)
                        {
                            // Factored column address: digit 0 ranges over the
                            // in-scope catalog tables, digit 1 over the chosen
                            // table's columns. Same helpers as apply/encode.
                            let columns_per_table = self
                                .menu_bindings
                                .column_site_columns_per_table(
                                    &payload.owner_rule,
                                    &payload.owner_field,
                                )
                                .unwrap_or(&[]);
                            let scope = self.current_scope_tables();
                            let allowed = self.join_on_allowed_column_range();
                            let candidates: Vec<u32> = if payload.menu_digits_seen == 0 {
                                scope
                                    .iter()
                                    .copied()
                                    .filter(|&table| {
                                        factored_table_digit_admissible(
                                            table,
                                            &scope,
                                            columns_per_table,
                                            allowed,
                                        )
                                    })
                                    .collect()
                            } else {
                                let table = payload.menu_partial;
                                let count = factored_table_slice(&scope, columns_per_table, table)
                                    .map_or(0, |(_, count)| count);
                                (0..count)
                                    .filter(|&column| {
                                        factored_column_digit_admissible(
                                            column,
                                            table,
                                            &scope,
                                            columns_per_table,
                                            allowed,
                                        )
                                    })
                                    .collect()
                            };
                            for digit in candidates {
                                let candidate = menu_range.start.saturating_add(digit);
                                if candidate < menu_range.end_exclusive {
                                    finite.insert(candidate);
                                }
                            }
                        } else if pushes_scope {
                            let scope = self.current_scope_tables();
                            for digit in 0..end_digit {
                                if menu_digit_admissible(
                                    digit,
                                    cardinality,
                                    base,
                                    width,
                                    digits_seen,
                                    partial,
                                    Some(scope.as_slice()),
                                ) {
                                    finite.insert(menu_range.start.saturating_add(digit));
                                }
                            }
                        } else if let Some(allowed) = self.join_on_allowed_column_range() {
                            // Structural join-ON right-side column: admit only
                            // digits whose residual completions intersect the
                            // allowed pair range (same filter as apply).
                            for digit in 0..end_digit {
                                if menu_digit_admissible_in_range(
                                    digit,
                                    cardinality,
                                    base,
                                    width,
                                    digits_seen,
                                    partial,
                                    Some(allowed),
                                ) {
                                    finite.insert(menu_range.start.saturating_add(digit));
                                }
                            }
                        } else if end_digit > 0 {
                            let end = menu_range.start.saturating_add(end_digit);
                            ranges.insert(
                                menu_range.role.clone(),
                                ProjectionTokenLayoutRangeSummary {
                                    id: menu_range.id.clone(),
                                    role: menu_range.role.clone(),
                                    start: menu_range.start,
                                    end_exclusive: end,
                                },
                            );
                        }
                    }
                } else {
                    finite.insert(self.roles.payload_end);
                    payload_end_admissible = true;
                }
                return RawSyntaxAdmissibleTokens {
                    finite_raw_token_ids: finite.into_iter().collect(),
                    raw_token_ranges: ranges.into_values().collect(),
                    payload_end_admissible,
                };
            }
            if payload
                .candidates
                .iter()
                .all(|candidate| is_boolean_payload_kind(&candidate.payload_kind))
            {
                finite.insert(self.roles.boolean_false);
                finite.insert(self.roles.boolean_true);
            } else {
                let all_integer = payload
                    .candidates
                    .iter()
                    .all(|candidate| is_integer_payload_kind(&candidate.payload_kind));
                if all_integer && is_u8_integer_range(self.roles) {
                    // Variable-width integer on the 256 band: first digit admits
                    // the full band; continuations admit only canonical digits;
                    // after a complete form only payload_end is legal.
                    if super::integer_codec::integer_digits_complete(&payload.bytes) {
                        finite.insert(self.roles.payload_end);
                        payload_end_admissible = true;
                    } else if payload.bytes.is_empty() {
                        // Any first digit 0..=255 (markers 254/255 included).
                        if let Some(range) =
                            payload_token_range_for_kind("integer_payload", self.roles)
                        {
                            ranges.entry(range.role.clone()).or_insert(range);
                        }
                    } else if let Some(int_range) =
                        payload_token_range_for_kind("integer_payload", self.roles)
                    {
                        // Sparse or dense continuation mask from canonical rules.
                        for digit in 0u8..=255 {
                            if super::integer_codec::integer_digit_admissible(&payload.bytes, digit)
                            {
                                finite.insert(int_range.start.saturating_add(u32::from(digit)));
                            }
                        }
                    }
                } else {
                    // Variable-length UTF-8 / integer-lexeme payloads: payload_end
                    // is admissible only when the accumulated bytes already form
                    // valid UTF-8. apply_payload_token rejects the terminator
                    // otherwise, and transition acceptance must equal mask membership.
                    if std::str::from_utf8(&payload.bytes).is_ok() {
                        finite.insert(self.roles.payload_end);
                        payload_end_admissible = true;
                    }
                    for candidate in &payload.candidates {
                        if let Some(range) =
                            payload_token_range_for_kind(&candidate.payload_kind, self.roles)
                        {
                            ranges.entry(range.role.clone()).or_insert(range);
                        }
                    }
                }
            }
            return RawSyntaxAdmissibleTokens {
                finite_raw_token_ids: finite.into_iter().collect(),
                raw_token_ranges: ranges.into_values().collect(),
                payload_end_admissible,
            };
        }

        let Some(frame) = self.live.stack.last() else {
            finite.insert(self.roles.node_start);
            return RawSyntaxAdmissibleTokens {
                finite_raw_token_ids: finite.into_iter().collect(),
                raw_token_ranges: Vec::new(),
                payload_end_admissible,
            };
        };
        if let Some(field) = frame.open_field.as_ref() {
            self.collect_open_field_admissible_tokens(frame, field, &mut finite);
        } else {
            let has_remaining = frame.fields.get(frame.field_index).is_some();
            if has_remaining {
                finite.insert(self.roles.field_start);
            }
            let remaining_required = frame
                .fields
                .iter()
                .skip(frame.field_index)
                .any(|field| field.required);
            let admit_node_end = if self.menu_bindings.visit_all_remaining_fields() {
                !has_remaining
            } else {
                !remaining_required
            };
            if admit_node_end {
                finite.insert(self.roles.node_end);
            }
        }

        RawSyntaxAdmissibleTokens {
            finite_raw_token_ids: finite.into_iter().collect(),
            raw_token_ranges: Vec::new(),
            payload_end_admissible,
        }
    }

    fn collect_open_field_admissible_tokens(
        &self,
        frame: &FrameState,
        field: &FieldState,
        tokens: &mut BTreeSet<u32>,
    ) {
        use crate::shared_alphabet_layout::SyntaxTokenLayoutMode;

        if field.missing_seen || field.repeated_empty_seen {
            tokens.insert(self.roles.field_end);
            return;
        }
        if field.value_count > 0 {
            tokens.insert(self.roles.field_end);
            if !field.multiple {
                return;
            }
        }
        if field.value_count == 0 && !field.required {
            tokens.insert(self.roles.field_missing);
            if field.multiple {
                tokens.insert(self.roles.repeated_empty);
            }
        }

        let field_key = (frame.rule.clone(), field.name.clone());
        let shared_alphabet = self.roles.layout_mode == SyntaxTokenLayoutMode::SharedAlphabet;
        // Mixed fields: only alternative-specific structural choice tokens.
        // Never admit generic `terminal` or raw payload bytes before a choice.
        if self.facts.mixed_payload_fields.contains(&field_key) {
            if let Some(row) = self.roles.field_choice_row(&frame.rule, &field.name) {
                // SharedAlphabet field-relative row (exact CHOICE(n,*)).
                for alt in row {
                    tokens.insert(alt.token_id);
                }
            } else if let Some(alts) = self.facts.alternatives_by_field.get(&field_key) {
                // Decision 3 (I-DECODE-SHARED): under SharedAlphabet the residual
                // global first-wins map is not a sound reverse identity without the
                // active (rule, field). Missing field-relative row → admit nothing
                // (fail closed). PerGrammar keeps the historical global-map fallback.
                if !shared_alphabet {
                    for alt in alts {
                        if let Some(token) =
                            self.roles.node_start_alternative_tokens.get(&alt.node_kind)
                        {
                            tokens.insert(*token);
                        }
                    }
                }
            }
            return;
        }

        if self.facts.payloads_by_field.contains_key(&field_key) {
            tokens.insert(self.roles.terminal);
        }
        let multi_alternative = self
            .facts
            .multi_alternative_nonterminal_fields
            .contains(&field_key);
        if multi_alternative
            && let Some(row) = self.roles.field_choice_row(&frame.rule, &field.name)
        {
            for alt in row {
                tokens.insert(alt.token_id);
            }
            return;
        }
        if let Some(children) = self.facts.children_by_field.get(&field_key) {
            for child in children {
                if multi_alternative {
                    // Decision 3 (I-DECODE-SHARED): under SharedAlphabet the residual
                    // global first-wins map is not a sound reverse identity without the
                    // active (rule, field). Missing field-relative row → admit nothing
                    // (fail closed). PerGrammar keeps the historical global-map fallback.
                    if shared_alphabet {
                        continue;
                    }
                    if let Some(token) = self
                        .roles
                        .node_start_alternative_tokens
                        .get(&child.node_kind)
                    {
                        tokens.insert(*token);
                    }
                } else {
                    tokens.insert(self.roles.node_start);
                }
            }
        }
    }

    /// When the open field is mixed-payload, map a structural choice token to
    /// its discriminated alternative. Returns `None` when the active field is
    /// not mixed or the token is not one of that field's choice tokens.
    fn mixed_field_choice_for_token(&self, token: u32) -> Option<&DiscriminatedAlternative> {
        // Homogeneous grammars (Go, C#, tree-sitter JSON/C subsets) leave this
        // set empty; skip all per-token key construction and lookups.
        if self.facts.mixed_payload_fields.is_empty() {
            return None;
        }
        let frame = self.live.stack.last()?;
        let field = frame.open_field.as_ref()?;
        if field.missing_seen || field.repeated_empty_seen {
            return None;
        }
        if !field.multiple && field.value_count > 0 {
            return None;
        }
        if !self
            .facts
            .mixed_payload_fields
            .contains(&(frame.rule.clone(), field.name.clone()))
        {
            return None;
        }
        // SharedAlphabet: field-relative reverse (I-DECODE-SHARED). PerGrammar:
        // historical global reverse map, then mixed alternative lookup.
        let node_kind =
            self.roles
                .node_kind_for_field_choice_token(&frame.rule, &field.name, token)?;
        self.facts
            .mixed_field_alternative(&frame.rule, &field.name, node_kind)
    }

    fn apply_mixed_field_choice(
        &mut self,
        token: u32,
        alt: &DiscriminatedAlternative,
    ) -> Result<(), RawSyntaxError> {
        match alt.body_kind {
            AlternativeBodyKind::ChildRuleDescent => self.apply_node_start(token),
            AlternativeBodyKind::BooleanPayload => self.apply_mixed_boolean_choice(token, alt),
            AlternativeBodyKind::IntegerPayload | AlternativeBodyKind::Utf8Payload => {
                self.apply_mixed_payload_choice(token, alt)
            }
        }
    }

    /// Choice token alone completes a BooleanPayload: zero payload bytes,
    /// no second finite boolean token, no look-ahead.
    fn apply_mixed_boolean_choice(
        &mut self,
        token: u32,
        alt: &DiscriminatedAlternative,
    ) -> Result<(), RawSyntaxError> {
        let frame_index = self.live.stack.len().checked_sub(1).ok_or_else(|| {
            self.error(
                "unexpected_rule_or_node_kind",
                "mixed boolean choice appeared with no active node",
                Some(token),
            )
        })?;
        let frame = &self.live.stack[frame_index];
        let Some(field) = frame.open_field.as_ref() else {
            return Err(self.error(
                "unexpected_field",
                "mixed boolean choice appeared with no open field",
                Some(token),
            ));
        };
        if field.missing_seen || field.repeated_empty_seen {
            return Err(self.error(
                "unexpected_structural_token",
                format!(
                    "field {}.{} already carries a missing or repeated-empty marker; only field_end is admissible",
                    frame.rule, field.name
                ),
                Some(token),
            ));
        }
        if !field.multiple && field.value_count > 0 {
            return Err(self.error(
                "repeated_field_cardinality",
                format!(
                    "field {}.{} is single-value but received another value",
                    frame.rule, field.name
                ),
                Some(token),
            ));
        }
        let owner_rule = frame.rule.clone();
        let owner_field = field.name.clone();
        let node_kind = alt.node_kind.clone();
        let payload_kind = alt
            .payload_kind
            .clone()
            .unwrap_or_else(|| "boolean".to_string());
        let payload_value = boolean_value_for_explicit_identity(&node_kind)
            .ok_or_else(|| {
                self.error(
                    "invalid_boolean_identity",
                    format!(
                        "mixed BooleanPayload node_kind {node_kind:?} is not an explicit true/false identity"
                    ),
                    Some(token),
                )
            })?
            .to_string();

        // TerminalStart then BooleanPayload on the same choice token so the
        // decoded-tree builder sees a complete terminal leaf without any
        // additional raw token.
        self.push_decoded_event(
            token,
            RawSyntaxDecodedEventKind::TerminalStart,
            RawSyntaxDecodedEventContext::new(
                Some(owner_rule.clone()),
                Some(owner_field.clone()),
                Some(node_kind.clone()),
                Some(payload_kind.clone()),
            ),
            RawSyntaxDecodedPayloadMaterialization::default(),
        );
        let Some(open_field) = self.live.stack[frame_index].open_field.as_mut() else {
            return Err(self.error(
                "missing_state_metadata",
                "open field disappeared before mixed boolean accounting",
                Some(token),
            ));
        };
        open_field.value_count += 1;
        self.push_decoded_event(
            token,
            RawSyntaxDecodedEventKind::BooleanPayload,
            RawSyntaxDecodedEventContext::new(
                Some(owner_rule),
                Some(owner_field),
                Some(node_kind),
                Some(payload_kind),
            ),
            RawSyntaxDecodedPayloadMaterialization {
                payload_byte_count: 0,
                payload_token_byte: None,
                payload_value: Some(payload_value),
            },
        );
        Ok(())
    }

    /// Choice token begins a class-specific payload substate; subsequent
    /// admissible tokens are only that class's payload band / terminator.
    fn apply_mixed_payload_choice(
        &mut self,
        token: u32,
        alt: &DiscriminatedAlternative,
    ) -> Result<(), RawSyntaxError> {
        let frame_index = self.live.stack.len().checked_sub(1).ok_or_else(|| {
            self.error(
                "unexpected_rule_or_node_kind",
                "mixed payload choice appeared with no active node",
                Some(token),
            )
        })?;
        let frame = &self.live.stack[frame_index];
        let Some(field) = frame.open_field.as_ref() else {
            return Err(self.error(
                "unexpected_field",
                "mixed payload choice appeared with no open field",
                Some(token),
            ));
        };
        if field.missing_seen || field.repeated_empty_seen {
            return Err(self.error(
                "unexpected_structural_token",
                format!(
                    "field {}.{} already carries a missing or repeated-empty marker; only field_end is admissible",
                    frame.rule, field.name
                ),
                Some(token),
            ));
        }
        if !field.multiple && field.value_count > 0 {
            return Err(self.error(
                "repeated_field_cardinality",
                format!(
                    "field {}.{} is single-value but received another value",
                    frame.rule, field.name
                ),
                Some(token),
            ));
        }
        let payload_kind = alt.payload_kind.clone().ok_or_else(|| {
            self.error(
                "missing_state_metadata",
                format!(
                    "mixed payload alternative {} is missing payload_kind",
                    alt.node_kind
                ),
                Some(token),
            )
        })?;
        let owner_rule = frame.rule.clone();
        let owner_field = field.name.clone();
        let node_kind = alt.node_kind.clone();
        let menu_cardinality = self.resolve_menu_cardinality(&owner_rule, &owner_field, token)?;
        self.live.payload = Some(PayloadState {
            owner_frame_index: frame_index,
            owner_rule: owner_rule.clone(),
            owner_field: owner_field.clone(),
            candidates: vec![PayloadSpec {
                node_kind: node_kind.clone(),
                payload_kind: payload_kind.clone(),
            }],
            bytes: Vec::new(),
            byte_count: 0,
            menu_cardinality,
            menu_width: None,
            menu_digits_seen: 0,
            menu_partial: 0,
            menu_index: None,
        });
        self.push_decoded_event(
            token,
            RawSyntaxDecodedEventKind::TerminalStart,
            RawSyntaxDecodedEventContext::new(
                Some(owner_rule),
                Some(owner_field),
                Some(node_kind),
                Some(payload_kind),
            ),
            RawSyntaxDecodedPayloadMaterialization::default(),
        );
        Ok(())
    }

    /// Resolve menu cardinality from bindings + live scope (**P4**/**P5**).
    /// Empty binding → `None` (menu path unreachable, **P10**).
    fn resolve_menu_cardinality(
        &self,
        owner_rule: &str,
        owner_field: &str,
        token: u32,
    ) -> Result<Option<u32>, RawSyntaxError> {
        if self
            .menu_bindings
            .site_kind(owner_rule, owner_field)
            .is_none()
        {
            return Ok(None);
        }
        let scope = self.current_scope_tables();
        let Some(cardinality) =
            self.menu_bindings
                .resolve_cardinality(owner_rule, owner_field, &scope)
        else {
            return Ok(None);
        };
        if cardinality == 0 {
            return Err(self.error(
                "menu_scope_empty",
                format!(
                    "menu site {owner_rule}.{owner_field} resolved to empty scope (no tables in scope)"
                ),
                Some(token),
            ));
        }
        Ok(Some(cardinality))
    }

    /// Initial join-ON pair memory for a frame about to be pushed: present iff
    /// the structural constraint is bound, the pushed rule is the comparison
    /// rule, and the nearest enclosing join frame (before any scope root) has
    /// its ON field open. Unbound models never allocate this (**P10**).
    fn join_on_cmp_init_for_push(&self, rule: &str) -> Option<JoinOnCmpState> {
        let constraint = self.menu_bindings.join_on()?;
        if rule != constraint.comparison_rule {
            return None;
        }
        for frame in self.live.stack.iter().rev() {
            if frame.rule == constraint.join_rule {
                let under_on = frame
                    .open_field
                    .as_ref()
                    .is_some_and(|open| open.name == constraint.on_field);
                return under_on.then(JoinOnCmpState::default);
            }
            if frame.scope_tables.is_some() {
                // Crossed into an enclosing scope root (e.g. a subquery inside
                // the ON expression): the constraint does not apply there.
                return None;
            }
        }
        None
    }

    /// Comparison ancestor of a column frame, skipping only pure expression
    /// wrapper frames. Columns nested under any other rule (aggregate calls,
    /// subqueries) resolve to `None` and stay unconstrained by design.
    fn join_on_comparison_ancestor(
        &self,
        column_frame_index: usize,
        constraint: &crate::syntax_state_machine::JoinOnConstraint,
    ) -> Option<usize> {
        let mut index = column_frame_index.checked_sub(1)?;
        loop {
            let frame = self.live.stack.get(index)?;
            if frame.rule == constraint.expression_wrapper_rule {
                index = index.checked_sub(1)?;
                continue;
            }
            return (frame.rule == constraint.comparison_rule).then_some(index);
        }
    }

    /// Validate one factored column digit against the live scope (and the
    /// join-ON constraint when bound), returning
    /// `(new_digits_seen, new_partial, completed_flat_ordinal)`.
    ///
    /// `menu_partial` carries the catalog table index between the two digits;
    /// on completion it becomes the flat scope-relative ordinal so the decoded
    /// payload value, the join-ON left-table memory, and the rendered AST are
    /// identical to the flat encoding's.
    fn factored_menu_digit_step(
        &self,
        digit: u32,
        digits_seen: u32,
        partial: u32,
        owner_rule: &str,
        owner_field: &str,
        token: u32,
    ) -> Result<(u32, u32, Option<u32>), RawSyntaxError> {
        let columns_per_table = self
            .menu_bindings
            .column_site_columns_per_table(owner_rule, owner_field)
            .unwrap_or(&[]);
        let scope = self.current_scope_tables();
        let allowed = self.join_on_allowed_column_range();
        match digits_seen {
            0 => {
                if !factored_table_digit_admissible(digit, &scope, columns_per_table, allowed) {
                    let kind = if allowed.is_some() {
                        "join_on_structural_violation"
                    } else {
                        "invalid_payload_token"
                    };
                    return Err(self.error(
                        kind,
                        format!(
                            "factored column table digit {digit} is not an addressable in-scope table for {owner_rule}.{owner_field} (scope {scope:?}, allowed flat range {allowed:?})"
                        ),
                        Some(token),
                    ));
                }
                Ok((1, digit, None))
            }
            1 => {
                let table = partial;
                if !factored_column_digit_admissible(
                    digit,
                    table,
                    &scope,
                    columns_per_table,
                    allowed,
                ) {
                    let kind = if allowed.is_some() {
                        "join_on_structural_violation"
                    } else {
                        "invalid_payload_token"
                    };
                    return Err(self.error(
                        kind,
                        format!(
                            "factored column digit {digit} is not a column of table {table} for {owner_rule}.{owner_field} (scope {scope:?}, allowed flat range {allowed:?})"
                        ),
                        Some(token),
                    ));
                }
                let Some(flat) =
                    factored_flat_ordinal(table, digit, &scope, columns_per_table)
                else {
                    return Err(self.error(
                        "invalid_payload_token",
                        format!(
                            "factored column pair ({table}, {digit}) has no flat scope-relative ordinal for {owner_rule}.{owner_field}"
                        ),
                        Some(token),
                    ));
                };
                Ok((FACTORED_COLUMN_MENU_DIGITS, flat, Some(flat)))
            }
            _ => Err(self.error(
                "invalid_payload_continuation",
                format!(
                    "factored column menu for {owner_rule}.{owner_field} already consumed {digits_seen} digits"
                ),
                Some(token),
            )),
        }
    }

    /// Allowed scope-relative column-ordinal range for the live payload when it
    /// is the RIGHT side of an equality comparison under a join-ON field:
    /// columns of the just-joined table when the left side named an earlier
    /// table, and earlier-table columns when the left side named the joined
    /// table. `None` = unconstrained (not a join-ON right column, non-equality
    /// operator, or no plain-column left side). An empty range fails closed.
    fn join_on_allowed_column_range(&self) -> Option<(u32, u32)> {
        let constraint = self.menu_bindings.join_on()?;
        let payload = self.live.payload.as_ref()?;
        if payload.owner_rule != constraint.column_rule
            || payload.owner_field != constraint.column_field
        {
            return None;
        }
        let parent_index =
            self.join_on_comparison_ancestor(payload.owner_frame_index, constraint)?;
        let parent = self.live.stack.get(parent_index)?;
        let cmp = parent.join_on_cmp.as_ref()?;
        let open = parent.open_field.as_ref()?;
        if open.name != constraint.comparison_right_field {
            return None;
        }
        if cmp.op_is_eq != Some(true) {
            return None;
        }
        let left_table = cmp.left_table?;
        // Both column-site kinds are constrained; the allowed range is stated
        // in flat scope-relative ordinals either way, and the factored digit
        // filters intersect against it.
        let columns_per_table = self
            .menu_bindings
            .column_site_columns_per_table(&payload.owner_rule, &payload.owner_field)?;
        let scope = self.current_scope_tables();
        let Some((&joined, earlier)) = scope.split_last() else {
            return Some((0, 0));
        };
        if earlier.is_empty() {
            // A join-ON pair needs at least two tables in scope; fail closed.
            return Some((0, 0));
        }
        let mut prefix_before_joined = 0u32;
        for &table in earlier {
            prefix_before_joined = prefix_before_joined
                .saturating_add(columns_per_table.get(table as usize).copied().unwrap_or(0));
        }
        if left_table == joined {
            if earlier.len() == 1
                && let Some(col) = self
                    .menu_bindings
                    .unique_fk_right_column(left_table, earlier[0])
            {
                return Some((col, col.saturating_add(1)));
            }
            Some((0, prefix_before_joined))
        } else {
            let joined_width = columns_per_table.get(joined as usize).copied().unwrap_or(0);
            let total = prefix_before_joined.saturating_add(joined_width);
            if let Some(col) = self
                .menu_bindings
                .unique_fk_right_column(left_table, joined)
                && col < joined_width
            {
                let flat = prefix_before_joined.saturating_add(col);
                return Some((flat, flat.saturating_add(1)));
            }
            Some((prefix_before_joined, total))
        }
    }

    /// Live menu cursor for an open menu payload, if any.
    pub(super) fn menu_cursor(&self) -> Option<crate::syntax_state_machine::LiveMenuCursor> {
        let payload = self.live.payload.as_ref()?;
        let site_kind = self
            .menu_bindings
            .site_kind(&payload.owner_rule, &payload.owner_field)?
            .clone();
        Some(crate::syntax_state_machine::LiveMenuCursor {
            owner_rule: payload.owner_rule.clone(),
            owner_field: payload.owner_field.clone(),
            site_kind,
            scope_tables: self.current_scope_tables(),
            digits_seen: payload.menu_digits_seen,
            menu_partial: payload.menu_partial,
            menu_cardinality: payload.menu_cardinality,
            menu_width: payload.menu_width,
        })
    }

    /// Nearest scope-root frame's table ordinals (empty if no scope root yet).
    fn current_scope_tables(&self) -> Vec<u32> {
        for frame in self.live.stack.iter().rev() {
            if let Some(scope) = frame.scope_tables.as_ref() {
                return scope.clone();
            }
        }
        Vec::new()
    }

    /// Push a completed table-menu ordinal onto the nearest scope root (**P5**).
    ///
    /// Pre-M6: fails closed if the ordinal is already present (no occurrence
    /// aliases yet; repeated tables break P8 render fixpoint).
    fn push_scope_table(&mut self, table_ordinal: u32, token: u32) -> Result<(), RawSyntaxError> {
        for frame in self.live.stack.iter_mut().rev() {
            if let Some(scope) = frame.scope_tables.as_mut() {
                if scope.contains(&table_ordinal) {
                    return Err(self.error(
                        "menu_duplicate_scope_table",
                        format!(
                            "table menu ordinal {table_ordinal} is already in scope; repeated table ordinals are rejected until occurrence aliases (M6)"
                        ),
                        Some(token),
                    ));
                }
                scope.push(table_ordinal);
                return Ok(());
            }
        }
        Err(self.error(
            "menu_scope_missing",
            format!(
                "table menu ordinal {table_ordinal} completed without a scope-root frame on the stack"
            ),
            Some(token),
        ))
    }

    fn error(
        &self,
        kind: &'static str,
        message: impl Into<String>,
        failing_token: Option<u32>,
    ) -> RawSyntaxError {
        self.error_at(kind, message, self.live.consumed, failing_token)
    }

    fn error_at(
        &self,
        kind: &'static str,
        message: impl Into<String>,
        consumed_raw_tokens: usize,
        failing_token: Option<u32>,
    ) -> RawSyntaxError {
        RawSyntaxError {
            kind,
            message: message.into(),
            consumed_raw_tokens,
            failing_token,
            stack_depth: self.live.stack.len(),
            active_rule: self.active_rule(),
            active_field: self.active_field(),
            expected_payload_class: self.active_payload_kind(),
        }
    }

    fn active_rule(&self) -> Option<String> {
        self.live.stack.last().map(|frame| frame.rule.clone())
    }

    fn active_field(&self) -> Option<String> {
        if let Some(payload) = self.live.payload.as_ref() {
            return Some(payload.owner_field.clone());
        }
        self.live
            .stack
            .last()
            .and_then(|frame| frame.open_field.as_ref())
            .map(|field| field.name.clone())
    }

    fn active_payload_kind(&self) -> Option<String> {
        let payload = self.live.payload.as_ref()?;
        let mut kinds = payload
            .candidates
            .iter()
            .map(|candidate| candidate.payload_kind.as_str())
            .collect::<BTreeSet<_>>();
        if kinds.len() == 1 {
            kinds.pop_first().map(str::to_string)
        } else {
            Some(kinds.into_iter().collect::<Vec<_>>().join("|"))
        }
    }

    fn active_payload_node_kind(&self) -> Option<String> {
        let payload = self.live.payload.as_ref()?;
        let mut node_kinds = payload
            .candidates
            .iter()
            .map(|candidate| candidate.node_kind.as_str())
            .collect::<BTreeSet<_>>();
        if node_kinds.len() == 1 {
            node_kinds.pop_first().map(str::to_string)
        } else {
            Some(node_kinds.into_iter().collect::<Vec<_>>().join("|"))
        }
    }

    fn push_decoded_event(
        &mut self,
        raw_token_id: u32,
        event_kind: RawSyntaxDecodedEventKind,
        context: RawSyntaxDecodedEventContext,
        payload: RawSyntaxDecodedPayloadMaterialization,
    ) {
        self.live.last_event_kind = Some(event_kind);
        if !self.live.record_events {
            return;
        }
        self.live.decoded_events.push(RawSyntaxDecodedEvent {
            raw_token_offset: self.live.consumed,
            raw_token_id,
            event_kind,
            rule: context.rule,
            field: context.field,
            node_kind: context.node_kind,
            payload_kind: context.payload_kind,
            payload_byte_count: payload.payload_byte_count,
            payload_token_byte: payload.payload_token_byte,
            payload_value: payload.payload_value,
        });
    }
}

impl<'model> RawSyntaxStream<'model> {
    /// Applies one raw token to the live state.
    ///
    /// Runs the same `apply_token` dispatch as the replay loop, so
    /// acceptance, errors, and state effects are identical to replaying the
    /// extended prefix from scratch. `advance` is error-atomic: every
    /// `apply_*` path validates before mutating, so on `Err` the handle is
    /// unchanged and remains usable (the transition-acceptance property test
    /// asserts post-`Err` snapshot/mask equality and reuse for every
    /// rejected candidate over the checked corpora).
    pub fn advance(&mut self, raw_token_id: u32) -> Result<RawSyntaxAdvanceDelta, RawSyntaxError> {
        // Shared with owned live-cursor advance: one transition, error-atomic.
        self.machine.advance_one(raw_token_id)
    }

    /// Admissible-mask access on the live state: the same pure mask
    /// computation the replay path uses, with no stream re-walk.
    pub fn admissible_next_raw_tokens(&self) -> RawSyntaxAdmissibleTokens {
        self.machine.admissible_next_raw_tokens()
    }

    /// Mask membership for one candidate raw token. Equivalent to checking
    /// `admissible_next_raw_tokens()` for membership, which the equivalence
    /// property test proves equal to transition acceptance.
    pub fn admits(&self, raw_token_id: u32) -> bool {
        admissible_contains_raw_token(&self.machine.admissible_next_raw_tokens(), raw_token_id)
    }

    /// Full `RawSyntaxState` snapshot, field-for-field equal to
    /// `CompiledSyntaxModel::state_from_prefix` over the consumed tokens
    /// (shared `AllowPrefix` classification and diagnostics;
    /// `total_raw_tokens` equals `consumed_raw_tokens`).
    pub fn state_snapshot(&self) -> RawSyntaxState {
        self.machine.prefix_state()
    }

    pub fn consumed_raw_tokens(&self) -> usize {
        self.machine.live.consumed
    }

    pub fn eof_valid(&self) -> bool {
        self.machine.eof_valid()
    }

    pub fn stack_depth(&self) -> usize {
        self.machine.live.stack.len()
    }

    pub fn active_rule(&self) -> Option<&str> {
        self.machine
            .live
            .stack
            .last()
            .map(|frame| frame.rule.as_str())
    }

    pub fn active_field(&self) -> Option<&str> {
        if let Some(payload) = self.machine.live.payload.as_ref() {
            return Some(payload.owner_field.as_str());
        }
        self.machine
            .live
            .stack
            .last()
            .and_then(|frame| frame.open_field.as_ref())
            .map(|field| field.name.as_str())
    }
}

impl<'model> ElidedRawSyntaxStream<'model> {
    /// Builds the elided handle by eagerly draining the initial mask-forced
    /// run (the root node_start is always forced at the empty prefix), so
    /// the handle's observable position is non-forced from construction on.
    /// Fails only when the drain hits the totality guard
    /// (`forced_run_limit_exceeded`) or the engine-corruption guard
    /// (`expansion_stuck`); both are unreachable for the checked grammars.
    pub(super) fn from_stream(stream: RawSyntaxStream<'model>) -> Result<Self, RawSyntaxError> {
        let mut elided = Self {
            stream,
            consumed_elided_tokens: 0,
            initial_forced_run: Vec::new(),
            last_forced_run: Vec::new(),
        };
        let initial_forced_run = elided.drain_forced_run()?;
        elided.initial_forced_run = initial_forced_run.clone();
        elided.last_forced_run = initial_forced_run;
        Ok(elided)
    }

    /// Applies one elided raw token to the live state, then eagerly drains
    /// the following mask-forced run so the observable position stays
    /// non-forced. The auto-applied forced tokens are returned in the
    /// advance result so callers can record the re-expansion.
    ///
    /// Strictness (canonical elided encoding is unique): an input token that
    /// sits on a mask-forced position is rejected with
    /// `forced_token_in_elided_stream` even when it equals the forced
    /// token, and a rejected input token that spells out a forced token
    /// from the immediately preceding drain is reclassified to the same
    /// dedicated kind so the diagnosis names the elision violation.
    ///
    /// Error atomicity is inherited from `RawSyntaxStream::advance` for
    /// input-token rejections: on `Err` the handle is unchanged and remains
    /// usable. A drain failure (the totality or engine-corruption guards)
    /// is terminal: the handle has consumed part of the forced run and must
    /// be discarded.
    pub fn advance(&mut self, raw_token_id: u32) -> Result<ElidedRawSyntaxAdvance, RawSyntaxError> {
        // Defensive restatement of the owner rule: after construction and
        // every successful advance the eager drain guarantees a non-forced
        // position, so this only fires on a handle that was kept alive past
        // a terminal drain error.
        if let Some(forced_token) =
            singleton_admissible_raw_token(&self.stream.machine.admissible_next_raw_tokens())
        {
            return Err(self.forced_token_error(
                raw_token_id,
                forced_token,
                self.stream.consumed_raw_tokens(),
            ));
        }
        let consumed = match self.stream.advance(raw_token_id) {
            Ok(delta) => delta,
            Err(error) => {
                if !self.stream.eof_valid()
                    && let Some(forced) = self
                        .last_forced_run
                        .iter()
                        .find(|forced| forced.raw_token_id == raw_token_id)
                {
                    return Err(self.forced_token_error(
                        raw_token_id,
                        forced.raw_token_id,
                        forced.expanded_raw_token_offset,
                    ));
                }
                return Err(error);
            }
        };
        self.consumed_elided_tokens += 1;
        let forced_tokens_applied = self.drain_forced_run()?;
        self.last_forced_run = forced_tokens_applied.clone();
        Ok(ElidedRawSyntaxAdvance {
            consumed,
            forced_tokens_applied,
        })
    }

    /// Admissible-mask access on the live elided position. By the eager
    /// forced-run drain this mask is never a singleton on a usable handle:
    /// it is exactly the choice set a model-side sampler must see.
    pub fn admissible_next_raw_tokens(&self) -> RawSyntaxAdmissibleTokens {
        self.stream.machine.admissible_next_raw_tokens()
    }

    /// Mask-forced run auto-applied at construction, before the first
    /// elided input token.
    pub fn initial_forced_run(&self) -> &[ElidedForcedRawToken] {
        &self.initial_forced_run
    }

    /// Mask-forced run auto-applied by the most recent drain (construction
    /// or the latest successful `advance`).
    pub fn last_forced_run(&self) -> &[ElidedForcedRawToken] {
        &self.last_forced_run
    }

    pub fn consumed_elided_tokens(&self) -> usize {
        self.consumed_elided_tokens
    }

    /// Expanded-layer token count: consumed elided tokens plus every
    /// auto-applied forced token.
    pub fn expanded_raw_token_count(&self) -> usize {
        self.stream.consumed_raw_tokens()
    }

    pub fn eof_valid(&self) -> bool {
        self.stream.eof_valid()
    }

    /// Full `RawSyntaxState` snapshot of the underlying expanded-layer
    /// position (consumed counts, offsets, and decoded-event semantics stay
    /// defined over the expanded stream).
    pub fn state_snapshot(&self) -> RawSyntaxState {
        self.stream.state_snapshot()
    }

    /// Eagerly applies mask-forced tokens while the live mask is a
    /// singleton, returning the applied run with expanded-layer offsets.
    ///
    /// The run cap is a totality guard, not a tuning knob: a grammar with
    /// unbounded required recursion would otherwise make this loop diverge.
    /// The engine-proven mask/transition equivalence guarantees that
    /// advancing a singleton-mask member succeeds, so an `advance` failure
    /// here is engine corruption and is propagated as-is.
    fn drain_forced_run(&mut self) -> Result<Vec<ElidedForcedRawToken>, RawSyntaxError> {
        let mut run = Vec::new();
        loop {
            let admissible = self.stream.machine.admissible_next_raw_tokens();
            let Some(forced_token) = singleton_admissible_raw_token(&admissible) else {
                if finite_admissible_raw_token_count(&admissible) == 0 && !self.stream.eof_valid() {
                    return Err(self.stream.machine.error(
                        "expansion_stuck",
                        format!(
                            "elided-stream expansion reached an empty admissible mask before \
                             EOF validity at expanded offset {} (after {} consumed elided \
                             tokens); the engine empties the mask only after the root closes",
                            self.stream.consumed_raw_tokens(),
                            self.consumed_elided_tokens
                        ),
                        None,
                    ));
                }
                break;
            };
            if run.len() >= ELIDED_FORCED_RUN_LIMIT {
                return Err(self.stream.machine.error(
                    "forced_run_limit_exceeded",
                    format!(
                        "elided-stream expansion auto-applied {ELIDED_FORCED_RUN_LIMIT} \
                         consecutive mask-forced tokens at expanded offset {} (after {} \
                         consumed elided tokens) without reaching a free choice; the run cap \
                         is a totality guard against unbounded forced recursion",
                        self.stream.consumed_raw_tokens(),
                        self.consumed_elided_tokens
                    ),
                    Some(forced_token),
                ));
            }
            let expanded_raw_token_offset = self.stream.consumed_raw_tokens();
            self.stream.advance(forced_token)?;
            run.push(ElidedForcedRawToken {
                expanded_raw_token_offset,
                raw_token_id: forced_token,
            });
        }
        Ok(run)
    }

    fn forced_token_error(
        &self,
        offending_token: u32,
        forced_token: u32,
        expanded_raw_token_offset: usize,
    ) -> RawSyntaxError {
        self.stream.machine.error(
            "forced_token_in_elided_stream",
            format!(
                "elided input token {offending_token} at elided offset {} spells out the \
                 mask-forced token {forced_token} that expansion auto-applies at expanded \
                 offset {expanded_raw_token_offset}; canonical elided streams omit every \
                 mask-forced token, even when the spelled-out token matches the forced value",
                self.consumed_elided_tokens
            ),
            Some(offending_token),
        )
    }
}

fn join_payload_kinds(candidates: &[PayloadSpec]) -> String {
    candidates
        .iter()
        .map(|candidate| candidate.payload_kind.as_str())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect::<Vec<_>>()
        .join("|")
}

fn join_payload_node_kinds(candidates: &[PayloadSpec]) -> String {
    candidates
        .iter()
        .map(|candidate| candidate.node_kind.as_str())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect::<Vec<_>>()
        .join("|")
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct PayloadState {
    owner_frame_index: usize,
    owner_rule: String,
    owner_field: String,
    candidates: Vec<PayloadSpec>,
    bytes: Vec<u8>,
    byte_count: usize,
    /// When set, this payload is a menu-index site with exclusive cardinality
    /// (**P4**). Menu values never flow through `bytes`.
    menu_cardinality: Option<u32>,
    /// Digit width `W` for this site instance (from live cardinality + digit
    /// budget). `None` until the first digit is accepted or the mask path
    /// materialises it; recomputed deterministically from cardinality.
    menu_width: Option<u32>,
    /// Digits of the multi-digit ordinal already accepted.
    menu_digits_seen: u32,
    /// Big-endian partial integer formed by digits accepted so far.
    menu_partial: u32,
    /// Complete menu ordinal once all `menu_width` digits are accepted.
    /// Intermediate digits leave this `None` so existing `menu_index.is_some()`
    /// sites keep "value complete" semantics (payload_end, scope push).
    menu_index: Option<u32>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct FrameState {
    rule: String,
    node_kind: String,
    fields: Vec<FieldSpec>,
    field_index: usize,
    open_field: Option<FieldState>,
    /// Table menu ordinals in FROM/JOIN order when this rule is a scope root
    /// (**P5**). `None` on non-scope-root frames.
    scope_tables: Option<Vec<u32>>,
    /// Join-ON pair memory, present only on comparison frames pushed under an
    /// open join-ON field while the structural constraint is bound. `None`
    /// everywhere else keeps unbound models bit-identical (**P10**).
    join_on_cmp: Option<JoinOnCmpState>,
}

/// Per-comparison memory for the structural join-ON constraint.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
struct JoinOnCmpState {
    /// Catalog table index of the completed left-side plain column, if any.
    left_table: Option<u32>,
    /// Whether the comparison's operator alternative is the equality op.
    op_is_eq: Option<bool>,
}

impl FrameState {
    fn from_record(record: &RecordSpec, is_scope_root: bool) -> Self {
        Self {
            rule: record.rule.clone(),
            node_kind: record.node_kind.clone(),
            fields: record.fields.clone(),
            field_index: 0,
            open_field: None,
            scope_tables: if is_scope_root {
                Some(Vec::new())
            } else {
                None
            },
            join_on_cmp: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct FieldState {
    name: String,
    required: bool,
    multiple: bool,
    value_count: usize,
    missing_seen: bool,
    repeated_empty_seen: bool,
}

fn finite_admissible_raw_token_count(admissible: &RawSyntaxAdmissibleTokens) -> usize {
    admissible.finite_raw_token_ids.len()
        + admissible
            .raw_token_ranges
            .iter()
            .map(|range| (range.end_exclusive - range.start) as usize)
            .sum::<usize>()
}

fn admissible_contains_raw_token(admissible: &RawSyntaxAdmissibleTokens, token_id: u32) -> bool {
    admissible.finite_raw_token_ids.contains(&token_id)
        || admissible
            .raw_token_ranges
            .iter()
            .any(|range| token_id >= range.start && token_id < range.end_exclusive)
}

/// Total singleton extractor over the admissible-mask shape: returns
/// `Some(id)` iff the finite ids plus the range widths sum to exactly one
/// admissible raw token. The check is defined over total mask cardinality,
/// not token classes, so a width-1 payload range, a lone `payload_end`, or
/// a lone boolean is a singleton exactly like a lone structural role.
///
/// Payload-side singletons **do** occur in practice (for example a 1-table
/// catalog admits exactly one menu ordinal at a table site, and a completed
/// menu site admits only `payload_end`). Callers must not assume payload
/// masks are always wide ranges.
pub fn singleton_admissible_raw_token(admissible: &RawSyntaxAdmissibleTokens) -> Option<u32> {
    if finite_admissible_raw_token_count(admissible) != 1 {
        return None;
    }
    if let Some(&token_id) = admissible.finite_raw_token_ids.first() {
        return Some(token_id);
    }
    admissible
        .raw_token_ranges
        .iter()
        .find(|range| range.end_exclusive - range.start == 1)
        .map(|range| range.start)
}

fn payload_token_range_for_kind(
    payload_kind: &str,
    roles: &SyntaxTokenRoles,
) -> Option<ProjectionTokenLayoutRangeSummary> {
    if is_integer_payload_kind(payload_kind) {
        Some(roles.integer_range.clone())
    } else if is_string_payload_kind(payload_kind) || is_identifier_payload_kind(payload_kind) {
        Some(roles.utf8_range.clone())
    } else {
        None
    }
}

fn payload_token_byte_for_candidates(
    candidates: &[PayloadSpec],
    token: u32,
    roles: &SyntaxTokenRoles,
) -> Option<u8> {
    candidates
        .iter()
        .filter_map(|candidate| payload_token_range_for_kind(&candidate.payload_kind, roles))
        .find_map(|range| {
            if token >= range.start && token < range.end_exclusive {
                u8::try_from(token - range.start).ok()
            } else {
                None
            }
        })
}

fn is_identifier_payload_kind(payload_kind: &str) -> bool {
    matches!(payload_kind, "identifier" | "identifier_payload")
        || payload_kind.contains(":identifier_payload")
}

fn is_string_payload_kind(payload_kind: &str) -> bool {
    matches!(payload_kind, "string" | "string_payload") || payload_kind.contains(":string_payload")
}

fn is_integer_payload_kind(payload_kind: &str) -> bool {
    matches!(payload_kind, "integer" | "integer_payload")
        || payload_kind.contains(":integer_payload")
}

fn is_u8_integer_range(roles: &SyntaxTokenRoles) -> bool {
    roles.integer_range.role == "u8_integer_payload"
}

fn is_boolean_payload_kind(payload_kind: &str) -> bool {
    matches!(payload_kind, "boolean" | "boolean_payload")
        || payload_kind.contains(":boolean_payload")
}

/// Zero-width BooleanPayload value from an explicit true/false node kind.
/// Mixed-field construction rejects any other BooleanPayload identity, so
/// materialization never defaults non-`false` names to true.
fn boolean_value_for_explicit_identity(node_kind: &str) -> Option<&'static str> {
    match node_kind {
        "true" => Some("true"),
        "false" => Some("false"),
        _ => None,
    }
}

#[cfg(test)]
mod mixed_payload_contract_tests {
    use super::super::model_facts::mixed_payload_witness::mixed_payload_sidecar;
    use crate::syntax_go_source_instance_projection::RAW_TOKEN_ALTERNATIVE_NODE_START_OFFSET;
    use crate::syntax_state_machine::{
        CompiledSyntaxModel, RawSyntaxDecodedEventKind, RawSyntaxDecodedValue, RawSyntaxStatus,
    };
    use std::collections::BTreeSet;

    fn compiled_mixed_model() -> CompiledSyntaxModel {
        let sidecar = mixed_payload_sidecar();
        CompiledSyntaxModel::from_sidecar(&sidecar).expect("mixed witness should compile")
    }

    fn choice_token(model: &CompiledSyntaxModel, node_kind: &str) -> u32 {
        *model
            .token_roles()
            .node_start_alternative_tokens
            .get(node_kind)
            .unwrap_or_else(|| panic!("missing choice token for {node_kind}"))
    }

    /// Prefix through field_start on wrapper.value (root open, value open).
    fn open_mixed_field_prefix(model: &CompiledSyntaxModel) -> Vec<u32> {
        let roles = model.token_roles();
        vec![roles.node_start, roles.field_start]
    }

    fn complete_child_stream(model: &CompiledSyntaxModel) -> Vec<u32> {
        let roles = model.token_roles();
        let child = choice_token(model, "child");
        vec![
            roles.node_start,
            roles.field_start,
            child,
            roles.node_end, // empty child record
            roles.field_end,
            roles.node_end, // wrapper
        ]
    }

    fn complete_boolean_stream(model: &CompiledSyntaxModel, node_kind: &str) -> Vec<u32> {
        let roles = model.token_roles();
        let choice = choice_token(model, node_kind);
        vec![
            roles.node_start,
            roles.field_start,
            choice,
            roles.field_end,
            roles.node_end,
        ]
    }

    fn complete_integer_stream(model: &CompiledSyntaxModel, value: u8) -> Vec<u32> {
        let roles = model.token_roles();
        let number = choice_token(model, "number");
        let mut tokens = vec![roles.node_start, roles.field_start, number];
        let digits = crate::syntax_state_machine::integer_codec_encode_digits(u32::from(value))
            .expect("u8 value is within integer capacity");
        for digit in digits {
            tokens.push(roles.integer_range.start + u32::from(digit));
        }
        tokens.push(roles.payload_end);
        tokens.push(roles.field_end);
        tokens.push(roles.node_end);
        tokens
    }

    fn complete_utf8_stream(model: &CompiledSyntaxModel, text: &str) -> Vec<u32> {
        let roles = model.token_roles();
        let text_choice = choice_token(model, "text");
        let mut tokens = vec![roles.node_start, roles.field_start, text_choice];
        for byte in text.as_bytes() {
            tokens.push(roles.utf8_range.start + u32::from(*byte));
        }
        tokens.push(roles.payload_end);
        tokens.push(roles.field_end);
        tokens.push(roles.node_end);
        tokens
    }

    #[test]
    fn mixed_field_mask_admits_only_alternative_choice_tokens() {
        let model = compiled_mixed_model();
        let roles = model.token_roles();
        let prefix = open_mixed_field_prefix(&model);
        let admissible = model
            .admissible_next_raw_tokens(&prefix)
            .expect("prefix should be valid");

        let expected: BTreeSet<u32> = ["child", "number", "text", "true", "false"]
            .into_iter()
            .map(|kind| choice_token(&model, kind))
            .collect();
        let observed: BTreeSet<u32> = admissible.finite_raw_token_ids.iter().copied().collect();
        assert_eq!(
            observed, expected,
            "mixed field mask must be exactly the five alternative choice tokens"
        );
        assert!(
            !observed.contains(&roles.terminal),
            "mixed field must not admit generic terminal"
        );
        assert!(
            !observed.contains(&roles.node_start),
            "mixed field must not admit generic node_start"
        );
        assert!(
            admissible.raw_token_ranges.is_empty(),
            "mixed field must not admit raw payload ranges before a choice"
        );
        assert!(!admissible.payload_end_admissible);
        // Choice tokens live in the structural alternative band.
        for token in &expected {
            assert!(
                *token >= roles.structural_range.start + RAW_TOKEN_ALTERNATIVE_NODE_START_OFFSET
            );
            assert!(*token < roles.structural_range.end_exclusive);
            assert!(*token < roles.vocab_size);
            assert_ne!(*token, roles.pad_token);
        }
    }

    #[test]
    fn mixed_nonterminal_choice_descends_into_child_rule() {
        let model = compiled_mixed_model();
        let roles = model.token_roles();
        let mut tokens = open_mixed_field_prefix(&model);
        tokens.push(choice_token(&model, "child"));
        let state = model
            .state_from_prefix(&tokens)
            .expect("child choice should descend");
        assert_eq!(state.stack_depth, 2);
        assert_eq!(state.active_rule.as_deref(), Some("child"));
        assert!(state.active_payload_kind.is_none());
        // Empty child: node_end is admissible.
        assert!(
            state
                .admissible
                .finite_raw_token_ids
                .contains(&roles.node_end)
        );
    }

    #[test]
    fn mixed_boolean_true_choice_completes_zero_width_and_decodes_true() {
        let model = compiled_mixed_model();
        let roles = model.token_roles();
        let mut tokens = open_mixed_field_prefix(&model);
        tokens.push(choice_token(&model, "true"));
        let state = model
            .state_from_prefix(&tokens)
            .expect("true choice should complete");
        assert!(
            state.active_payload_kind.is_none(),
            "boolean body is zero-width"
        );
        assert_eq!(state.stack_depth, 1);
        assert_eq!(state.active_field.as_deref(), Some("value"));
        assert!(
            state
                .admissible
                .finite_raw_token_ids
                .contains(&roles.field_end),
            "after boolean choice only field_end (and not payload tokens) should remain"
        );
        assert!(
            !state
                .admissible
                .finite_raw_token_ids
                .contains(&roles.boolean_true)
                && !state
                    .admissible
                    .finite_raw_token_ids
                    .contains(&roles.boolean_false),
            "must not require a second boolean token after the choice"
        );

        let stream = complete_boolean_stream(&model, "true");
        let tree = model
            .decode_complete_tree(&stream)
            .expect("true complete stream");
        assert_eq!(tree.state.status, RawSyntaxStatus::Complete);
        assert_eq!(tree.state.consumed_raw_tokens, stream.len());
        let root = tree.decoded_node_tree().expect("decoded node tree");
        let value = root.field("value").expect("value field");
        match &value.values[0] {
            RawSyntaxDecodedValue::Terminal(terminal) => {
                assert_eq!(terminal.node_kind, "true");
                assert_eq!(terminal.terminal, "boolean");
                assert_eq!(terminal.payload, "true");
            }
            other => panic!("expected true boolean terminal, got {other:?}"),
        }
        // Events: choice produces TerminalStart + BooleanPayload on one token.
        let boolean_events: Vec<_> = tree
            .events
            .iter()
            .filter(|e| e.event_kind == RawSyntaxDecodedEventKind::BooleanPayload)
            .collect();
        assert_eq!(boolean_events.len(), 1);
        assert_eq!(boolean_events[0].payload_value.as_deref(), Some("true"));
        assert_eq!(boolean_events[0].node_kind.as_deref(), Some("true"));
    }

    #[test]
    fn mixed_boolean_false_choice_completes_zero_width_and_decodes_false() {
        let model = compiled_mixed_model();
        let roles = model.token_roles();
        let mut tokens = open_mixed_field_prefix(&model);
        tokens.push(choice_token(&model, "false"));
        let state = model
            .state_from_prefix(&tokens)
            .expect("false choice should complete");
        assert!(
            state.active_payload_kind.is_none(),
            "boolean body is zero-width"
        );
        assert_eq!(state.stack_depth, 1);
        assert_eq!(state.active_field.as_deref(), Some("value"));
        assert!(
            state
                .admissible
                .finite_raw_token_ids
                .contains(&roles.field_end),
            "after boolean choice field_end is admissible; choice alone completed the value"
        );
        assert!(
            !state
                .admissible
                .finite_raw_token_ids
                .contains(&roles.boolean_true)
                && !state
                    .admissible
                    .finite_raw_token_ids
                    .contains(&roles.boolean_false),
            "must not require a second boolean token after the choice"
        );

        let stream = complete_boolean_stream(&model, "false");
        let tree = model
            .decode_complete_tree(&stream)
            .expect("false complete stream");
        assert_eq!(tree.state.status, RawSyntaxStatus::Complete);
        assert_eq!(tree.state.consumed_raw_tokens, stream.len());
        let root = tree.decoded_node_tree().expect("decoded node tree");
        let value = root.field("value").expect("value field");
        match &value.values[0] {
            RawSyntaxDecodedValue::Terminal(terminal) => {
                assert_eq!(terminal.node_kind, "false");
                assert_eq!(terminal.terminal, "boolean");
                assert_eq!(terminal.payload, "false");
            }
            other => panic!("expected false boolean terminal, got {other:?}"),
        }
        let boolean_events: Vec<_> = tree
            .events
            .iter()
            .filter(|e| e.event_kind == RawSyntaxDecodedEventKind::BooleanPayload)
            .collect();
        assert_eq!(boolean_events.len(), 1);
        assert_eq!(boolean_events[0].payload_value.as_deref(), Some("false"));
        assert_eq!(boolean_events[0].node_kind.as_deref(), Some("false"));
    }

    #[test]
    fn mixed_integer_choice_opens_only_integer_payload_band() {
        let model = compiled_mixed_model();
        let roles = model.token_roles();
        let mut tokens = open_mixed_field_prefix(&model);
        tokens.push(choice_token(&model, "number"));
        let state = model
            .state_from_prefix(&tokens)
            .expect("integer choice should open payload");
        assert_eq!(state.active_payload_kind.as_deref(), Some("integer"));
        assert_eq!(state.active_payload_node_kind.as_deref(), Some("number"));
        // Only the integer range is admissible (no utf8, no sibling boolean tokens).
        assert_eq!(state.admissible.raw_token_ranges.len(), 1);
        assert_eq!(
            state.admissible.raw_token_ranges[0].role,
            roles.integer_range.role
        );
        assert!(
            !state
                .admissible
                .finite_raw_token_ids
                .contains(&roles.boolean_true)
        );
        assert!(
            !state
                .admissible
                .finite_raw_token_ids
                .contains(&roles.terminal)
        );

        // Sibling utf8 byte must be rejected without look-ahead reclassification.
        let utf8_byte = roles.utf8_range.start;
        let transition = model.transition(&tokens, utf8_byte);
        assert!(!transition.accepted);
        assert_eq!(
            transition.error.as_ref().map(|e| e.kind),
            Some("invalid_payload_token")
        );

        let stream = complete_integer_stream(&model, 42);
        let tree = model
            .decode_complete_tree(&stream)
            .expect("integer complete stream");
        assert_eq!(tree.state.consumed_raw_tokens, stream.len());
        let root = tree.decoded_node_tree().expect("decoded node tree");
        match &root.field("value").unwrap().values[0] {
            RawSyntaxDecodedValue::Terminal(terminal) => {
                assert_eq!(terminal.node_kind, "number");
                assert_eq!(terminal.payload, "42");
            }
            other => panic!("expected integer terminal, got {other:?}"),
        }
    }

    #[test]
    fn mixed_utf8_choice_opens_only_utf8_band_and_round_trips_multibyte() {
        let model = compiled_mixed_model();
        let roles = model.token_roles();
        let mut tokens = open_mixed_field_prefix(&model);
        tokens.push(choice_token(&model, "text"));
        let state = model
            .state_from_prefix(&tokens)
            .expect("string choice should open utf8 payload");
        assert_eq!(state.active_payload_kind.as_deref(), Some("string"));
        assert_eq!(state.active_payload_node_kind.as_deref(), Some("text"));

        // Only the utf8 band is admissible — not the integer band, not generic terminal.
        assert_eq!(
            state.admissible.raw_token_ranges.len(),
            1,
            "utf8 mixed body must admit exactly one payload range; got {:?}",
            state.admissible.raw_token_ranges
        );
        assert_eq!(
            state.admissible.raw_token_ranges[0].role,
            roles.utf8_range.role
        );
        assert_eq!(
            state.admissible.raw_token_ranges[0].start,
            roles.utf8_range.start
        );
        assert_eq!(
            state.admissible.raw_token_ranges[0].end_exclusive,
            roles.utf8_range.end_exclusive
        );
        // Empty bytes are valid UTF-8, so payload_end is also admissible — that is
        // correct empty-string completion, not a silent range-resolution failure.
        // A real resolution bug would admit *only* payload_end with no utf8 range.
        assert!(
            state
                .admissible
                .finite_raw_token_ids
                .contains(&roles.payload_end),
            "empty utf8 must admit payload_end"
        );
        assert!(
            !state
                .admissible
                .finite_raw_token_ids
                .contains(&roles.terminal),
            "utf8 body must not admit generic terminal"
        );
        let integer_byte = roles.integer_range.start;
        let integer_transition = model.transition(&tokens, integer_byte);
        assert!(
            !integer_transition.accepted,
            "integer band must not be admissible after a StringPayload choice"
        );
        assert_eq!(
            integer_transition.error.as_ref().map(|e| e.kind),
            Some("invalid_payload_token")
        );

        // Multi-byte UTF-8 payload must round-trip (not silently decode as empty).
        let expected_text = "café"; // multi-byte: c a f \xc3\xa9
        assert!(
            expected_text.as_bytes().iter().any(|b| *b >= 0x80),
            "fixture text must include a multi-byte UTF-8 sequence"
        );
        let stream = complete_utf8_stream(&model, expected_text);
        let tree = model
            .decode_complete_tree(&stream)
            .expect("utf8 complete stream");
        assert_eq!(tree.state.status, RawSyntaxStatus::Complete);
        assert_eq!(tree.state.consumed_raw_tokens, stream.len());
        let root = tree.decoded_node_tree().expect("decoded node tree");
        match &root.field("value").unwrap().values[0] {
            RawSyntaxDecodedValue::Terminal(terminal) => {
                assert_eq!(terminal.node_kind, "text");
                assert_eq!(terminal.terminal, "string");
                assert_eq!(
                    terminal.payload, expected_text,
                    "multi-byte utf8 must decode to the original string, not empty"
                );
                assert!(
                    !terminal.payload.is_empty(),
                    "payload must not silently collapse to empty"
                );
            }
            other => panic!("expected string terminal, got {other:?}"),
        }
    }

    #[test]
    fn mixed_decode_never_looks_ahead_at_payload_bytes_for_choice() {
        let model = compiled_mixed_model();
        // After field_start, only choice tokens are legal — applying an
        // integer payload byte without a choice must fail (no look-ahead).
        let roles = model.token_roles();
        let tokens = open_mixed_field_prefix(&model);
        let bare_integer = roles.integer_range.start + 7;
        let transition = model.transition(&tokens, bare_integer);
        assert!(
            !transition.accepted,
            "payload byte must not select the alternative"
        );
        assert_eq!(
            transition.error.as_ref().map(|e| e.kind),
            Some("unexpected_structural_token")
        );
        // Generic terminal is also rejected at a mixed field.
        let terminal_transition = model.transition(&tokens, roles.terminal);
        assert!(!terminal_transition.accepted);
    }

    #[test]
    fn mixed_strict_truncation_and_trailing_tokens() {
        let model = compiled_mixed_model();
        let roles = model.token_roles();
        let number = choice_token(&model, "number");
        // Truncated after integer choice (payload body open, zero bytes).
        let truncated_after_choice = vec![roles.node_start, roles.field_start, number];
        let err = model
            .decode_complete(&truncated_after_choice)
            .expect_err("truncated payload must fail strict decode");
        assert_eq!(err.kind, "premature_eof");
        assert_eq!(err.consumed_raw_tokens, truncated_after_choice.len());
        assert_eq!(
            err.expected_payload_class.as_deref(),
            Some("integer"),
            "strict mid-payload EOF must name the expected class"
        );
        assert!(
            err.message.contains("expected class integer"),
            "message should name the class: {}",
            err.message
        );
        assert_eq!(err.active_field.as_deref(), Some("value"));

        // Truncated mid-payload (value byte without payload_end).
        let truncated_mid = vec![
            roles.node_start,
            roles.field_start,
            number,
            roles.integer_range.start + 1,
        ];
        let err = model
            .decode_complete(&truncated_mid)
            .expect_err("mid-payload truncation must fail");
        assert_eq!(err.kind, "premature_eof");
        assert_eq!(err.consumed_raw_tokens, truncated_mid.len());
        assert_eq!(err.expected_payload_class.as_deref(), Some("integer"));

        // Trailing token after complete stream.
        let mut trailing = complete_boolean_stream(&model, "true");
        trailing.push(roles.node_start);
        let err = model
            .decode_complete(&trailing)
            .expect_err("trailing tokens must fail");
        assert_eq!(err.kind, "extra_trailing_token");
        assert_eq!(
            err.expected_payload_class, None,
            "non-payload errors leave expected_payload_class unset"
        );
    }

    #[test]
    fn mixed_integer_payload_requires_declared_integer_value_range() {
        use crate::grammar_ir::{
            GrammarField, GrammarFieldAlternative, GrammarIr, GrammarIrProvenance, GrammarRule,
            GrammarTerminalKind,
        };
        use crate::grammar_ir_structural_model::{
            GrammarStructuralTokenLayout, lower_grammar_ir_to_structural_model,
        };
        use crate::syntax_backend_boundary::SyntaxBackendBoundaryError;
        use crate::toy_grammar_frontend::ToyGrammarSpec;

        let mut grammar = GrammarIr::new(
            "mixed_integer_range_required_v0",
            GrammarIrProvenance {
                source_kind: "synthetic_mixed_integer_range".to_string(),
                source_path: Some("src/syntax_state_machine/raw_replay.rs".to_string()),
                source_sha256: None,
                metadata_path: None,
                metadata_sha256: None,
                parser: None,
            },
        );
        grammar.rules.push(GrammarRule {
            name: "wrapper".to_string(),
            node_kind: "wrapper".to_string(),
            fields: vec![GrammarField {
                name: "value".to_string(),
                required: true,
                multiple: false,
                alternatives: vec![
                    GrammarFieldAlternative {
                        node_kind: "child".to_string(),
                        terminal: GrammarTerminalKind::Nonterminal,
                    },
                    GrammarFieldAlternative {
                        node_kind: "number".to_string(),
                        terminal: GrammarTerminalKind::IntegerPayload,
                    },
                ],
            }],
        });
        grammar.rules.push(GrammarRule {
            name: "child".to_string(),
            node_kind: "child".to_string(),
            fields: vec![],
        });

        let mut layout = GrammarStructuralTokenLayout::from_toy_assignment(
            &ToyGrammarSpec::assignment_language_v1().layout,
        );
        layout.integer_value_range = None;
        let sidecar = lower_grammar_ir_to_structural_model(&grammar, layout)
            .expect("grammar without integer_value_range still lowers");
        let err: SyntaxBackendBoundaryError = CompiledSyntaxModel::from_sidecar(&sidecar)
            .expect_err("mixed IntegerPayload without GrammarIntegerValueRange must reject");
        assert_eq!(
            err.field(),
            "syntax_state_machine.mixed_payload.missing_integer_value_range"
        );
        let err_text = err.to_string();
        assert!(
            err_text.contains("owner_rule=wrapper"),
            "diagnostic must name owner rule: {err_text}"
        );
        assert!(
            err_text.contains("field=value"),
            "diagnostic must name field: {err_text}"
        );
        assert!(
            err_text.contains("node_kind=number"),
            "diagnostic must name node kind: {err_text}"
        );
    }

    #[test]
    fn mixed_incremental_and_complete_replay_agree() {
        let model = compiled_mixed_model();
        for stream in [
            complete_child_stream(&model),
            complete_boolean_stream(&model, "true"),
            complete_boolean_stream(&model, "false"),
            complete_integer_stream(&model, 255),
            complete_utf8_stream(&model, "café"),
        ] {
            let complete = model
                .decode_complete_tree(&stream)
                .expect("complete replay");
            assert_eq!(complete.state.status, RawSyntaxStatus::Complete);
            assert_eq!(complete.state.consumed_raw_tokens, stream.len());

            let mut walker = model.begin_stream();
            for (index, &token) in stream.iter().enumerate() {
                let delta = walker.advance(token).unwrap_or_else(|err| {
                    panic!("incremental advance failed at {index} token={token}: {err:?}")
                });
                assert_eq!(delta.consumed_raw_tokens, index + 1);
                let prefix_state = model
                    .state_from_prefix(&stream[..=index])
                    .expect("prefix state");
                let snap = walker.state_snapshot();
                assert_eq!(snap.stack_depth, prefix_state.stack_depth);
                assert_eq!(snap.active_payload_kind, prefix_state.active_payload_kind);
                assert_eq!(snap.admissible, prefix_state.admissible);
            }
            assert!(walker.eof_valid());
            assert_eq!(walker.consumed_raw_tokens(), stream.len());
        }
    }

    #[test]
    fn mixed_elide_expand_round_trip() {
        let model = compiled_mixed_model();

        // Boolean cases: every token except the choice is mask-forced, so the
        // elided stream must be exactly the single choice token. Without this
        // content assertion, a no-op elide_forced_raw_tokens would still
        // satisfy expand(elide(stream)) == stream.
        for node_kind in ["true", "false"] {
            let stream = complete_boolean_stream(&model, node_kind);
            let choice = choice_token(&model, node_kind);
            let elided = model
                .elide_forced_raw_tokens(&stream)
                .expect("elide boolean");
            assert_eq!(
                elided,
                vec![choice],
                "boolean elided stream must be exactly the choice token for {node_kind}; \
                 stream={stream:?} elided={elided:?}"
            );
            assert!(
                elided.len() < stream.len(),
                "boolean elision must shrink the stream"
            );
            let expanded = model
                .expand_elided_raw_tokens(&elided)
                .expect("expand boolean elided");
            assert_eq!(expanded, stream);
            let state = model
                .strict_decode_complete_elided(&elided)
                .expect("elided strict decode boolean");
            assert_eq!(state.status, RawSyntaxStatus::Complete);
        }

        // Non-boolean cases still require real elision (strict shrink) plus
        // round-trip identity; exact elided content is case-specific.
        for stream in [
            complete_child_stream(&model),
            complete_integer_stream(&model, 0),
            complete_utf8_stream(&model, "ok"),
        ] {
            let elided = model
                .elide_forced_raw_tokens(&stream)
                .expect("elide expanded");
            assert!(
                elided.len() < stream.len(),
                "elision must drop mask-forced tokens; stream={stream:?} elided={elided:?}"
            );
            assert!(
                !elided.is_empty(),
                "complete mixed stream must retain at least the free-choice token(s)"
            );
            let expanded = model
                .expand_elided_raw_tokens(&elided)
                .expect("expand elided");
            assert_eq!(
                expanded, stream,
                "elide/expand must preserve the expanded stream"
            );
            let state = model
                .strict_decode_complete_elided(&elided)
                .expect("elided strict decode");
            assert_eq!(state.status, RawSyntaxStatus::Complete);
        }
    }

    #[test]
    fn mixed_nonterminal_complete_decoded_tree() {
        let model = compiled_mixed_model();
        let stream = complete_child_stream(&model);
        let tree = model.decode_complete_tree(&stream).expect("child complete");
        assert_eq!(tree.state.consumed_raw_tokens, stream.len());
        let root = tree.decoded_node_tree().expect("node tree");
        assert_eq!(root.node_kind, "wrapper");
        match &root.field("value").unwrap().values[0] {
            RawSyntaxDecodedValue::Node(child) => {
                assert_eq!(child.node_kind, "child");
                assert!(child.fields.is_empty());
            }
            other => panic!("expected child node, got {other:?}"),
        }
    }

    #[test]
    fn homogeneous_payload_field_still_uses_generic_terminal() {
        // Legacy toy assignment.value is multi-payload without Nonterminal;
        // generic terminal must remain the start token.
        use crate::grammar_ir_structural_model::{
            GrammarStructuralTokenLayout, lower_grammar_ir_to_structural_model,
        };
        use crate::toy_grammar_frontend::ToyGrammarSpec;

        let spec = ToyGrammarSpec::assignment_language_v1();
        let sidecar = lower_grammar_ir_to_structural_model(
            &spec.to_grammar_ir(),
            GrammarStructuralTokenLayout::from_toy_assignment(&spec.layout),
        )
        .expect("toy lower");
        let model = CompiledSyntaxModel::from_sidecar(&sidecar).expect("toy compile");
        let roles = model.token_roles();
        // program.node_start, field_start statements, node_start assignment,
        // field_start name — then terminal for identifier is not the focus;
        // open assignment.value after encoding name as empty is awkward.
        // Instead: check ModelFacts path via admissible after building a
        // prefix that opens assignment.value. Use manual structural walk:
        // node_start, field_start, node_start (assignment), field_start (name),
        // terminal, payload_end (empty id), field_end, field_start (value).
        let tokens = vec![
            roles.node_start,
            roles.field_start,
            roles.node_start,  // assignment (single NT child)
            roles.field_start, // name
            roles.terminal,
            roles.payload_end, // empty identifier (valid UTF-8)
            roles.field_end,
            roles.field_start, // value
        ];
        let admissible = model
            .admissible_next_raw_tokens(&tokens)
            .expect("open assignment.value");
        assert!(
            admissible.finite_raw_token_ids.contains(&roles.terminal),
            "homogeneous payload field must still admit generic terminal; got {:?}",
            admissible.finite_raw_token_ids
        );
    }

    #[test]
    fn mixed_choice_tokens_are_below_vocab_and_pad_separated() {
        let model = compiled_mixed_model();
        let roles = model.token_roles();
        for kind in ["child", "number", "text", "true", "false"] {
            let token = choice_token(&model, kind);
            assert!(token < roles.vocab_size);
            assert_ne!(token, roles.pad_token);
            assert!(token >= roles.structural_range.start);
            assert!(token < roles.structural_range.end_exclusive);
            // Outside payload bands.
            assert!(token < roles.utf8_range.start || token >= roles.utf8_range.end_exclusive);
            assert!(
                token < roles.integer_range.start || token >= roles.integer_range.end_exclusive
            );
        }
        // Fixed structural roles remain distinct from choice tokens.
        let fixed = [
            roles.node_start,
            roles.node_end,
            roles.field_start,
            roles.field_end,
            roles.terminal,
            roles.payload_end,
            roles.boolean_false,
            roles.boolean_true,
        ];
        for kind in ["child", "number", "text", "true", "false"] {
            let token = choice_token(&model, kind);
            assert!(!fixed.contains(&token));
        }
    }

    #[test]
    fn repeated_mixed_field_readmits_choices_after_each_value() {
        use super::super::model_facts::mixed_payload_witness::repeated_mixed_field_sidecar;

        let model = CompiledSyntaxModel::from_sidecar(&repeated_mixed_field_sidecar())
            .expect("repeated mixed-field witness should compile");
        let roles = model.token_roles();
        let true_tok = choice_token(&model, "true");
        let false_tok = choice_token(&model, "false");
        let number_tok = choice_token(&model, "number");
        let child_tok = choice_token(&model, "child");
        let text_tok = choice_token(&model, "text");
        let expected_choices: BTreeSet<u32> =
            [true_tok, false_tok, number_tok, child_tok, text_tok]
                .into_iter()
                .collect();

        // Open wrapper.values (required, multiple, mixed).
        let mut prefix = vec![roles.node_start, roles.field_start];
        let open_mask = model
            .admissible_next_raw_tokens(&prefix)
            .expect("open repeated mixed field");
        let open_ids: BTreeSet<u32> = open_mask.finite_raw_token_ids.iter().copied().collect();
        assert_eq!(
            open_ids, expected_choices,
            "first value: mask is exactly the alternative choice tokens"
        );
        assert!(
            !open_ids.contains(&roles.field_end),
            "required repeated field must not offer field_end before any value"
        );

        // choice → value (zero-width boolean), then choices must re-admit with field_end.
        prefix.push(true_tok);
        let after_first = model
            .admissible_next_raw_tokens(&prefix)
            .expect("after first boolean value");
        let after_first_ids: BTreeSet<u32> =
            after_first.finite_raw_token_ids.iter().copied().collect();
        assert!(
            expected_choices.iter().all(|t| after_first_ids.contains(t)),
            "after a completed value the mask must re-admit every alternative choice token; got {after_first_ids:?}"
        );
        assert!(
            after_first_ids.contains(&roles.field_end),
            "after a completed value field_end must also be admissible"
        );

        // Second value: integer body, then re-admit again.
        prefix.push(number_tok);
        prefix.push(roles.integer_range.start + 7);
        prefix.push(roles.payload_end);
        let after_second = model
            .admissible_next_raw_tokens(&prefix)
            .expect("after second integer value");
        let after_second_ids: BTreeSet<u32> =
            after_second.finite_raw_token_ids.iter().copied().collect();
        assert!(
            expected_choices
                .iter()
                .all(|t| after_second_ids.contains(t)),
            "after each completed value choice tokens must re-admit; got {after_second_ids:?}"
        );
        assert!(after_second_ids.contains(&roles.field_end));

        // Complete stream: true, number(7), false — three values, then close.
        let stream = vec![
            roles.node_start,
            roles.field_start,
            true_tok,
            number_tok,
            roles.integer_range.start + 7,
            roles.payload_end,
            false_tok,
            roles.field_end,
            roles.node_end,
        ];
        let tree = model
            .decode_complete_tree(&stream)
            .expect("repeated mixed complete stream");
        assert_eq!(tree.state.status, RawSyntaxStatus::Complete);
        assert_eq!(
            tree.state.consumed_raw_tokens,
            stream.len(),
            "consumed_raw_tokens must equal stream length"
        );
        let root = tree.decoded_node_tree().expect("decoded node tree");
        let values = &root.field("values").expect("values field").values;
        assert_eq!(values.len(), 3, "three completed values expected");
        match &values[0] {
            RawSyntaxDecodedValue::Terminal(t) => {
                assert_eq!(t.node_kind, "true");
                assert_eq!(t.payload, "true");
            }
            other => panic!("values[0] expected true terminal, got {other:?}"),
        }
        match &values[1] {
            RawSyntaxDecodedValue::Terminal(t) => {
                assert_eq!(t.node_kind, "number");
                assert_eq!(t.payload, "7");
            }
            other => panic!("values[1] expected number terminal, got {other:?}"),
        }
        match &values[2] {
            RawSyntaxDecodedValue::Terminal(t) => {
                assert_eq!(t.node_kind, "false");
                assert_eq!(t.payload, "false");
            }
            other => panic!("values[2] expected false terminal, got {other:?}"),
        }

        // Also exercise Nonterminal descent as a repeated value: choice → child
        // → node_end completes one value and re-admits choices.
        let mut child_prefix = vec![roles.node_start, roles.field_start, child_tok];
        let in_child = model
            .state_from_prefix(&child_prefix)
            .expect("child choice descends");
        assert_eq!(in_child.active_rule.as_deref(), Some("child"));
        child_prefix.push(roles.node_end);
        let after_child = model
            .admissible_next_raw_tokens(&child_prefix)
            .expect("after completed child value");
        let after_child_ids: BTreeSet<u32> =
            after_child.finite_raw_token_ids.iter().copied().collect();
        assert!(
            expected_choices.iter().all(|t| after_child_ids.contains(t)),
            "after a completed Nonterminal value choice tokens must re-admit"
        );
        assert!(after_child_ids.contains(&roles.field_end));
    }

    #[test]
    fn nested_mixed_field_inside_child_rule_round_trips() {
        use super::super::model_facts::mixed_payload_witness::nested_mixed_field_sidecar;

        let model = CompiledSyntaxModel::from_sidecar(&nested_mixed_field_sidecar())
            .expect("nested mixed-field witness should compile");
        let roles = model.token_roles();
        let true_tok = choice_token(&model, "true");
        let number_tok = choice_token(&model, "number");
        let leaf_tok = choice_token(&model, "leaf");

        // Descend wrapper → container via homogeneous single-alt node_start,
        // then open the nested mixed field.
        let mut prefix = vec![
            roles.node_start,  // wrapper
            roles.field_start, // container field
            roles.node_start,  // descend into container (single-alt Nonterminal)
            roles.field_start, // inner mixed field
        ];
        let nested_open = model
            .state_from_prefix(&prefix)
            .expect("should open nested mixed field");
        assert_eq!(nested_open.stack_depth, 2);
        assert_eq!(nested_open.active_rule.as_deref(), Some("container"));
        assert_eq!(nested_open.active_field.as_deref(), Some("inner"));
        let nested_mask: BTreeSet<u32> = nested_open
            .admissible
            .finite_raw_token_ids
            .iter()
            .copied()
            .collect();
        let expected_choices: BTreeSet<u32> = ["leaf", "number", "text", "true", "false"]
            .into_iter()
            .map(|kind| choice_token(&model, kind))
            .collect();
        assert_eq!(
            nested_mask, expected_choices,
            "nested mixed field mask must be exactly its alternative choice tokens"
        );

        // Complete stream with an inner boolean true.
        let stream_true = vec![
            roles.node_start,
            roles.field_start,
            roles.node_start,
            roles.field_start,
            true_tok,
            roles.field_end, // inner
            roles.node_end,  // container
            roles.field_end, // container field
            roles.node_end,  // wrapper
        ];
        let tree_true = model
            .decode_complete_tree(&stream_true)
            .expect("nested true complete stream");
        assert_eq!(tree_true.state.status, RawSyntaxStatus::Complete);
        assert_eq!(
            tree_true.state.consumed_raw_tokens,
            stream_true.len(),
            "consumed_raw_tokens must equal stream length"
        );
        let root = tree_true.decoded_node_tree().expect("decoded tree");
        assert_eq!(root.node_kind, "wrapper");
        let container = match &root.field("container").expect("container field").values[0] {
            RawSyntaxDecodedValue::Node(node) => node,
            other => panic!("expected container node, got {other:?}"),
        };
        assert_eq!(container.node_kind, "container");
        match &container.field("inner").expect("inner field").values[0] {
            RawSyntaxDecodedValue::Terminal(t) => {
                assert_eq!(t.node_kind, "true");
                assert_eq!(t.payload, "true");
            }
            other => panic!("expected nested true terminal, got {other:?}"),
        }

        // Integer payload alternative through the nested mixed field.
        let stream_number = vec![
            roles.node_start,
            roles.field_start,
            roles.node_start,
            roles.field_start,
            number_tok,
            roles.integer_range.start + 42,
            roles.payload_end,
            roles.field_end,
            roles.node_end,
            roles.field_end,
            roles.node_end,
        ];
        let tree_number = model
            .decode_complete_tree(&stream_number)
            .expect("nested number complete stream");
        assert_eq!(tree_number.state.consumed_raw_tokens, stream_number.len());
        let root = tree_number.decoded_node_tree().expect("decoded tree");
        let container = match &root.field("container").unwrap().values[0] {
            RawSyntaxDecodedValue::Node(node) => node,
            other => panic!("expected container node, got {other:?}"),
        };
        match &container.field("inner").unwrap().values[0] {
            RawSyntaxDecodedValue::Terminal(t) => {
                assert_eq!(t.node_kind, "number");
                assert_eq!(t.payload, "42");
            }
            other => panic!("expected nested number terminal, got {other:?}"),
        }

        // Nested Nonterminal leaf: choice descends further then pops back.
        let stream_leaf = vec![
            roles.node_start,
            roles.field_start,
            roles.node_start,
            roles.field_start,
            leaf_tok,
            roles.node_end, // empty leaf
            roles.field_end,
            roles.node_end, // container
            roles.field_end,
            roles.node_end, // wrapper
        ];
        // Confirm intermediate stack depth after leaf choice.
        prefix.push(leaf_tok);
        let in_leaf = model
            .state_from_prefix(&prefix)
            .expect("leaf choice descends from nested mixed field");
        assert_eq!(in_leaf.stack_depth, 3);
        assert_eq!(in_leaf.active_rule.as_deref(), Some("leaf"));

        let tree_leaf = model
            .decode_complete_tree(&stream_leaf)
            .expect("nested leaf complete stream");
        assert_eq!(tree_leaf.state.consumed_raw_tokens, stream_leaf.len());
        let root = tree_leaf.decoded_node_tree().expect("decoded tree");
        let container = match &root.field("container").unwrap().values[0] {
            RawSyntaxDecodedValue::Node(node) => node,
            other => panic!("expected container node, got {other:?}"),
        };
        match &container.field("inner").unwrap().values[0] {
            RawSyntaxDecodedValue::Node(leaf) => {
                assert_eq!(leaf.node_kind, "leaf");
                assert!(leaf.fields.is_empty());
            }
            other => panic!("expected nested leaf node, got {other:?}"),
        }
    }
}
