//! Focused invalid raw-token stream builders for syntax state-machine replay.

use crate::syntax_backend_boundary::SyntaxBackendBoundaryError;
use crate::syntax_go_source_instance_projection::ProjectionRawTokenSegment;

use super::{RawReplayInput, ReplayTokenRoles, stream_from_segment_indices};

pub(super) struct MutationCaseInput {
    pub(super) case_id: &'static str,
    pub(super) input_kind: &'static str,
    pub(super) expected_error: &'static str,
    pub(super) stream: RawReplayInput,
}

pub(super) fn invalid_mutation_inputs(
    base_stream: &RawReplayInput,
    roles: &ReplayTokenRoles,
) -> Result<Vec<MutationCaseInput>, SyntaxBackendBoundaryError> {
    let mut inputs = vec![MutationCaseInput {
        case_id: "wrong_structural_token_for_active_state",
        input_kind: "mutated_raw_tokens",
        expected_error: "unexpected_structural_token",
        stream: mutate_wrong_structural_token(base_stream, roles),
    }];
    if let Some(stream) = mutate_missing_alternative_identity_token(base_stream, roles)? {
        inputs.push(MutationCaseInput {
            case_id: "missing_alternative_identity_token",
            input_kind: "mutated_raw_tokens",
            expected_error: "unexpected_structural_token",
            stream,
        });
    }
    inputs.extend([
        MutationCaseInput {
            case_id: "missing_required_field_before_node_end",
            input_kind: "mutated_raw_segments",
            expected_error: "missing_required_field",
            stream: mutate_missing_required_field(base_stream)?,
        },
        MutationCaseInput {
            case_id: "repeated_field_cardinality_violation",
            input_kind: "mutated_raw_segments",
            expected_error: "repeated_field_cardinality",
            stream: mutate_repeated_field_cardinality(base_stream)?,
        },
        MutationCaseInput {
            case_id: "invalid_payload_byte",
            input_kind: "mutated_raw_tokens",
            expected_error: "invalid_payload_token",
            stream: mutate_invalid_payload_byte(base_stream, roles)?,
        },
        MutationCaseInput {
            case_id: "invalid_payload_end_token",
            input_kind: "mutated_raw_tokens",
            expected_error: "invalid_payload_token",
            stream: mutate_invalid_payload_end_token(base_stream, roles)?,
        },
        MutationCaseInput {
            case_id: "premature_eof_inside_open_root",
            input_kind: "truncated_raw_tokens",
            expected_error: "premature_eof",
            stream: mutate_premature_eof(base_stream)?,
        },
        MutationCaseInput {
            case_id: "extra_trailing_token_after_root_close",
            input_kind: "mutated_raw_tokens",
            expected_error: "extra_trailing_token",
            stream: mutate_extra_trailing_token(base_stream, roles),
        },
        MutationCaseInput {
            case_id: "pad_token_inside_raw_content",
            input_kind: "mutated_raw_tokens",
            expected_error: "pad_token_in_raw_stream",
            stream: mutate_pad_token(base_stream, roles),
        },
        MutationCaseInput {
            case_id: "token_id_out_of_vocab",
            input_kind: "mutated_raw_tokens",
            expected_error: "token_id_out_of_vocab",
            stream: mutate_token_id_out_of_vocab(base_stream, roles),
        },
    ]);
    Ok(inputs)
}

fn mutate_wrong_structural_token(
    base_stream: &RawReplayInput,
    roles: &ReplayTokenRoles,
) -> RawReplayInput {
    let mut stream = base_stream.clone();
    stream.raw_token_ids[0] = roles.node_end;
    stream
}

fn mutate_missing_alternative_identity_token(
    base_stream: &RawReplayInput,
    roles: &ReplayTokenRoles,
) -> Result<Option<RawReplayInput>, SyntaxBackendBoundaryError> {
    let mut stream = base_stream.clone();
    let Some(raw_token_offset) = stream
        .raw_token_segments
        .iter()
        .find(|segment| {
            segment.event_kind == "node_start"
                && stream.raw_token_ids.get(segment.raw_token_offset).copied()
                    != Some(roles.node_start)
        })
        .map(|segment| segment.raw_token_offset)
    else {
        return Ok(None);
    };
    stream.raw_token_ids[raw_token_offset] = roles.node_start;
    Ok(Some(stream))
}

fn mutate_missing_required_field(
    base_stream: &RawReplayInput,
) -> Result<RawReplayInput, SyntaxBackendBoundaryError> {
    let root_end_index = base_stream
        .raw_token_segments
        .iter()
        .rposition(|segment| {
            segment.event_kind == "node_end" && segment.owner_rule.as_deref() == Some("File")
        })
        .ok_or_else(|| {
            SyntaxBackendBoundaryError::new(
                "replay.mutation.missing_required_field",
                "source-instance stream must contain a root File node_end",
            )
        })?;
    stream_from_segment_indices(base_stream, &[0, root_end_index])
}

fn mutate_repeated_field_cardinality(
    base_stream: &RawReplayInput,
) -> Result<RawReplayInput, SyntaxBackendBoundaryError> {
    let field_start_index = base_stream
        .raw_token_segments
        .iter()
        .position(|segment| {
            segment.event_kind == "field_start"
                && segment.owner_rule.as_deref() == Some("File")
                && segment.owner_field.as_deref() == Some("Name")
        })
        .ok_or_else(|| {
            SyntaxBackendBoundaryError::new(
                "replay.mutation.repeated_field_cardinality",
                "source-instance stream must contain File.Name field_start",
            )
        })?;
    let field_end_index = base_stream
        .raw_token_segments
        .iter()
        .enumerate()
        .skip(field_start_index + 1)
        .find(|(_, segment)| {
            segment.event_kind == "field_end"
                && segment.owner_rule.as_deref() == Some("File")
                && segment.owner_field.as_deref() == Some("Name")
        })
        .map(|(index, _)| index)
        .ok_or_else(|| {
            SyntaxBackendBoundaryError::new(
                "replay.mutation.repeated_field_cardinality",
                "source-instance stream must contain File.Name field_end",
            )
        })?;
    let mut indices = Vec::new();
    indices.extend(0..=field_start_index);
    indices.extend((field_start_index + 1)..field_end_index);
    indices.extend((field_start_index + 1)..field_end_index);
    indices.extend(field_end_index..base_stream.raw_token_segments.len());
    stream_from_segment_indices(base_stream, &indices)
}

fn mutate_invalid_payload_byte(
    base_stream: &RawReplayInput,
    roles: &ReplayTokenRoles,
) -> Result<RawReplayInput, SyntaxBackendBoundaryError> {
    let mut stream = base_stream.clone();
    let payload_segment = first_variable_payload_segment(base_stream)?;
    stream.raw_token_ids[payload_segment.raw_token_offset + 1] = roles.node_start;
    Ok(stream)
}

fn mutate_invalid_payload_end_token(
    base_stream: &RawReplayInput,
    roles: &ReplayTokenRoles,
) -> Result<RawReplayInput, SyntaxBackendBoundaryError> {
    let mut stream = base_stream.clone();
    let payload_segment = first_variable_payload_segment(base_stream)?;
    let end_offset = payload_segment.raw_token_offset + payload_segment.raw_token_count - 1;
    stream.raw_token_ids[end_offset] = roles.utf8_range.start;
    Ok(stream)
}

fn mutate_premature_eof(
    base_stream: &RawReplayInput,
) -> Result<RawReplayInput, SyntaxBackendBoundaryError> {
    let last_index = base_stream
        .raw_token_segments
        .len()
        .checked_sub(1)
        .ok_or_else(|| {
            SyntaxBackendBoundaryError::new(
                "replay.mutation.premature_eof",
                "source-instance stream must contain at least one segment",
            )
        })?;
    let indices = (0..last_index).collect::<Vec<_>>();
    stream_from_segment_indices(base_stream, &indices)
}

fn mutate_extra_trailing_token(
    base_stream: &RawReplayInput,
    roles: &ReplayTokenRoles,
) -> RawReplayInput {
    let mut stream = base_stream.clone();
    stream.raw_token_ids.push(roles.node_start);
    stream
}

fn mutate_pad_token(base_stream: &RawReplayInput, roles: &ReplayTokenRoles) -> RawReplayInput {
    let mut stream = base_stream.clone();
    stream.raw_token_ids[0] = roles.pad_token;
    stream
}

fn mutate_token_id_out_of_vocab(
    base_stream: &RawReplayInput,
    roles: &ReplayTokenRoles,
) -> RawReplayInput {
    let mut stream = base_stream.clone();
    stream.raw_token_ids[0] = roles.vocab_size;
    stream
}

fn first_variable_payload_segment(
    base_stream: &RawReplayInput,
) -> Result<&ProjectionRawTokenSegment, SyntaxBackendBoundaryError> {
    base_stream
        .raw_token_segments
        .iter()
        .find(|segment| {
            segment.event_kind == "terminal"
                && segment.raw_token_count > 2
                && !segment
                    .payload_kind
                    .as_deref()
                    .unwrap_or("")
                    .starts_with("boolean_payload:")
        })
        .ok_or_else(|| {
            SyntaxBackendBoundaryError::new(
                "replay.mutation.payload",
                "source-instance stream must contain a variable-length payload segment",
            )
        })
}
