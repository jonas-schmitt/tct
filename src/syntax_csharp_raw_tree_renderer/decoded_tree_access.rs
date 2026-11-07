use crate::syntax_backend_boundary::SyntaxBackendBoundaryError;
use crate::syntax_state_machine::{
    RawSyntaxDecodedField, RawSyntaxDecodedNode, RawSyntaxDecodedTerminal, RawSyntaxDecodedValue,
};

pub(super) fn single_node_field<'a>(
    path: &'static str,
    node: &'a RawSyntaxDecodedNode,
    field_name: &str,
    expected_node_kind: &str,
) -> Result<&'a RawSyntaxDecodedNode, SyntaxBackendBoundaryError> {
    let field = required_present_field(path, node, field_name)?;
    if field.values.len() != 1 {
        return Err(render_error(
            path,
            format!(
                "expected exactly one {expected_node_kind} value, got {}",
                field.values.len()
            ),
        ));
    }
    let RawSyntaxDecodedValue::Node(child) = &field.values[0] else {
        return Err(render_error(
            path,
            format!("expected {expected_node_kind} node value"),
        ));
    };
    require_node(path, child, expected_node_kind)?;
    Ok(child)
}

pub(super) fn single_node_field_any<'a>(
    path: &'static str,
    node: &'a RawSyntaxDecodedNode,
    field_name: &str,
    expected_node_kinds: &[&str],
) -> Result<&'a RawSyntaxDecodedNode, SyntaxBackendBoundaryError> {
    let nodes = node_values_field(path, node, field_name, expected_node_kinds)?;
    if nodes.len() != 1 {
        return Err(render_error(
            path,
            format!("expected exactly one node value, got {}", nodes.len()),
        ));
    }
    Ok(nodes[0])
}

pub(super) fn optional_node_field_any<'a>(
    path: &'static str,
    node: &'a RawSyntaxDecodedNode,
    field_name: &str,
    expected_node_kinds: &[&str],
) -> Result<Option<&'a RawSyntaxDecodedNode>, SyntaxBackendBoundaryError> {
    let field = node
        .field(field_name)
        .ok_or_else(|| render_error(path, format!("missing field {field_name}")))?;
    if field.missing {
        return Ok(None);
    }
    if field.repeated_empty {
        return Err(render_error(
            path,
            "optional single field cannot be repeated_empty",
        ));
    }
    let nodes = node_values_from_present_field(path, field, expected_node_kinds)?;
    if nodes.len() != 1 {
        return Err(render_error(
            path,
            format!("expected zero or one node value, got {}", nodes.len()),
        ));
    }
    Ok(Some(nodes[0]))
}

pub(super) fn node_values_field<'a>(
    path: &'static str,
    node: &'a RawSyntaxDecodedNode,
    field_name: &str,
    expected_node_kinds: &[&str],
) -> Result<Vec<&'a RawSyntaxDecodedNode>, SyntaxBackendBoundaryError> {
    let field = required_present_field(path, node, field_name)?;
    node_values_from_present_field(path, field, expected_node_kinds)
}

fn node_values_from_present_field<'a>(
    path: &'static str,
    field: &'a RawSyntaxDecodedField,
    expected_node_kinds: &[&str],
) -> Result<Vec<&'a RawSyntaxDecodedNode>, SyntaxBackendBoundaryError> {
    let mut nodes = Vec::with_capacity(field.values.len());
    for value in &field.values {
        let RawSyntaxDecodedValue::Node(child) = value else {
            return Err(render_error(path, "expected node field value"));
        };
        if !expected_node_kinds
            .iter()
            .any(|expected| child.node_kind == *expected)
        {
            return Err(render_error(
                path,
                format!(
                    "expected one of {:?}, got {}",
                    expected_node_kinds, child.node_kind
                ),
            ));
        }
        nodes.push(child.as_ref());
    }
    Ok(nodes)
}

pub(super) fn single_terminal_payload(
    path: &'static str,
    node: &RawSyntaxDecodedNode,
    field_name: &str,
    expected_node_kind: &str,
    expected_terminal: &str,
) -> Result<String, SyntaxBackendBoundaryError> {
    single_terminal_payload_any(
        path,
        node,
        field_name,
        &[expected_node_kind],
        expected_terminal,
    )
}

pub(super) fn single_terminal_payload_any(
    path: &'static str,
    node: &RawSyntaxDecodedNode,
    field_name: &str,
    expected_node_kinds: &[&str],
    expected_terminal: &str,
) -> Result<String, SyntaxBackendBoundaryError> {
    let field = required_present_field(path, node, field_name)?;
    if field.values.len() != 1 {
        return Err(render_error(
            path,
            format!(
                "expected exactly one of {:?} terminal, got {}",
                expected_node_kinds,
                field.values.len()
            ),
        ));
    }
    let RawSyntaxDecodedValue::Terminal(terminal) = &field.values[0] else {
        return Err(render_error(
            path,
            format!("expected one of {expected_node_kinds:?} terminal value"),
        ));
    };
    require_terminal_any(path, terminal, expected_node_kinds, expected_terminal)?;
    if terminal.payload.is_empty() {
        return Err(render_error(path, "terminal payload must not be empty"));
    }
    Ok(terminal.payload.clone())
}

fn required_present_field<'a>(
    path: &'static str,
    node: &'a RawSyntaxDecodedNode,
    field_name: &str,
) -> Result<&'a RawSyntaxDecodedField, SyntaxBackendBoundaryError> {
    let field = node
        .field(field_name)
        .ok_or_else(|| render_error(path, format!("missing field {field_name}")))?;
    if field.missing || field.repeated_empty {
        return Err(render_error(
            path,
            "field must contain a rendered value for the C# raw tree renderer",
        ));
    }
    Ok(field)
}

pub(super) fn require_node(
    path: &'static str,
    node: &RawSyntaxDecodedNode,
    expected_node_kind: &str,
) -> Result<(), SyntaxBackendBoundaryError> {
    if node.node_kind != expected_node_kind {
        return Err(render_error(
            path,
            format!(
                "expected node kind {expected_node_kind}, got {}",
                node.node_kind
            ),
        ));
    }
    Ok(())
}

fn require_terminal_any(
    path: &'static str,
    terminal: &RawSyntaxDecodedTerminal,
    expected_node_kinds: &[&str],
    expected_terminal: &str,
) -> Result<(), SyntaxBackendBoundaryError> {
    if !terminal_node_kind_matches(&terminal.node_kind, expected_node_kinds) {
        return Err(render_error(
            path,
            format!(
                "expected one of {:?} terminal node kind, got {}",
                expected_node_kinds, terminal.node_kind
            ),
        ));
    }
    if terminal.terminal != expected_terminal
        && payload_kind_alias(expected_terminal) != Some(terminal.terminal.as_str())
    {
        return Err(render_error(
            path,
            format!(
                "expected terminal kind {expected_terminal}, got {}",
                terminal.terminal
            ),
        ));
    }
    Ok(())
}

fn terminal_node_kind_matches(actual: &str, expected_node_kinds: &[&str]) -> bool {
    actual
        .split('|')
        .any(|actual_part| expected_node_kinds.contains(&actual_part))
}

fn payload_kind_alias(terminal: &str) -> Option<&'static str> {
    match terminal {
        "identifier_payload" => Some("identifier"),
        "string_payload" => Some("string"),
        "integer_payload" => Some("integer"),
        _ => None,
    }
}

pub(super) fn csharp_string_literal(value: &str) -> String {
    let mut rendered = String::with_capacity(value.len() + 2);
    rendered.push('"');
    for ch in value.chars() {
        match ch {
            '\\' => rendered.push_str("\\\\"),
            '"' => rendered.push_str("\\\""),
            '\n' => rendered.push_str("\\n"),
            '\r' => rendered.push_str("\\r"),
            '\t' => rendered.push_str("\\t"),
            _ => rendered.push(ch),
        }
    }
    rendered.push('"');
    rendered
}

pub(super) fn require_string_vec(
    path: &'static str,
    actual: &[String],
    expected: &[&str],
) -> Result<(), SyntaxBackendBoundaryError> {
    let actual = actual.iter().map(String::as_str).collect::<Vec<_>>();
    require_string_slice(path, &actual, expected)
}

pub(super) fn require_string(
    path: &'static str,
    actual: &str,
    expected: &str,
) -> Result<(), SyntaxBackendBoundaryError> {
    if actual != expected {
        return Err(render_error(
            path,
            format!("expected {expected}, got {actual}"),
        ));
    }
    Ok(())
}

pub(super) fn require_string_slice(
    path: &'static str,
    actual: &[&str],
    expected: &[&str],
) -> Result<(), SyntaxBackendBoundaryError> {
    if actual != expected {
        return Err(render_error(
            path,
            format!("expected {:?}, got {:?}", expected, actual),
        ));
    }
    Ok(())
}

pub(super) fn require_pair_slice(
    path: &'static str,
    actual: &[(&str, &str)],
    expected: &[(&str, &str)],
) -> Result<(), SyntaxBackendBoundaryError> {
    if actual != expected {
        return Err(render_error(
            path,
            format!("expected {:?}, got {:?}", expected, actual),
        ));
    }
    Ok(())
}

pub(super) fn render_error(
    path: &'static str,
    message: impl Into<String>,
) -> SyntaxBackendBoundaryError {
    SyntaxBackendBoundaryError::new(
        "syntax_csharp_raw_tree_renderer",
        format!("{path}: {}", message.into()),
    )
}
