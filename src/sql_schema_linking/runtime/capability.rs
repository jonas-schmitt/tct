//! Capability report, text-constraint placeholders, piece map, and errors
//! for the SQL schema-linking stage-1 runtime.

use crate::sql_schema_linking::catalog::{SqlCatalogError, normalize_sha256_hex};
use crate::sql_schema_linking::codec::SqlCodecError;
use crate::syntax_state_machine::RawSyntaxError;
use serde::Serialize;
use std::collections::BTreeMap;

/// Runtime claim boundary (same compiler-only envelope as the SQL frontend).
pub const SQL_RUNTIME_CLAIM_BOUNDARY: &str = "compiler_sql_schema_linking_only";
/// Stable artifact / capability id for this runtime surface.
pub const SQL_RUNTIME_SURFACE_ID: &str = "sql_schema_linking_runtime_v0";
/// Reusable blocker code for text-prefix (A1/A2) unavailability.
pub const SQL_TEXT_CONSTRAINT_BLOCKER_CODE: &str =
    "sql_tree_stream_order_diverges_from_surface_text";

/// Immutable base-token-id → UTF-8/byte mapping bound by an exact tokenizer
/// SHA-256.
///
/// Prepared for A1/A2 text constraints. Construction validates UTF-8 piece
/// bytes and requires `tokenizer_hash` to be an exact 64-hex SHA-256 digest
/// (case-agnostic; stored lowercase). The runtime does not currently consume
/// pieces for masking (see capability report).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TextTokenPieceMap {
    pub tokenizer_hash: String,
    pieces: BTreeMap<u32, Vec<u8>>,
}

impl TextTokenPieceMap {
    /// Build from `(base_token_id, utf8_or_bytes)` pairs.
    ///
    /// Fails closed on a non-exact SHA-256 `tokenizer_hash` (must be 64 hex
    /// digits), invalid UTF-8 piece bytes, or duplicate base token ids (last
    /// write rejected rather than silent overwrite — callers must pass a
    /// unique map).
    pub fn new(
        tokenizer_hash: impl Into<String>,
        pieces: impl IntoIterator<Item = (u32, Vec<u8>)>,
    ) -> Result<Self, SqlRuntimeError> {
        let tokenizer_hash = tokenizer_hash.into();
        let tokenizer_hash = normalize_sha256_hex(&tokenizer_hash, "tokenizer_hash")
            .map_err(|e| SqlRuntimeError::new("token_pieces", e.to_string()))?;
        let mut map = BTreeMap::new();
        for (id, bytes) in pieces {
            if std::str::from_utf8(&bytes).is_err() {
                return Err(SqlRuntimeError::new(
                    "token_pieces",
                    format!("base token id {id} piece is not valid UTF-8"),
                ));
            }
            if map.insert(id, bytes).is_some() {
                return Err(SqlRuntimeError::new(
                    "token_pieces",
                    format!("duplicate base token id {id} in token_pieces"),
                ));
            }
        }
        Ok(Self {
            tokenizer_hash,
            pieces: map,
        })
    }

    pub fn tokenizer_hash(&self) -> &str {
        &self.tokenizer_hash
    }

    pub fn piece(&self, base_token_id: u32) -> Option<&[u8]> {
        self.pieces.get(&base_token_id).map(|v| v.as_slice())
    }

    pub fn len(&self) -> usize {
        self.pieces.len()
    }

    pub fn is_empty(&self) -> bool {
        self.pieces.is_empty()
    }
}

/// Placeholder type for A1/A2 text constraint state.
///
/// Construction is unreachable: [`super::SqlSchemaLinkingRuntime::begin_text_constraint`]
/// always returns [`SqlRuntimeError`] with phase `text_constraint`.
#[derive(Debug, Clone)]
pub struct SqlTextConstraintState {
    pub(super) _private: (),
}

impl SqlTextConstraintState {
    pub fn allowed_token_ids(&self) -> Result<Vec<u32>, SqlRuntimeError> {
        Err(SqlRuntimeError::text_constraint_unavailable("state"))
    }

    pub fn apply(&mut self, _token: u32) -> Result<(), SqlRuntimeError> {
        Err(SqlRuntimeError::text_constraint_unavailable("state"))
    }

    pub fn is_complete(&self) -> Result<bool, SqlRuntimeError> {
        Err(SqlRuntimeError::text_constraint_unavailable("state"))
    }

    pub fn forced_suffix_token_count(&self) -> Result<usize, SqlRuntimeError> {
        Err(SqlRuntimeError::text_constraint_unavailable("state"))
    }

    pub fn jump_forward_count(&self) -> Result<usize, SqlRuntimeError> {
        Err(SqlRuntimeError::text_constraint_unavailable("state"))
    }

    pub fn scope_metadata(&self) -> Result<SqlTextScopeMetadata, SqlRuntimeError> {
        Err(SqlRuntimeError::text_constraint_unavailable("state"))
    }
}

/// Scope metadata surface for A2 (unused while text constraints are blocked).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct SqlTextScopeMetadata {
    pub dynamic_scope: bool,
    pub table_ordinals_in_scope: Vec<u32>,
    pub column_names_in_scope: Vec<String>,
    pub note: String,
}

/// Typed capability report for the stage-1 harness.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct SqlRuntimeCapabilityReport {
    pub surface_id: &'static str,
    pub claim_boundary: &'static str,
    /// Renderer and SQL literal admission semantics; distinct from layout IDs.
    pub sql_text_contract: &'static str,
    pub a5_encode_decode: bool,
    pub a5_incremental_tct_decode: bool,
    pub a1_text_structural_constraint: bool,
    pub a2_text_dynamic_scope_constraint: bool,
    /// Join-ON masking policy the runtime was compiled under (`off` |
    /// `structural`). Policy changes flip the elided stream contract.
    pub join_on_policy: &'static str,
    /// Column-reference addressing policy (`flat` | `factored`). `factored`
    /// spells a column as `(catalog table, column in that table)` digits;
    /// policy changes flip the elided stream contract.
    pub column_addressing: &'static str,
    /// Rank-1 visit-all remaining fields. When true, trailing optional
    /// skip-vs-visit (`rule._`) is mask-forced and elided.
    pub visit_all_fields: bool,
    pub text_constraint_blocker: Option<SqlTextConstraintBlocker>,
}

impl SqlRuntimeCapabilityReport {
    pub fn default_for_surface() -> Self {
        Self {
            surface_id: SQL_RUNTIME_SURFACE_ID,
            claim_boundary: SQL_RUNTIME_CLAIM_BOUNDARY,
            sql_text_contract: crate::sql_schema_linking::render::SQL_TEXT_CONTRACT,
            a5_encode_decode: true,
            a5_incremental_tct_decode: true,
            a1_text_structural_constraint: false,
            a2_text_dynamic_scope_constraint: false,
            join_on_policy: "off",
            column_addressing: "flat",
            visit_all_fields: false,
            text_constraint_blocker: Some(SqlTextConstraintBlocker::surface_order_divergence()),
        }
    }
}

/// Precise reusable blocker for A1/A2 text-prefix constraints.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct SqlTextConstraintBlocker {
    pub code: &'static str,
    pub summary: String,
    pub detail: String,
    /// When true, the blocker describes a durable design constraint rather
    /// than a transient local failure.
    pub reusable: bool,
}

impl SqlTextConstraintBlocker {
    pub fn surface_order_divergence() -> Self {
        Self {
            code: SQL_TEXT_CONSTRAINT_BLOCKER_CODE,
            summary: "SQL TCT stream is from-first tree order; surface SQL is select-list-first"
                .to_string(),
            detail: "\
The SQL schema-linking GrammarIR declares select_statement fields in from-first \
order (P6): from, where, group_by, having, order_by, limit, select_list. The \
canonical expanded TCT stream is a tree serialization, so column menu sites \
appear only after table sites have established scope. Surface SQL text, by \
contrast, places SELECT list identifiers before FROM. A1/A2 text-prefix \
constraints operate on conventional-tokenizer prefixes of surface SQL. Driving \
those prefixes from the raw syntax state machine would either (1) admit open-vocab \
identifiers before scope is established (unsound A2), or (2) require a second \
surface-order SQL constraint grammar/parser — a forbidden clone of the existing \
frontend. Until a dedicated surface-aligned constraint engine is designed (not in \
this tranche), begin_text_constraint fails closed for both dynamic_scope=false \
(A1 structural) and dynamic_scope=true (A2 live table/column scope). A5 encode, \
strict decode, and incremental TCT decode remain available and reuse the existing \
parser, model, menu binding, mask/apply, and renderer without a second grammar."
                .to_string(),
            reusable: true,
        }
    }
}

/// Phase-tagged runtime error (fail closed).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SqlRuntimeError {
    pub phase: &'static str,
    pub message: String,
    pub blocker_code: Option<&'static str>,
}

impl SqlRuntimeError {
    pub fn new(phase: &'static str, message: impl Into<String>) -> Self {
        Self {
            phase,
            message: message.into(),
            blocker_code: None,
        }
    }

    pub fn text_constraint_unavailable(mode: &str) -> Self {
        let blocker = SqlTextConstraintBlocker::surface_order_divergence();
        Self {
            phase: "text_constraint",
            message: format!(
                "{mode} text-prefix constraint unavailable: {} ({})",
                blocker.summary, blocker.code
            ),
            blocker_code: Some(SQL_TEXT_CONSTRAINT_BLOCKER_CODE),
        }
    }

    pub(super) fn from_catalog(err: SqlCatalogError) -> Self {
        Self::new("catalog", err.to_string())
    }

    pub(super) fn from_codec(err: SqlCodecError) -> Self {
        Self::new(err.phase, err.message)
    }

    pub(super) fn from_raw_sm(err: RawSyntaxError) -> Self {
        Self::new("raw_sm", format!("{err:?}"))
    }
}

impl std::fmt::Display for SqlRuntimeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "SQL runtime error ({}): {}", self.phase, self.message)?;
        if let Some(code) = self.blocker_code {
            write!(f, " [blocker={code}]")?;
        }
        Ok(())
    }
}

impl std::error::Error for SqlRuntimeError {}
