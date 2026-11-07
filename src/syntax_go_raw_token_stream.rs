use crate::syntax_backend_boundary::SyntaxBackendBoundaryError;
use crate::syntax_state_machine::SyntaxTokenRoles;

/// Trailing synthesized comment/layout trivia fields appended to selected
/// rules of the checked go1.24.3 inventory grammar (see
/// `goAstSchemaSyntheticTriviaFields` in
/// `scripts/analysis/go_ast_schema_inventory.go`). Every trivia field is
/// optional and repeated, so the canonical empty encoding is one
/// `repeated_empty` field per entry at the end of the owning rule.
pub(crate) fn go_syntax_synthetic_trivia_field_names(node_kind: &str) -> &'static [&'static str] {
    match node_kind {
        "BlockStmt" | "CaseClause" | "CommClause" => {
            &["CommentBefore", "TrailingComment", "BlankLineBefore"]
        }
        "File" => &["CommentBefore", "NoBlankLineBefore"],
        "IfStmt" => &["TrailingComment"],
        "FieldList" => &["BlankLineBefore"],
        "CompositeLit" | "CallExpr" => &["LineBreakBefore"],
        _ => &[],
    }
}

pub(crate) struct GoRawTokenStreamBuilder<'a> {
    roles: &'a SyntaxTokenRoles,
    pub(crate) raw_token_ids: Vec<u32>,
}

impl<'a> GoRawTokenStreamBuilder<'a> {
    pub(crate) fn new(roles: &'a SyntaxTokenRoles) -> Self {
        Self {
            roles,
            raw_token_ids: Vec::new(),
        }
    }

    pub(crate) fn source_file(
        &mut self,
        package_name: &str,
        go_version: &str,
        decl: impl FnOnce(&mut Self) -> Result<(), SyntaxBackendBoundaryError>,
    ) -> Result<(), SyntaxBackendBoundaryError> {
        self.source_file_with_comments(
            package_name,
            go_version,
            |stream| {
                stream.repeated_empty_field();
                Ok(())
            },
            decl,
        )
    }

    pub(crate) fn source_file_with_comments(
        &mut self,
        package_name: &str,
        go_version: &str,
        comments: impl FnOnce(&mut Self) -> Result<(), SyntaxBackendBoundaryError>,
        decl: impl FnOnce(&mut Self) -> Result<(), SyntaxBackendBoundaryError>,
    ) -> Result<(), SyntaxBackendBoundaryError> {
        self.root_file_start();
        self.missing_field();
        self.ident_node_field(package_name)?;
        decl(self)?;
        comments(self)?;
        self.terminal_field(go_version);
        self.node_end();
        Ok(())
    }

    pub(crate) fn func_decl_with_return_expr(
        &mut self,
        identifier: &str,
        result_type: impl FnOnce(&mut Self) -> Result<(), SyntaxBackendBoundaryError>,
        return_expr: impl FnOnce(&mut Self) -> Result<(), SyntaxBackendBoundaryError>,
    ) -> Result<(), SyntaxBackendBoundaryError> {
        self.func_decl_with_result_body(identifier, result_type, |stream| {
            stream.alternative_node_field("ReturnStmt", |stream| return_expr(stream))
        })
    }

    pub(crate) fn func_decl_with_result_body(
        &mut self,
        identifier: &str,
        result_type: impl FnOnce(&mut Self) -> Result<(), SyntaxBackendBoundaryError>,
        body: impl FnOnce(&mut Self) -> Result<(), SyntaxBackendBoundaryError>,
    ) -> Result<(), SyntaxBackendBoundaryError> {
        self.alternative_node_field("FuncDecl", |stream| {
            stream.missing_field();
            stream.missing_field();
            stream.ident_node_field(identifier)?;
            stream.func_type_no_params_result(result_type)?;
            stream.generic_node_field(|stream| {
                body(stream)?;
                stream.synthetic_trivia_fields("BlockStmt");
                Ok(())
            })?;
            Ok(())
        })
    }

    pub(crate) fn func_decl_with_body(
        &mut self,
        identifier: &str,
        body: impl FnOnce(&mut Self) -> Result<(), SyntaxBackendBoundaryError>,
    ) -> Result<(), SyntaxBackendBoundaryError> {
        self.alternative_node_field("FuncDecl", |stream| {
            stream.missing_field();
            stream.missing_field();
            stream.ident_node_field(identifier)?;
            stream.func_type_no_params_no_result()?;
            stream.generic_node_field(|stream| {
                body(stream)?;
                stream.synthetic_trivia_fields("BlockStmt");
                Ok(())
            })?;
            Ok(())
        })
    }

    pub(crate) fn ident_node_field(
        &mut self,
        payload: &str,
    ) -> Result<(), SyntaxBackendBoundaryError> {
        self.generic_node_field(|stream| {
            stream.terminal_field(payload);
            Ok(())
        })
    }

    pub(crate) fn ident_alternative_node_field(
        &mut self,
        payload: &str,
    ) -> Result<(), SyntaxBackendBoundaryError> {
        self.alternative_node_field("Ident", |stream| {
            stream.terminal_field(payload);
            Ok(())
        })
    }

    pub(crate) fn basic_lit_alternative(
        &mut self,
        literal_kind: &str,
        literal_value: &str,
    ) -> Result<(), SyntaxBackendBoundaryError> {
        self.alternative_node_field("BasicLit", |stream| {
            stream.basic_lit_value(literal_kind, literal_value);
            Ok(())
        })
    }

    pub(crate) fn array_type_ident_alternative(
        &mut self,
        element_type: &str,
    ) -> Result<(), SyntaxBackendBoundaryError> {
        self.alternative_node_field("ArrayType", |stream| {
            stream.missing_field();
            stream.ident_alternative_node_field(element_type)
        })
    }

    pub(crate) fn array_type_ellipsis_ident_alternative(
        &mut self,
        element_type: &str,
    ) -> Result<(), SyntaxBackendBoundaryError> {
        self.alternative_node_field("ArrayType", |stream| {
            stream.alternative_node_field("Ellipsis", |stream| {
                stream.missing_field();
                Ok(())
            })?;
            stream.ident_alternative_node_field(element_type)
        })
    }

    pub(crate) fn chan_type_ident_alternative(
        &mut self,
        direction: &str,
        element_type: &str,
    ) -> Result<(), SyntaxBackendBoundaryError> {
        self.alternative_node_field("ChanType", |stream| {
            stream.terminal_field(direction);
            stream.ident_alternative_node_field(element_type)
        })
    }

    pub(crate) fn selector_call_alternative(
        &mut self,
        receiver: &str,
        selector: &str,
    ) -> Result<(), SyntaxBackendBoundaryError> {
        self.alternative_node_field("CallExpr", |stream| {
            stream.selector_call_fields(receiver, selector)
        })
    }

    pub(crate) fn expr_stmt_selector_call_alternative_value(
        &mut self,
        receiver: &str,
        selector: &str,
    ) -> Result<(), SyntaxBackendBoundaryError> {
        self.alternative_node_value("ExprStmt", |stream| {
            stream.selector_call_alternative(receiver, selector)
        })
    }

    pub(crate) fn empty_stmt_alternative_value(
        &mut self,
        implicit: bool,
    ) -> Result<(), SyntaxBackendBoundaryError> {
        self.alternative_node_value("EmptyStmt", |stream| {
            stream.boolean_terminal_field(implicit);
            Ok(())
        })
    }

    pub(crate) fn repeated_node_field(
        &mut self,
        populate: impl FnOnce(&mut Self) -> Result<(), SyntaxBackendBoundaryError>,
    ) -> Result<(), SyntaxBackendBoundaryError> {
        self.raw_token_ids.push(self.roles.field_start);
        populate(self)?;
        self.raw_token_ids.push(self.roles.field_end);
        Ok(())
    }

    pub(crate) fn alternative_node_field(
        &mut self,
        node_kind: &str,
        populate: impl FnOnce(&mut Self) -> Result<(), SyntaxBackendBoundaryError>,
    ) -> Result<(), SyntaxBackendBoundaryError> {
        self.raw_token_ids.push(self.roles.field_start);
        self.alternative_node_value(node_kind, populate)?;
        self.raw_token_ids.push(self.roles.field_end);
        Ok(())
    }

    pub(crate) fn alternative_node_value(
        &mut self,
        node_kind: &str,
        populate: impl FnOnce(&mut Self) -> Result<(), SyntaxBackendBoundaryError>,
    ) -> Result<(), SyntaxBackendBoundaryError> {
        let node_start = self.alternative_node_start_token(node_kind)?;
        self.raw_token_ids.push(node_start);
        populate(self)?;
        // The inventory grammar appends synthesized trivia metadata fields
        // to selected rules; witness populate closures describe only the
        // go/ast fields, so the empty trailing trivia fields are appended
        // here once per kind-aware node construction.
        self.synthetic_trivia_fields(node_kind);
        self.node_end();
        Ok(())
    }

    /// Append the empty trailing synthesized trivia fields for `node_kind`.
    /// Generic (non-kind-aware) node constructions of enriched rules must
    /// call this explicitly before `node_end`.
    pub(crate) fn synthetic_trivia_fields(&mut self, node_kind: &str) {
        for _ in 0..go_syntax_synthetic_trivia_field_names(node_kind).len() {
            self.repeated_empty_field();
        }
    }

    pub(crate) fn generic_node_value(
        &mut self,
        populate: impl FnOnce(&mut Self) -> Result<(), SyntaxBackendBoundaryError>,
    ) -> Result<(), SyntaxBackendBoundaryError> {
        self.raw_token_ids.push(self.roles.node_start);
        populate(self)?;
        self.node_end();
        Ok(())
    }

    pub(crate) fn terminal_field(&mut self, payload: &str) {
        self.raw_token_ids.push(self.roles.field_start);
        self.raw_token_ids.push(self.roles.terminal);
        self.push_utf8_payload(payload);
        self.raw_token_ids.push(self.roles.payload_end);
        self.raw_token_ids.push(self.roles.field_end);
    }

    pub(crate) fn missing_field(&mut self) {
        self.raw_token_ids.push(self.roles.field_start);
        self.raw_token_ids.push(self.roles.field_missing);
        self.raw_token_ids.push(self.roles.field_end);
    }

    fn func_type_no_params_result(
        &mut self,
        result_type: impl FnOnce(&mut Self) -> Result<(), SyntaxBackendBoundaryError>,
    ) -> Result<(), SyntaxBackendBoundaryError> {
        self.generic_node_field(|stream| {
            stream.boolean_terminal_field(true);
            stream.missing_field();
            stream.generic_node_field(|stream| {
                stream.repeated_empty_field();
                stream.synthetic_trivia_fields("FieldList");
                Ok(())
            })?;
            stream.generic_node_field(|stream| {
                stream.generic_node_field(|stream| {
                    stream.missing_field();
                    stream.repeated_empty_field();
                    result_type(stream)?;
                    stream.missing_field();
                    stream.missing_field();
                    Ok(())
                })?;
                stream.synthetic_trivia_fields("FieldList");
                Ok(())
            })?;
            Ok(())
        })
    }

    fn func_type_no_params_no_result(&mut self) -> Result<(), SyntaxBackendBoundaryError> {
        self.generic_node_field(|stream| {
            stream.boolean_terminal_field(true);
            stream.missing_field();
            stream.generic_node_field(|stream| {
                stream.repeated_empty_field();
                stream.synthetic_trivia_fields("FieldList");
                Ok(())
            })?;
            stream.missing_field();
            Ok(())
        })
    }

    fn root_file_start(&mut self) {
        self.raw_token_ids.push(self.roles.node_start);
    }

    pub(crate) fn generic_node_field(
        &mut self,
        populate: impl FnOnce(&mut Self) -> Result<(), SyntaxBackendBoundaryError>,
    ) -> Result<(), SyntaxBackendBoundaryError> {
        self.raw_token_ids.push(self.roles.field_start);
        self.raw_token_ids.push(self.roles.node_start);
        populate(self)?;
        self.node_end();
        self.raw_token_ids.push(self.roles.field_end);
        Ok(())
    }

    pub(crate) fn has_alternative_node_start_token(&self, node_kind: &str) -> bool {
        self.roles
            .node_start_alternative_tokens
            .contains_key(node_kind)
    }

    pub(crate) fn boolean_terminal_field(&mut self, value: bool) {
        self.raw_token_ids.push(self.roles.field_start);
        self.raw_token_ids.push(self.roles.terminal);
        self.raw_token_ids.push(if value {
            self.roles.boolean_true
        } else {
            self.roles.boolean_false
        });
        self.raw_token_ids.push(self.roles.field_end);
    }

    fn basic_lit_value(&mut self, literal_kind: &str, literal_value: &str) {
        self.terminal_field(literal_kind);
        self.terminal_field(literal_value);
    }

    fn selector_call_fields(
        &mut self,
        receiver: &str,
        selector: &str,
    ) -> Result<(), SyntaxBackendBoundaryError> {
        self.alternative_node_field("SelectorExpr", |stream| {
            stream.ident_alternative_node_field(receiver)?;
            stream.ident_node_field(selector)?;
            Ok(())
        })?;
        self.repeated_empty_field();
        self.boolean_terminal_field(false);
        Ok(())
    }

    pub(crate) fn repeated_empty_field(&mut self) {
        self.raw_token_ids.push(self.roles.field_start);
        self.raw_token_ids.push(self.roles.repeated_empty);
        self.raw_token_ids.push(self.roles.field_end);
    }

    fn node_end(&mut self) {
        self.raw_token_ids.push(self.roles.node_end);
    }

    fn alternative_node_start_token(
        &self,
        node_kind: &str,
    ) -> Result<u32, SyntaxBackendBoundaryError> {
        self.roles
            .node_start_alternative_tokens
            .get(node_kind)
            .copied()
            .ok_or_else(|| {
                SyntaxBackendBoundaryError::new(
                    "go_scratch_seed.alternative_node_start_token",
                    format!("missing alternative node-start token for {node_kind}"),
                )
            })
    }

    fn push_utf8_payload(&mut self, payload: &str) {
        for byte in payload.bytes() {
            self.raw_token_ids
                .push(self.roles.utf8_range.start + u32::from(byte));
        }
    }
}
