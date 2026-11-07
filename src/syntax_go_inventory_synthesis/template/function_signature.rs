use crate::syntax_backend_boundary::SyntaxBackendBoundaryError;
use crate::syntax_go_raw_token_stream::GoRawTokenStreamBuilder;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GoAstInventoryFunctionSignatureTemplate {
    NoParamsNoResults,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GoAstInventoryBlockTemplate {
    Empty,
}

pub(crate) fn validate_function_signature_template(
    signature: &GoAstInventoryFunctionSignatureTemplate,
) {
    match signature {
        GoAstInventoryFunctionSignatureTemplate::NoParamsNoResults => {}
    }
}

pub(crate) fn validate_block_template(body: &GoAstInventoryBlockTemplate) {
    match body {
        GoAstInventoryBlockTemplate::Empty => {}
    }
}

pub(crate) fn emit_func_lit_template_contents(
    stream: &mut GoRawTokenStreamBuilder<'_>,
    signature: &GoAstInventoryFunctionSignatureTemplate,
    body: &GoAstInventoryBlockTemplate,
) -> Result<(), SyntaxBackendBoundaryError> {
    emit_function_signature_node_field(stream, signature)?;
    emit_block_template(stream, body)
}

pub(crate) fn emit_function_signature_contents(
    stream: &mut GoRawTokenStreamBuilder<'_>,
    signature: &GoAstInventoryFunctionSignatureTemplate,
) -> Result<(), SyntaxBackendBoundaryError> {
    match signature {
        GoAstInventoryFunctionSignatureTemplate::NoParamsNoResults => {
            stream.boolean_terminal_field(true);
            stream.missing_field();
            stream.generic_node_field(|stream| {
                stream.repeated_empty_field();
                Ok(())
            })?;
            stream.missing_field();
            Ok(())
        }
    }
}

fn emit_function_signature_node_field(
    stream: &mut GoRawTokenStreamBuilder<'_>,
    signature: &GoAstInventoryFunctionSignatureTemplate,
) -> Result<(), SyntaxBackendBoundaryError> {
    stream.generic_node_field(|stream| emit_function_signature_contents(stream, signature))
}

fn emit_block_template(
    stream: &mut GoRawTokenStreamBuilder<'_>,
    body: &GoAstInventoryBlockTemplate,
) -> Result<(), SyntaxBackendBoundaryError> {
    match body {
        GoAstInventoryBlockTemplate::Empty => stream.generic_node_field(|stream| {
            stream.repeated_empty_field();
            Ok(())
        }),
    }
}
