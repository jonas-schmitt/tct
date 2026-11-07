//! SQL schema-linking stage-1 runtime (Rust-first; Python via PyO3).
//!
//! Caches the compiled SQL parser/GrammarIR/StructuralModel/menu binding and
//! exposes A5 encode/decode plus incremental TCT decode. A1/A2 text-prefix
//! constraints are **typed fail-closed** in this tranche: the SQL GrammarIR
//! uses from-first tree order (**P6**), so the canonical TCT stream is not a
//! surface-text serialization and cannot safely drive conventional-tokenizer
//! prefixes without a second surface-order constraint engine (forbidden clone).
//!
//! Claim boundary: `compiler_sql_schema_linking_only`. Not a paper claim.

mod capability;

#[cfg(test)]
mod factored_tests;
#[cfg(test)]
mod menu_binding_tests;
#[cfg(test)]
mod tests;

pub use capability::{
    SQL_RUNTIME_CLAIM_BOUNDARY, SQL_RUNTIME_SURFACE_ID, SQL_TEXT_CONSTRAINT_BLOCKER_CODE,
    SqlRuntimeCapabilityReport, SqlRuntimeError, SqlTextConstraintBlocker, SqlTextConstraintState,
    SqlTextScopeMetadata, TextTokenPieceMap,
};

// Re-export the shared totality cap so SQL callers do not hard-code a second limit.
pub use crate::syntax_state_machine::ELIDED_FORCED_RUN_LIMIT;

use crate::provenance::sha256_hex;
use crate::sql_schema_linking::catalog::{SqlCatalog, load_fixture_catalog, normalize_sha256_hex};
use crate::sql_schema_linking::codec::SqlSchemaLinkingCodec;
use crate::sql_schema_linking::layout::{
    SQL_SHARED_PAYLOAD_END_EXCLUSIVE, SQL_SHARED_PAYLOAD_START, SQL_SYNTAX_LAYOUT_PAD_TOKEN,
    SQL_SYNTAX_LAYOUT_VOCAB_SIZE,
};
use crate::sql_schema_linking::menu_binding::{SqlColumnAddressing, SqlJoinOnPolicy};
use crate::syntax_state_machine::{
    FACTORED_COLUMN_MENU_DIGITS, LiveMenuCursor, MenuSiteKind, RawSyntaxAdmissibleTokens,
    RawSyntaxLiveState, RawSyntaxStatus, menu_digit_width,
};
use serde::Serialize;
use std::collections::BTreeSet;
use std::path::Path;
use std::sync::Arc;

/// Cached SQL schema-linking runtime bound to one catalog + compiled model.
#[derive(Clone)]
pub struct SqlSchemaLinkingRuntime {
    inner: Arc<SqlSchemaLinkingCodec>,
    join_on_policy: SqlJoinOnPolicy,
    column_addressing: SqlColumnAddressing,
    visit_all_fields: bool,
}

impl std::fmt::Debug for SqlSchemaLinkingRuntime {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SqlSchemaLinkingRuntime")
            .field("db_id", &self.inner.catalog().db_id)
            .field("catalog_sha256", &self.inner.catalog().tables_json_sha256)
            .field("vocab_size", &self.inner.model().token_roles().vocab_size)
            .field("join_on_policy", &self.join_on_policy.as_str())
            .field("column_addressing", &self.column_addressing.as_str())
            .field("visit_all_fields", &self.visit_all_fields)
            .finish()
    }
}

impl SqlSchemaLinkingRuntime {
    /// Construct from Spider-style `tables.json` bytes, content SHA-256, and `db_id`.
    ///
    /// Verifies the digest against the raw bytes (fail closed on mismatch),
    /// then compiles GrammarIR → StructuralModel → menu bindings once.
    pub fn from_catalog_json(
        catalog_json: &str,
        catalog_sha256: &str,
        db_id: &str,
    ) -> Result<Self, SqlRuntimeError> {
        Self::from_catalog_json_bytes(catalog_json.as_bytes(), catalog_sha256, db_id)
    }

    /// [`Self::from_catalog_json`] with an explicit join-ON masking policy.
    ///
    /// `Structural` changes admissibility (and therefore canonical elided
    /// streams) without touching the token layout; the runtime's
    /// [`Self::stream_contract`] is policy-tagged so downstream corpora and
    /// BPE artifacts cannot silently mix policies.
    pub fn from_catalog_json_with_join_policy(
        catalog_json: &str,
        catalog_sha256: &str,
        db_id: &str,
        join_on_policy: SqlJoinOnPolicy,
    ) -> Result<Self, SqlRuntimeError> {
        Self::from_catalog_json_bytes_with_join_policy(
            catalog_json.as_bytes(),
            catalog_sha256,
            db_id,
            join_on_policy,
        )
    }

    /// Byte-oriented constructor (same contract as [`Self::from_catalog_json`]).
    pub fn from_catalog_json_bytes(
        catalog_json: &[u8],
        catalog_sha256: &str,
        db_id: &str,
    ) -> Result<Self, SqlRuntimeError> {
        Self::from_catalog_json_bytes_with_join_policy(
            catalog_json,
            catalog_sha256,
            db_id,
            SqlJoinOnPolicy::Off,
        )
    }

    /// Byte-oriented constructor with an explicit join-ON masking policy.
    pub fn from_catalog_json_bytes_with_join_policy(
        catalog_json: &[u8],
        catalog_sha256: &str,
        db_id: &str,
        join_on_policy: SqlJoinOnPolicy,
    ) -> Result<Self, SqlRuntimeError> {
        Self::from_catalog_json_bytes_with_policies(
            catalog_json,
            catalog_sha256,
            db_id,
            join_on_policy,
            SqlColumnAddressing::Flat,
        )
    }

    /// [`Self::from_catalog_json`] with both SQL menu policies stated.
    pub fn from_catalog_json_with_policies(
        catalog_json: &str,
        catalog_sha256: &str,
        db_id: &str,
        join_on_policy: SqlJoinOnPolicy,
        column_addressing: SqlColumnAddressing,
    ) -> Result<Self, SqlRuntimeError> {
        Self::from_catalog_json_bytes_with_policies(
            catalog_json.as_bytes(),
            catalog_sha256,
            db_id,
            join_on_policy,
            column_addressing,
        )
    }

    /// Byte-oriented constructor with both SQL menu policies stated.
    ///
    /// The policies compose; [`Self::stream_contract`] tags the combination so
    /// corpora and BPE artifacts from different policy pairs cannot mix.
    pub fn from_catalog_json_bytes_with_policies(
        catalog_json: &[u8],
        catalog_sha256: &str,
        db_id: &str,
        join_on_policy: SqlJoinOnPolicy,
        column_addressing: SqlColumnAddressing,
    ) -> Result<Self, SqlRuntimeError> {
        Self::from_catalog_json_bytes_with_sql_policies(
            catalog_json,
            catalog_sha256,
            db_id,
            join_on_policy,
            column_addressing,
            false,
        )
    }

    /// Byte-oriented constructor with menu policies plus visit-all fields.
    pub fn from_catalog_json_bytes_with_sql_policies(
        catalog_json: &[u8],
        catalog_sha256: &str,
        db_id: &str,
        join_on_policy: SqlJoinOnPolicy,
        column_addressing: SqlColumnAddressing,
        visit_all_fields: bool,
    ) -> Result<Self, SqlRuntimeError> {
        if db_id.is_empty() {
            return Err(SqlRuntimeError::new("catalog", "db_id must be non-empty"));
        }
        let expected = normalize_sha256_hex(catalog_sha256, "catalog_sha256")
            .map_err(SqlRuntimeError::from_catalog)?;
        let actual = sha256_hex(catalog_json);
        if actual != expected {
            return Err(SqlRuntimeError::new(
                "catalog",
                format!("catalog sha256 mismatch: expected {expected}, got {actual}"),
            ));
        }
        let catalog = SqlCatalog::parse_tables_json_unpinned(catalog_json, db_id)
            .map_err(SqlRuntimeError::from_catalog)?;
        // Defensive re-check: parse path records its own sha of the same bytes.
        if catalog.tables_json_sha256 != actual {
            return Err(SqlRuntimeError::new(
                "catalog",
                format!(
                    "internal catalog sha mismatch: parse recorded {}, constructor saw {actual}",
                    catalog.tables_json_sha256
                ),
            ));
        }
        Self::from_catalog_with_sql_policies(
            catalog,
            join_on_policy,
            column_addressing,
            visit_all_fields,
        )
    }

    /// Compile from an already-validated [`SqlCatalog`].
    pub fn from_catalog(catalog: SqlCatalog) -> Result<Self, SqlRuntimeError> {
        Self::from_catalog_with_join_policy(catalog, SqlJoinOnPolicy::Off)
    }

    /// [`Self::from_catalog`] with an explicit join-ON masking policy.
    pub fn from_catalog_with_join_policy(
        catalog: SqlCatalog,
        join_on_policy: SqlJoinOnPolicy,
    ) -> Result<Self, SqlRuntimeError> {
        Self::from_catalog_with_policies(catalog, join_on_policy, SqlColumnAddressing::Flat)
    }

    /// [`Self::from_catalog`] with both SQL menu policies stated.
    pub fn from_catalog_with_policies(
        catalog: SqlCatalog,
        join_on_policy: SqlJoinOnPolicy,
        column_addressing: SqlColumnAddressing,
    ) -> Result<Self, SqlRuntimeError> {
        Self::from_catalog_with_sql_policies(catalog, join_on_policy, column_addressing, false)
    }

    /// [`Self::from_catalog`] with menu policies plus visit-all fields.
    ///
    /// `visit_all_fields` is the rank-1 compiler merge: trailing optional
    /// fields are always visited (`field_start` forced), so the historical
    /// skip-vs-visit free token (`rule._`) is elided. Alias / join-list
    /// alternatives stay representable. Stream-contract break; do not decode
    /// L6-v1 streams under this policy.
    pub fn from_catalog_with_sql_policies(
        catalog: SqlCatalog,
        join_on_policy: SqlJoinOnPolicy,
        column_addressing: SqlColumnAddressing,
        visit_all_fields: bool,
    ) -> Result<Self, SqlRuntimeError> {
        let codec = SqlSchemaLinkingCodec::for_catalog_with_sql_policies(
            catalog,
            join_on_policy,
            column_addressing,
            visit_all_fields,
        )
        .map_err(SqlRuntimeError::from_codec)?;
        Ok(Self {
            inner: Arc::new(codec),
            join_on_policy,
            column_addressing,
            visit_all_fields,
        })
    }

    /// Load the in-repo join fixture catalog and compile.
    pub fn fixture() -> Result<Self, SqlRuntimeError> {
        let catalog = load_fixture_catalog().map_err(SqlRuntimeError::from_catalog)?;
        Self::from_catalog(catalog)
    }

    /// Load catalog JSON from a path and verify against `catalog_sha256`.
    pub fn from_catalog_path(
        path: &Path,
        catalog_sha256: &str,
        db_id: &str,
    ) -> Result<Self, SqlRuntimeError> {
        let bytes = std::fs::read(path).map_err(|e| {
            SqlRuntimeError::new("catalog", format!("failed to read {}: {e}", path.display()))
        })?;
        Self::from_catalog_json_bytes(&bytes, catalog_sha256, db_id)
    }

    pub fn catalog(&self) -> &SqlCatalog {
        self.inner.catalog()
    }

    pub fn codec(&self) -> &SqlSchemaLinkingCodec {
        self.inner.as_ref()
    }

    pub fn vocab_size(&self) -> u32 {
        self.inner.model().token_roles().vocab_size
    }

    pub fn pad_token(&self) -> u32 {
        self.inner.model().token_roles().pad_token
    }

    pub fn catalog_sha256(&self) -> &str {
        &self.inner.catalog().tables_json_sha256
    }

    pub fn db_id(&self) -> &str {
        &self.inner.catalog().db_id
    }

    /// Join-ON masking policy this runtime was compiled under.
    pub fn join_on_policy(&self) -> SqlJoinOnPolicy {
        self.join_on_policy
    }

    /// Column-reference addressing policy this runtime was compiled under.
    pub fn column_addressing(&self) -> SqlColumnAddressing {
        self.column_addressing
    }

    /// Whether remaining fields must be visited before `node_end` (rank-1).
    pub fn visit_all_fields(&self) -> bool {
        self.visit_all_fields
    }

    /// Policy-tagged elided stream contract for a SQL policy triple.
    pub fn elided_stream_contract_for(
        join_on_policy: SqlJoinOnPolicy,
        column_addressing: SqlColumnAddressing,
        visit_all_fields: bool,
    ) -> &'static str {
        sql_elided_stream_contract(join_on_policy, column_addressing, visit_all_fields)
    }

    /// Canonical elided stream contract, tagged with the full policy set.
    ///
    /// The token layout is policy-independent, but the structural join-ON
    /// policy changes which tokens are mask-forced (and therefore elided),
    /// factored addressing changes both the values and the count of column
    /// tokens, and visit-all fields elides the skip-vs-visit free decision
    /// on trailing optionals. The policies compose, so the tag names the
    /// combination: streams from different policy triples must never share
    /// a contract string.
    pub fn stream_contract(&self) -> &'static str {
        sql_elided_stream_contract(
            self.join_on_policy,
            self.column_addressing,
            self.visit_all_fields,
        )
    }

    /// Capability report: A5 available; A1/A2 text constraints unavailable.
    pub fn capability_report(&self) -> SqlRuntimeCapabilityReport {
        let mut report = SqlRuntimeCapabilityReport::default_for_surface();
        report.join_on_policy = self.join_on_policy.as_str();
        report.column_addressing = self.column_addressing.as_str();
        report.visit_all_fields = self.visit_all_fields;
        report
    }

    /// Parse + encode SQL source to the expanded raw TCT stream (A5).
    pub fn encode(&self, sql: &str) -> Result<Vec<u32>, SqlRuntimeError> {
        Ok(self.encode_with_menu_trace(sql)?.0)
    }

    /// Parse + encode SQL to the **canonical elided** TCT stream (A5).
    ///
    /// Equivalent to [`Self::encode`] followed by [`Self::elide_forced_tokens`]:
    /// mask-forced tokens are dropped so the stream keeps only free-choice
    /// positions. Invert with [`Self::expand_elided_tokens`].
    pub fn encode_elided(&self, sql: &str) -> Result<Vec<u32>, SqlRuntimeError> {
        let expanded = self.encode(sql)?;
        self.elide_forced_tokens(&expanded)
    }

    /// Encode A5 tokens **and** the menu trace taken from the same parse.
    ///
    /// Menu ordinals are those already on the AST after
    /// [`SqlSchemaLinkingCodec::parse`] — the same resolution path A5 encode
    /// uses. Names are catalog lookups of those ordinals (not a second policy).
    pub fn encode_with_menu_trace(
        &self,
        sql: &str,
    ) -> Result<
        (
            Vec<u32>,
            Vec<crate::sql_schema_linking::menu_trace::MenuTraceSite>,
        ),
        SqlRuntimeError,
    > {
        use crate::sql_schema_linking::menu_trace::menu_trace_from_select;

        let stmt = self.inner.parse(sql).map_err(SqlRuntimeError::from_codec)?;
        let trace = menu_trace_from_select(&stmt, self.inner.catalog())
            .map_err(|e| SqlRuntimeError::new("menu_trace", e.to_string()))?;
        let tokens = self
            .inner
            .encode(&stmt)
            .map_err(SqlRuntimeError::from_codec)?;
        let pad = self.pad_token();
        if tokens.iter().any(|&t| t >= pad) {
            return Err(SqlRuntimeError::new(
                "encode",
                format!("encode produced token id >= PAD {pad}"),
            ));
        }
        Ok((tokens, trace))
    }

    /// Strict decode of a complete expanded stream → rendered SQL (A5).
    ///
    /// Fails closed on incomplete streams, tokens ≥ vocab/PAD, trailing
    /// garbage after a complete tree, and tree/render divergence.
    pub fn decode(&self, tokens: &[u32]) -> Result<String, SqlRuntimeError> {
        self.validate_token_ids(tokens)?;
        let stmt = self
            .inner
            .decode(tokens)
            .map_err(SqlRuntimeError::from_codec)?;
        let rendered = self
            .inner
            .render(&stmt)
            .map_err(SqlRuntimeError::from_codec)?;
        // Fixpoint: re-encode must match (strict whole-stream contract).
        let reencoded = self
            .inner
            .encode(&stmt)
            .map_err(SqlRuntimeError::from_codec)?;
        if reencoded != tokens {
            return Err(SqlRuntimeError::new(
                "decode",
                "decoded stream is not a canonical complete encoding (strict decode fixpoint)",
            ));
        }
        Ok(rendered)
    }

    /// Incremental A5 TCT decode state over the expanded raw stream.
    ///
    /// Owns a live [`RawSyntaxLiveState`] cursor that advances in O(one
    /// transition) against the immutable compiled model — no full-prefix
    /// replay on mask/apply.
    pub fn begin_tct_decode(&self) -> SqlTctDecodeState {
        SqlTctDecodeState {
            codec: Arc::clone(&self.inner),
            tokens: Vec::new(),
            live: self.inner.model().begin_live_state(),
        }
    }

    /// Incremental handle over the **canonical elided** stream contract.
    ///
    /// Construction eagerly drains the initial mask-forced run (root
    /// `node_start` is always forced), so the handle's observable position
    /// is a free choice from the first `apply`. Fails only when the drain
    /// hits [`ELIDED_FORCED_RUN_LIMIT`] or the engine-corruption
    /// `expansion_stuck` guard.
    pub fn begin_elided_tct_decode(&self) -> Result<SqlElidedTctDecodeState, SqlRuntimeError> {
        SqlElidedTctDecodeState::from_expanded(self.begin_tct_decode())
    }

    /// Elide every mask-forced token from a complete expanded raw stream.
    ///
    /// At each position the admissible mask is computed *before* consuming:
    /// a singleton mask means the token is forced and is **dropped** (it
    /// must equal the forced id, else `apply` rejects the stream); otherwise
    /// the token is kept. The walk must end EOF-valid. Invert exactly with
    /// [`Self::expand_elided_tokens`].
    pub fn elide_forced_tokens(&self, expanded: &[u32]) -> Result<Vec<u32>, SqlRuntimeError> {
        let mut walker = self.begin_tct_decode();
        let mut elided = Vec::with_capacity(expanded.len());
        for &token in expanded {
            let allowed = walker.allowed_token_ids()?;
            let forced = singleton_allowed_token_id(&allowed);
            // A singleton mask admits only the forced token, so `apply`
            // rejects any non-matching token with the engine's typed error.
            walker.apply(token)?;
            if forced != Some(token) {
                elided.push(token);
            }
        }
        if !walker.is_complete()? {
            return Err(SqlRuntimeError::new(
                "premature_eof",
                format!(
                    "elide_forced_tokens: expanded stream of {} tokens is not EOF-valid \
                     after full walk (consumed {})",
                    expanded.len(),
                    walker.consumed_tokens()
                ),
            ));
        }
        Ok(elided)
    }

    /// Expand a canonical elided stream back to the native expanded stream.
    ///
    /// Eagerly drains the initial forced run, then for each elided input
    /// token auto-applies forced tokens while the live mask is a singleton,
    /// consumes the input at the resulting free-choice position, and drains
    /// the trailing forced run. Every drain is bounded by
    /// [`ELIDED_FORCED_RUN_LIMIT`]. Accepts exactly the image of
    /// [`Self::elide_forced_tokens`]; spelling out a mask-forced token fails
    /// with typed `forced_token_in_elided_stream`.
    pub fn expand_elided_tokens(&self, elided: &[u32]) -> Result<Vec<u32>, SqlRuntimeError> {
        let mut walker = self.begin_elided_tct_decode()?;
        let mut expanded = Vec::with_capacity(elided.len().saturating_mul(2));
        expanded.extend_from_slice(walker.initial_forced_run());
        for &token in elided {
            let forced_after = walker.apply(token)?;
            expanded.push(token);
            expanded.extend_from_slice(&forced_after);
        }
        if !walker.is_complete()? {
            return Err(SqlRuntimeError::new(
                "premature_eof",
                format!(
                    "expand_elided_tokens: elided stream of {} tokens is not EOF-valid \
                     after full walk (expanded consumed {})",
                    elided.len(),
                    walker.expanded_tokens().len()
                ),
            ));
        }
        Ok(expanded)
    }

    /// Strict decode of a complete canonical elided stream → rendered SQL.
    ///
    /// Expands via [`Self::expand_elided_tokens`], then runs the expanded-path
    /// strict decoder so the replay engine stays the single source of truth.
    pub fn strict_decode_complete_elided(&self, elided: &[u32]) -> Result<String, SqlRuntimeError> {
        let expanded = self.expand_elided_tokens(elided)?;
        self.decode(&expanded)
    }

    /// A1/A2 text-prefix constraint entry point — **always fail-closed**.
    ///
    /// See [`SQL_TEXT_CONSTRAINT_BLOCKER_CODE`] and
    /// [`SqlRuntimeCapabilityReport`]. Token pieces and `dynamic_scope` are
    /// accepted only for signature stability; they are not interpreted.
    pub fn begin_text_constraint(
        &self,
        _token_pieces: &TextTokenPieceMap,
        dynamic_scope: bool,
    ) -> Result<SqlTextConstraintState, SqlRuntimeError> {
        let mode = if dynamic_scope { "A2" } else { "A1" };
        Err(SqlRuntimeError::text_constraint_unavailable(mode))
    }

    fn validate_token_ids(&self, tokens: &[u32]) -> Result<(), SqlRuntimeError> {
        let pad = self.pad_token();
        let vocab = self.vocab_size();
        for (i, &t) in tokens.iter().enumerate() {
            if t >= vocab {
                return Err(SqlRuntimeError::new(
                    "tokens",
                    format!("token[{i}]={t} is out of range for VOCAB_SIZE {vocab}"),
                ));
            }
            if t >= pad {
                return Err(SqlRuntimeError::new(
                    "tokens",
                    format!("token[{i}]={t} is PAD or above (PAD={pad}); not a payload token"),
                ));
            }
        }
        Ok(())
    }
}

fn sql_elided_stream_contract(
    join_on_policy: SqlJoinOnPolicy,
    column_addressing: SqlColumnAddressing,
    visit_all_fields: bool,
) -> &'static str {
    use crate::syntax_token_layout_diff as contracts;
    match (join_on_policy, column_addressing, visit_all_fields) {
        (SqlJoinOnPolicy::Off, SqlColumnAddressing::Flat, false) => {
            contracts::RAW_SYNTAX_STREAM_CONTRACT_ELIDED_V1
        }
        (SqlJoinOnPolicy::Structural, SqlColumnAddressing::Flat, false) => {
            contracts::RAW_SYNTAX_STREAM_CONTRACT_ELIDED_V1_JOINON_STRUCTURAL
        }
        (SqlJoinOnPolicy::Off, SqlColumnAddressing::Factored, false) => {
            contracts::RAW_SYNTAX_STREAM_CONTRACT_ELIDED_V1_FACTORED_COLUMNS
        }
        (SqlJoinOnPolicy::Structural, SqlColumnAddressing::Factored, false) => {
            contracts::RAW_SYNTAX_STREAM_CONTRACT_ELIDED_V1_JOINON_STRUCTURAL_FACTORED_COLUMNS
        }
        (SqlJoinOnPolicy::FkDeclared, SqlColumnAddressing::Flat, false) => {
            contracts::RAW_SYNTAX_STREAM_CONTRACT_ELIDED_V1_JOINON_FK_DECLARED
        }
        (SqlJoinOnPolicy::FkDeclared, SqlColumnAddressing::Factored, false) => {
            contracts::RAW_SYNTAX_STREAM_CONTRACT_ELIDED_V1_JOINON_FK_DECLARED_FACTORED_COLUMNS
        }
        (SqlJoinOnPolicy::Off, SqlColumnAddressing::Flat, true) => {
            contracts::RAW_SYNTAX_STREAM_CONTRACT_ELIDED_V1_VISIT_ALL_FIELDS
        }
        (SqlJoinOnPolicy::Structural, SqlColumnAddressing::Flat, true) => {
            contracts::RAW_SYNTAX_STREAM_CONTRACT_ELIDED_V1_JOINON_STRUCTURAL_VISIT_ALL_FIELDS
        }
        (SqlJoinOnPolicy::Off, SqlColumnAddressing::Factored, true) => {
            contracts::RAW_SYNTAX_STREAM_CONTRACT_ELIDED_V1_FACTORED_COLUMNS_VISIT_ALL_FIELDS
        }
        (SqlJoinOnPolicy::Structural, SqlColumnAddressing::Factored, true) => {
            contracts::RAW_SYNTAX_STREAM_CONTRACT_ELIDED_V1_JOINON_STRUCTURAL_FACTORED_COLUMNS_VISIT_ALL_FIELDS
        }
        (SqlJoinOnPolicy::FkDeclared, SqlColumnAddressing::Flat, true) => {
            contracts::RAW_SYNTAX_STREAM_CONTRACT_ELIDED_V1_JOINON_FK_DECLARED_VISIT_ALL_FIELDS
        }
        (SqlJoinOnPolicy::FkDeclared, SqlColumnAddressing::Factored, true) => {
            contracts::RAW_SYNTAX_STREAM_CONTRACT_ELIDED_V1_JOINON_FK_DECLARED_FACTORED_COLUMNS_VISIT_ALL_FIELDS
        }
    }
}

/// Incremental TCT decode over the expanded raw SQL stream (A5).
///
/// Holds an owned live raw-syntax cursor (`RawSyntaxLiveState`) advanced
/// against the codec's immutable compiled model — one transition per
/// `apply`, not a full-prefix replay.
#[derive(Clone)]
pub struct SqlTctDecodeState {
    codec: Arc<SqlSchemaLinkingCodec>,
    tokens: Vec<u32>,
    live: RawSyntaxLiveState,
}

impl std::fmt::Debug for SqlTctDecodeState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SqlTctDecodeState")
            .field("consumed_tokens", &self.tokens.len())
            .field("applied_transitions", &self.live.applied_transitions())
            .field("tokens", &self.tokens)
            .finish()
    }
}

impl SqlTctDecodeState {
    pub fn consumed_tokens(&self) -> usize {
        self.tokens.len()
    }

    pub fn tokens(&self) -> &[u32] {
        &self.tokens
    }

    /// Engine-level successful transitions applied on this cursor.
    ///
    /// Live incremental advance increments this by exactly one per successful
    /// `apply`. Full-prefix replay would accumulate a triangular sum.
    pub fn applied_transitions(&self) -> u64 {
        self.live.applied_transitions()
    }

    /// Finite admissible raw token ids at the current prefix (mask).
    ///
    /// Expands ranged payload bands and includes `payload_end` when admissible.
    /// Never includes PAD. Empty after completion.
    pub fn allowed_token_ids(&self) -> Result<Vec<u32>, SqlRuntimeError> {
        if self.is_complete()? {
            return Ok(Vec::new());
        }
        let admissible = self.admissible();
        let mut allowed = expand_admissible_token_ids(
            &admissible,
            self.codec.model().token_roles().payload_end,
            self.codec.model().token_roles().pad_token,
            self.codec.model().token_roles().vocab_size,
        );
        let nul_token = self.codec.model().token_roles().utf8_range.start;
        if allowed.contains(&nul_token) && self.at_string_literal_value() {
            allowed.retain(|&token| token != nul_token);
        }
        Ok(allowed)
    }

    fn at_string_literal_value(&self) -> bool {
        let snap = self.codec.model().live_state_snapshot(&self.live);
        snap.active_rule.as_deref() == Some("string_literal")
            && snap.active_field.as_deref() == Some("value")
    }

    /// Apply one expanded raw token. Fail closed on PAD/out-of-vocab, mask
    /// rejection, and extra tokens after completion.
    ///
    /// Advances the owned live cursor in O(one transition).
    pub fn apply(&mut self, token: u32) -> Result<(), SqlRuntimeError> {
        let pad = self.codec.model().token_roles().pad_token;
        let vocab = self.codec.model().token_roles().vocab_size;
        if token >= vocab {
            return Err(SqlRuntimeError::new(
                "apply",
                format!("token {token} is out of range for VOCAB_SIZE {vocab}"),
            ));
        }
        if token >= pad {
            return Err(SqlRuntimeError::new(
                "apply",
                format!("token {token} is PAD or above (PAD={pad})"),
            ));
        }
        if self.is_complete()? {
            return Err(SqlRuntimeError::new(
                "apply",
                "stream is already complete; extra tokens are rejected",
            ));
        }
        if token == self.codec.model().token_roles().utf8_range.start
            && self.at_string_literal_value()
        {
            return Err(SqlRuntimeError::new(
                "literal",
                "string_literal.value rejects NUL, which is not representable in SQLite SQL text",
            ));
        }
        self.codec
            .model()
            .advance_live_state(&mut self.live, token)
            .map_err(SqlRuntimeError::from_raw_sm)?;
        self.tokens.push(token);
        Ok(())
    }

    /// True when the consumed prefix is a complete, EOF-valid expanded stream.
    pub fn is_complete(&self) -> Result<bool, SqlRuntimeError> {
        if self.tokens.is_empty() {
            return Ok(false);
        }
        Ok(self.codec.model().live_state_eof_valid(&self.live))
    }

    /// Render canonical SQL for a complete stream only.
    ///
    /// Uses the same strict complete-decode + encode-fixpoint contract as
    /// batch [`SqlSchemaLinkingRuntime::decode`].
    pub fn render_sql(&self) -> Result<String, SqlRuntimeError> {
        if !self.is_complete()? {
            return Err(SqlRuntimeError::new(
                "render",
                "cannot render_sql on incomplete TCT decode state",
            ));
        }
        // Strict whole-stream contract (match batch decode).
        let pad = self.codec.model().token_roles().pad_token;
        let vocab = self.codec.model().token_roles().vocab_size;
        for (i, &t) in self.tokens.iter().enumerate() {
            if t >= vocab {
                return Err(SqlRuntimeError::new(
                    "tokens",
                    format!("token[{i}]={t} is out of range for VOCAB_SIZE {vocab}"),
                ));
            }
            if t >= pad {
                return Err(SqlRuntimeError::new(
                    "tokens",
                    format!("token[{i}]={t} is PAD or above (PAD={pad}); not a payload token"),
                ));
            }
        }
        let stmt = self
            .codec
            .decode(&self.tokens)
            .map_err(SqlRuntimeError::from_codec)?;
        let rendered = self
            .codec
            .render(&stmt)
            .map_err(SqlRuntimeError::from_codec)?;
        let reencoded = self
            .codec
            .encode(&stmt)
            .map_err(SqlRuntimeError::from_codec)?;
        if reencoded != self.tokens {
            return Err(SqlRuntimeError::new(
                "decode",
                "decoded stream is not a canonical complete encoding (strict decode fixpoint)",
            ));
        }
        Ok(rendered)
    }

    /// Active rule/field snapshot for debugging (empty strings when none).
    pub fn active_site(&self) -> Result<SqlTctActiveSite, SqlRuntimeError> {
        let snap = self.codec.model().live_state_snapshot(&self.live);
        Ok(SqlTctActiveSite {
            active_rule: snap.active_rule,
            active_field: snap.active_field,
            status: match snap.status {
                RawSyntaxStatus::Partial => "partial",
                RawSyntaxStatus::Complete => "complete",
            },
            stack_depth: snap.stack_depth,
            eof_valid: snap.eof_valid,
        })
    }

    /// Named menu at the current free-choice position (empty when not a menu).
    pub fn active_menu(&self) -> Result<SqlActiveMenu, SqlRuntimeError> {
        let cursor = self.codec.model().live_menu_cursor(&self.live);
        let allowed = self.allowed_token_ids()?;
        Ok(resolve_active_menu(
            self.codec.catalog(),
            cursor.as_ref(),
            &allowed,
        ))
    }

    /// Opt-in qualified catalog identities for the current menu's candidates.
    ///
    /// This is a read-only snapshot before the next choice. Match a selected
    /// base token to `bindings.token_id`; use that selected identity only for
    /// subsequent positions, never as input to predicting its own token.
    pub fn active_menu_bindings(&self) -> Result<SqlActiveMenuBindings, SqlRuntimeError> {
        let cursor = self.codec.model().live_menu_cursor(&self.live);
        let allowed = self.allowed_token_ids()?;
        Ok(resolve_active_menu_bindings(
            self.codec.catalog(),
            cursor.as_ref(),
            &allowed,
        ))
    }

    fn admissible(&self) -> RawSyntaxAdmissibleTokens {
        self.codec.model().admissible_live_state(&self.live)
    }
}

/// Incremental TCT decode over the **canonical elided** SQL stream (A5).
///
/// Wraps an expanded-layer [`SqlTctDecodeState`] cursor. Construction and
/// every successful [`Self::apply`] eagerly drain mask-forced runs so the
/// observable position is always a free choice (or EOF-complete). Python
/// and training decoders should use this handle so forced steps do not
/// consume a model forward pass.
#[derive(Clone)]
pub struct SqlElidedTctDecodeState {
    state: SqlTctDecodeState,
    consumed_elided_tokens: usize,
    initial_forced_run: Vec<u32>,
    last_forced_run: Vec<u32>,
}

impl std::fmt::Debug for SqlElidedTctDecodeState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SqlElidedTctDecodeState")
            .field("consumed_elided_tokens", &self.consumed_elided_tokens)
            .field("expanded_tokens", &self.state.tokens.len())
            .field("initial_forced_run_len", &self.initial_forced_run.len())
            .finish()
    }
}

impl SqlElidedTctDecodeState {
    fn from_expanded(state: SqlTctDecodeState) -> Result<Self, SqlRuntimeError> {
        let mut elided = Self {
            state,
            consumed_elided_tokens: 0,
            initial_forced_run: Vec::new(),
            last_forced_run: Vec::new(),
        };
        let initial_forced_run = elided.drain_forced_run()?;
        elided.initial_forced_run = initial_forced_run.clone();
        elided.last_forced_run = initial_forced_run;
        Ok(elided)
    }

    /// Mask-forced run auto-applied at construction (before the first elided input).
    pub fn initial_forced_run(&self) -> &[u32] {
        &self.initial_forced_run
    }

    /// Mask-forced run auto-applied by the most recent drain.
    pub fn last_forced_run(&self) -> &[u32] {
        &self.last_forced_run
    }

    pub fn consumed_elided_tokens(&self) -> usize {
        self.consumed_elided_tokens
    }

    /// Expanded-layer tokens consumed so far (elided inputs + auto-applied forced).
    pub fn expanded_tokens(&self) -> &[u32] {
        self.state.tokens()
    }

    /// Active rule/field snapshot at the current free-choice position.
    ///
    /// Consumers classifying a decision's *role* (table menu, column menu,
    /// literal, operator) must use this rather than inferring role from a
    /// token id's band: under the collapsed shared-payload-pool layout
    /// `utf8_byte`, `u8_integer` and `menu_index` occupy coincident ranges, so
    /// the band carries no role information by construction.
    pub fn active_site(&self) -> Result<SqlTctActiveSite, SqlRuntimeError> {
        self.state.active_site()
    }

    /// Named menu at the current free-choice (elided) position.
    pub fn active_menu(&self) -> Result<SqlActiveMenu, SqlRuntimeError> {
        self.state.active_menu()
    }

    /// Qualified candidates after all implicit mask-forced steps have drained.
    pub fn active_menu_bindings(&self) -> Result<SqlActiveMenuBindings, SqlRuntimeError> {
        self.state.active_menu_bindings()
    }

    /// Finite admissible raw token ids at the current free-choice position.
    ///
    /// After construction and every successful `apply`, the mask is never a
    /// singleton on a usable handle (empty only after completion).
    pub fn allowed_token_ids(&self) -> Result<Vec<u32>, SqlRuntimeError> {
        self.state.allowed_token_ids()
    }

    /// Apply one elided raw token, then drain the following forced run.
    ///
    /// Returns the auto-applied forced tokens after the input. Rejects an
    /// input that sits on a mask-forced position with typed
    /// `forced_token_in_elided_stream` (even when it equals the forced id).
    pub fn apply(&mut self, token: u32) -> Result<Vec<u32>, SqlRuntimeError> {
        // Defensive restatement: after construction and every successful
        // advance the eager drain guarantees a non-forced position.
        if let Some(forced_token) = singleton_allowed_token_id(&self.state.allowed_token_ids()?) {
            return Err(self.forced_token_error(token, forced_token, self.state.consumed_tokens()));
        }
        match self.state.apply(token) {
            Ok(()) => {}
            Err(error) => {
                // Reclassify spelled-out forced tokens from the preceding drain.
                if !self.state.is_complete().unwrap_or(false)
                    && self.last_forced_run.contains(&token)
                {
                    let expanded_offset = self
                        .state
                        .tokens()
                        .iter()
                        .rposition(|&t| t == token)
                        .unwrap_or(self.state.consumed_tokens());
                    return Err(self.forced_token_error(token, token, expanded_offset));
                }
                return Err(error);
            }
        }
        self.consumed_elided_tokens += 1;
        let forced_tokens_applied = self.drain_forced_run()?;
        self.last_forced_run = forced_tokens_applied.clone();
        Ok(forced_tokens_applied)
    }

    /// True when the expanded-layer cursor is a complete EOF-valid stream.
    pub fn is_complete(&self) -> Result<bool, SqlRuntimeError> {
        self.state.is_complete()
    }

    /// Render canonical SQL for a complete stream only (strict fixpoint).
    pub fn render_sql(&self) -> Result<String, SqlRuntimeError> {
        self.state.render_sql()
    }

    /// Eagerly apply mask-forced tokens while the live mask is a singleton.
    fn drain_forced_run(&mut self) -> Result<Vec<u32>, SqlRuntimeError> {
        let mut run = Vec::new();
        loop {
            let allowed = self.state.allowed_token_ids()?;
            let Some(forced_token) = singleton_allowed_token_id(&allowed) else {
                if allowed.is_empty() && !self.state.is_complete().unwrap_or(false) {
                    return Err(SqlRuntimeError::new(
                        "expansion_stuck",
                        format!(
                            "elided-stream expansion reached an empty admissible mask before \
                             EOF validity at expanded offset {} (after {} consumed elided \
                             tokens); the engine empties the mask only after the root closes",
                            self.state.consumed_tokens(),
                            self.consumed_elided_tokens
                        ),
                    ));
                }
                break;
            };
            if run.len() >= ELIDED_FORCED_RUN_LIMIT {
                return Err(SqlRuntimeError::new(
                    "forced_run_limit_exceeded",
                    format!(
                        "elided-stream expansion auto-applied {ELIDED_FORCED_RUN_LIMIT} \
                         consecutive mask-forced tokens at expanded offset {} (after {} \
                         consumed elided tokens) without reaching a free choice; the run cap \
                         is a totality guard against unbounded forced recursion",
                        self.state.consumed_tokens(),
                        self.consumed_elided_tokens
                    ),
                ));
            }
            self.state.apply(forced_token)?;
            run.push(forced_token);
        }
        Ok(run)
    }

    fn forced_token_error(
        &self,
        offending_token: u32,
        forced_token: u32,
        expanded_raw_token_offset: usize,
    ) -> SqlRuntimeError {
        SqlRuntimeError::new(
            "forced_token_in_elided_stream",
            format!(
                "elided input token {offending_token} at elided offset {} spells out the \
                 mask-forced token {forced_token} that expansion auto-applies at expanded \
                 offset {expanded_raw_token_offset}; canonical elided streams omit every \
                 mask-forced token, even when the spelled-out token matches the forced value",
                self.consumed_elided_tokens
            ),
        )
    }
}

/// Lightweight active-site metadata for incremental TCT decode.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct SqlTctActiveSite {
    pub active_rule: Option<String>,
    pub active_field: Option<String>,
    pub status: &'static str,
    pub stack_depth: usize,
    pub eof_valid: bool,
}

/// One admissible menu ordinal and the catalog name it denotes.
///
/// `name` is Spider `*_original` so it matches the schema prompt
/// (`table #i Singer(#0 Singer_ID, …)`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct SqlActiveMenuReferent {
    pub ordinal: u32,
    pub name: String,
}

/// Named menu at the current free-choice position.
///
/// `kind`:
/// - `none` — not a menu site
/// - `table` — catalog table menu (`table_ref`)
/// - `column_table` — factored column digit 0 (catalog table, scope-masked)
/// - `column` — factored column digit 1 (columns of the chosen table)
/// - `column_flat` — historical flat scope-relative column menu
///
/// Radix sites (`table`, `column_flat`) spell one ordinal as `width`
/// big-endian digits over the payload band; factored column sites spell the
/// `(table, column)` pair as `width == 2` positional digits. `referents` is
/// populated only when `resolved` is true: at a digit whose admissible values
/// each complete one ordinal (the final radix digit, or either factored
/// digit). At an earlier radix digit every admissible value still spans a
/// *range* of ordinals, and once `digits_seen == width` the ordinal is
/// already chosen (only `payload_end` can follow); both states report
/// `resolved == false` with no referents rather than a wrong name.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct SqlActiveMenu {
    pub kind: &'static str,
    pub referents: Vec<SqlActiveMenuReferent>,
    pub digits_seen: u32,
    /// Total digits this site's ordinal (or factored pair) spells.
    pub width: u32,
    /// True iff `referents` names the admissible completions of the current digit.
    pub resolved: bool,
    pub addressing: Option<&'static str>,
}

/// One admissible base token and the catalog identity that selecting it resolves.
///
/// `ordinal` retains [`SqlActiveMenuReferent::ordinal`] semantics. At a final
/// radix digit it is the full ordinal, not the digit in `token_id`.
/// `table_ordinal` indexes the runtime's pinned catalog; `column_ordinal`
/// indexes that table's real columns (excluding Spider's `*` pseudo-column),
/// not the global Spider column array. Names are the catalog `*_original`
/// spellings. Table and factored-parent choices have no column identity.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct SqlActiveMenuBinding {
    pub token_id: u32,
    pub ordinal: u32,
    pub table_ordinal: u32,
    pub table_name: String,
    pub column_ordinal: Option<u32>,
    pub column_name: Option<String>,
}

/// Additive menu snapshot with qualified catalog identities.
///
/// `menu` is identical to [`SqlTctDecodeState::active_menu`]. Bindings follow
/// its referent order and are empty at unresolved radix prefixes, completed
/// menu sites, and non-menu positions. This does not identify already chosen
/// referents after `apply`, or introduce token IDs, masks, or stream semantics.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct SqlActiveMenuBindings {
    #[serde(flatten)]
    pub menu: SqlActiveMenu,
    pub bindings: Vec<SqlActiveMenuBinding>,
}

impl SqlActiveMenu {
    fn none() -> Self {
        Self {
            kind: "none",
            referents: Vec::new(),
            digits_seen: 0,
            width: 0,
            resolved: false,
            addressing: None,
        }
    }
}

fn payload_digit(token: u32) -> Option<u32> {
    if (SQL_SHARED_PAYLOAD_START..SQL_SHARED_PAYLOAD_END_EXCLUSIVE).contains(&token) {
        Some(token - SQL_SHARED_PAYLOAD_START)
    } else {
        None
    }
}

fn referent(ordinal: u32, name: impl Into<String>) -> SqlActiveMenuReferent {
    SqlActiveMenuReferent {
        ordinal,
        name: name.into(),
    }
}

fn admissible_digits(allowed: &[u32]) -> BTreeSet<u32> {
    allowed.iter().copied().filter_map(payload_digit).collect()
}

/// Referents for the final digit of a radix menu code.
///
/// With `digits_seen == width - 1` every admissible digit `d` completes the
/// ordinal `partial * base + d`. An earlier digit's admissible values each
/// span a `base^k`-ordinal range, and an exhausted width means the ordinal is
/// already chosen; both yield `(empty, false)` so a consumer never receives a
/// digit value dressed up as a full ordinal's name.
fn radix_referents(
    allowed_digits: &BTreeSet<u32>,
    base: u32,
    width: u32,
    digits_seen: u32,
    partial: u32,
    name_of_ordinal: impl Fn(u32) -> Option<String>,
) -> (Vec<SqlActiveMenuReferent>, bool) {
    if digits_seen.saturating_add(1) != width {
        return (Vec::new(), false);
    }
    let prefix = (partial as u64) * (base as u64);
    let referents = allowed_digits
        .iter()
        .filter_map(|&digit| {
            let ordinal = u32::try_from(prefix + digit as u64).ok()?;
            let name = name_of_ordinal(ordinal)?;
            Some(referent(ordinal, name))
        })
        .collect();
    (referents, true)
}

/// Catalog indices for a flat scope-relative ordinal (tables in FROM/JOIN order,
/// each table's columns in catalog order).
fn flat_column_address(
    catalog: &SqlCatalog,
    scope_tables: &[u32],
    columns_per_table: &[u32],
    flat: u32,
) -> Option<(u32, u32)> {
    let mut offset = 0u32;
    for &table_idx in scope_tables {
        let table = catalog.tables.get(table_idx as usize);
        let count = match table {
            Some(table) => table.columns.len() as u32,
            None => columns_per_table
                .get(table_idx as usize)
                .copied()
                .unwrap_or(0),
        };
        if flat < offset.saturating_add(count) {
            let column = flat - offset;
            return table
                .and_then(|table| table.columns.get(column as usize))
                .map(|_| (table_idx, column));
        }
        offset = offset.saturating_add(count);
    }
    None
}

fn flat_column_name(
    catalog: &SqlCatalog,
    scope_tables: &[u32],
    columns_per_table: &[u32],
    flat: u32,
) -> Option<String> {
    let (table, column) = flat_column_address(catalog, scope_tables, columns_per_table, flat)?;
    catalog
        .tables
        .get(table as usize)?
        .columns
        .get(column as usize)
        .map(|column| column.name_original.clone())
}

fn resolve_active_menu(
    catalog: &SqlCatalog,
    cursor: Option<&LiveMenuCursor>,
    allowed: &[u32],
) -> SqlActiveMenu {
    let Some(cursor) = cursor else {
        return SqlActiveMenu::none();
    };
    let allowed_digits = admissible_digits(allowed);
    let base = SQL_SHARED_PAYLOAD_END_EXCLUSIVE - SQL_SHARED_PAYLOAD_START;
    match &cursor.site_kind {
        MenuSiteKind::Fixed { cardinality, .. } => {
            let live_cardinality = cursor.menu_cardinality.unwrap_or(*cardinality);
            let width = cursor
                .menu_width
                .unwrap_or_else(|| menu_digit_width(live_cardinality, base));
            let (referents, resolved) = radix_referents(
                &allowed_digits,
                base,
                width,
                cursor.digits_seen,
                cursor.menu_partial,
                |ordinal| {
                    catalog
                        .tables
                        .get(ordinal as usize)
                        .map(|table| table.name_original.clone())
                },
            );
            SqlActiveMenu {
                kind: "table",
                referents,
                digits_seen: cursor.digits_seen,
                width,
                resolved,
                addressing: None,
            }
        }
        MenuSiteKind::FactoredScopeColumns { .. } if cursor.digits_seen == 0 => {
            let mut referents = Vec::new();
            for (ordinal, table) in catalog.tables.iter().enumerate() {
                let ordinal = ordinal as u32;
                if !allowed_digits.contains(&ordinal) {
                    continue;
                }
                referents.push(referent(ordinal, table.name_original.clone()));
            }
            SqlActiveMenu {
                kind: "column_table",
                referents,
                digits_seen: cursor.digits_seen,
                width: FACTORED_COLUMN_MENU_DIGITS,
                resolved: true,
                addressing: Some("factored"),
            }
        }
        MenuSiteKind::FactoredScopeColumns { .. } if cursor.digits_seen == 1 => {
            // `menu_partial` holds the digit-0 catalog table index until the
            // pair completes (the engine then rewrites it to the flat ordinal).
            let table_idx = cursor.menu_partial as usize;
            let mut referents = Vec::new();
            if let Some(table) = catalog.tables.get(table_idx) {
                for (ordinal, column) in table.columns.iter().enumerate() {
                    let ordinal = ordinal as u32;
                    if !allowed_digits.contains(&ordinal) {
                        continue;
                    }
                    referents.push(referent(ordinal, column.name_original.clone()));
                }
            }
            SqlActiveMenu {
                kind: "column",
                referents,
                digits_seen: cursor.digits_seen,
                width: FACTORED_COLUMN_MENU_DIGITS,
                resolved: true,
                addressing: Some("factored"),
            }
        }
        MenuSiteKind::FactoredScopeColumns { .. } => {
            // Both digits consumed: `menu_partial` is now the completed flat
            // scope-relative ordinal, not a table index, and only
            // `payload_end` can follow — there is no current-digit menu.
            SqlActiveMenu {
                kind: "column",
                referents: Vec::new(),
                digits_seen: cursor.digits_seen,
                width: FACTORED_COLUMN_MENU_DIGITS,
                resolved: false,
                addressing: Some("factored"),
            }
        }
        MenuSiteKind::ScopeRelativeColumns { columns_per_table } => {
            let live_cardinality = cursor.menu_cardinality.unwrap_or_else(|| {
                cursor
                    .scope_tables
                    .iter()
                    .map(|&table_idx| {
                        columns_per_table
                            .get(table_idx as usize)
                            .copied()
                            .unwrap_or(0)
                    })
                    .fold(0u32, u32::saturating_add)
            });
            let width = cursor
                .menu_width
                .unwrap_or_else(|| menu_digit_width(live_cardinality, base));
            let (referents, resolved) = radix_referents(
                &allowed_digits,
                base,
                width,
                cursor.digits_seen,
                cursor.menu_partial,
                |ordinal| {
                    flat_column_name(catalog, &cursor.scope_tables, columns_per_table, ordinal)
                },
            );
            SqlActiveMenu {
                kind: "column_flat",
                referents,
                digits_seen: cursor.digits_seen,
                width,
                resolved,
                addressing: Some("flat"),
            }
        }
    }
}

fn resolve_active_menu_bindings(
    catalog: &SqlCatalog,
    cursor: Option<&LiveMenuCursor>,
    allowed: &[u32],
) -> SqlActiveMenuBindings {
    // Preserve the historical naming, radix-prefix, and completion semantics.
    // All scope information comes from this same compiler-owned cursor.
    let menu = resolve_active_menu(catalog, cursor, allowed);
    let mut bindings = Vec::new();
    if menu.resolved
        && let Some(cursor) = cursor
    {
        let base = SQL_SHARED_PAYLOAD_END_EXCLUSIVE - SQL_SHARED_PAYLOAD_START;
        for referent in &menu.referents {
            let address = match &cursor.site_kind {
                MenuSiteKind::Fixed { .. } => Some((referent.ordinal, None)),
                MenuSiteKind::FactoredScopeColumns { .. } if cursor.digits_seen == 0 => {
                    Some((referent.ordinal, None))
                }
                MenuSiteKind::FactoredScopeColumns { .. } if cursor.digits_seen == 1 => {
                    Some((cursor.menu_partial, Some(referent.ordinal)))
                }
                MenuSiteKind::FactoredScopeColumns { .. } => None,
                MenuSiteKind::ScopeRelativeColumns { columns_per_table } => flat_column_address(
                    catalog,
                    &cursor.scope_tables,
                    columns_per_table,
                    referent.ordinal,
                )
                .map(|(table, column)| (table, Some(column))),
            };
            let Some((table_ordinal, column_ordinal)) = address else {
                continue;
            };
            let Some(table) = catalog.tables.get(table_ordinal as usize) else {
                continue;
            };
            let column_name = match column_ordinal {
                Some(column) => {
                    let Some(column) = table.columns.get(column as usize) else {
                        continue;
                    };
                    Some(column.name_original.clone())
                }
                None => None,
            };
            bindings.push(SqlActiveMenuBinding {
                token_id: SQL_SHARED_PAYLOAD_START + referent.ordinal % base,
                ordinal: referent.ordinal,
                table_ordinal,
                table_name: table.name_original.clone(),
                column_ordinal,
                column_name,
            });
        }
    }
    SqlActiveMenuBindings { menu, bindings }
}

/// Total singleton extractor over an expanded admissible id list.
///
/// Returns `Some(id)` iff `allowed` has cardinality exactly one. Defined
/// over the same list [`SqlTctDecodeState::allowed_token_ids`] produces
/// (finite ids + expanded ranges + payload_end), so a width-1 menu site or
/// a lone structural role is a singleton exactly like Go's
/// `singleton_admissible_raw_token`.
pub fn singleton_allowed_token_id(allowed: &[u32]) -> Option<u32> {
    match *allowed {
        [id] => Some(id),
        _ => None,
    }
}

/// Expand an admissible mask into a sorted unique list of raw token ids.
fn expand_admissible_token_ids(
    admissible: &RawSyntaxAdmissibleTokens,
    payload_end: u32,
    pad_token: u32,
    vocab_size: u32,
) -> Vec<u32> {
    let mut ids: Vec<u32> = Vec::new();
    for &id in &admissible.finite_raw_token_ids {
        if id < pad_token && id < vocab_size {
            ids.push(id);
        }
    }
    for range in &admissible.raw_token_ranges {
        let start = range.start;
        let end = range.end_exclusive.min(pad_token).min(vocab_size);
        for id in start..end {
            ids.push(id);
        }
    }
    if admissible.payload_end_admissible && payload_end < pad_token && payload_end < vocab_size {
        ids.push(payload_end);
    }
    ids.sort_unstable();
    ids.dedup();
    ids
}

// Layout constants re-exported for tests that assert PAD/VOCAB stability.
#[allow(dead_code)]
const _: () = {
    let _ = SQL_SYNTAX_LAYOUT_PAD_TOKEN;
    let _ = SQL_SYNTAX_LAYOUT_VOCAB_SIZE;
};
