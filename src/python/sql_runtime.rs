//! Experimental Python bindings for the SQL schema-linking runtime.
//!
//! Compiled only with the `experimental` Cargo feature and registered into the
//! `tct` Python module by `register`. Stage-1 runtime surface: A5
//! encode/decode, incremental TCT decode over the expanded, elided, and
//! elided-BPE streams; A1/A2 text constraints fail closed.

use pyo3::exceptions::{PyIOError, PyRuntimeError, PyValueError};
use pyo3::prelude::*;
use pyo3::types::{PyBytes, PyDict, PyList};
use std::sync::Arc;

use crate::bpe::{BpeMergeTable, encoder::BpeDecoder};
use crate::sql_schema_linking::elided_bpe_mask;
use crate::sql_schema_linking::layout::SQL_SYNTAX_LAYOUT_PAD_TOKEN;
use crate::sql_schema_linking::menu_binding::{SqlColumnAddressing, SqlJoinOnPolicy};
use crate::sql_schema_linking::runtime::{
    self, SQL_RUNTIME_CLAIM_BOUNDARY, SQL_RUNTIME_SURFACE_ID, SQL_TEXT_CONSTRAINT_BLOCKER_CODE,
    SqlRuntimeError, TextTokenPieceMap,
};

fn sql_runtime_error(err: SqlRuntimeError) -> PyErr {
    let msg = err.to_string();
    if err.phase == "text_constraint" || err.blocker_code.is_some() {
        PyRuntimeError::new_err(msg)
    } else if err.phase == "catalog" || err.phase == "token_pieces" || err.phase == "tokens" {
        PyValueError::new_err(msg)
    } else {
        PyRuntimeError::new_err(msg)
    }
}

/// Stage-1 SQL schema-linking runtime (Rust-first; A5 + fail-closed A1/A2).
///
/// Constructed from Spider-style catalog JSON, its SHA-256, and `db_id`.
/// Caches the compiled parser/GrammarIR/StructuralModel/menu binding.
#[pyclass(name = "SqlSchemaLinkingRuntime")]
pub struct PySqlSchemaLinkingRuntime {
    inner: runtime::SqlSchemaLinkingRuntime,
}

#[pymethods]
impl PySqlSchemaLinkingRuntime {
    /// Create a runtime from catalog JSON text, content SHA-256, and db_id.
    ///
    /// `join_on_policy` selects join-key masking: `"off"` (default, historical
    /// streams), `"structural"` (equality join keys must pair the just-joined
    /// table with an earlier in-scope table), or `"fk_declared"` (structural
    /// plus unique declared-FK shrink). Canonical elided streams change; see
    /// `stream_contract`.
    ///
    /// `column_addressing` selects how a column reference is spelled:
    /// `"flat"` (default, one scope-relative ordinal) or `"factored"` (two
    /// digits — catalog table index, then column index inside that table —
    /// matching the schema prompt's own notation). The policies compose and
    /// both are reflected in `stream_contract`.
    ///
    /// `visit_all_fields` (default false) is the rank-1 compiler merge: trailing
    /// optional fields are always visited, so skip-vs-visit (`rule._`) is
    /// elided. Stream-contract break versus historical SQL cells.
    #[new]
    #[pyo3(signature = (catalog_json, catalog_sha256, db_id, join_on_policy=None, column_addressing=None, visit_all_fields=None))]
    fn new(
        catalog_json: &str,
        catalog_sha256: &str,
        db_id: &str,
        join_on_policy: Option<&str>,
        column_addressing: Option<&str>,
        visit_all_fields: Option<bool>,
    ) -> PyResult<Self> {
        let policy = match join_on_policy {
            None => SqlJoinOnPolicy::Off,
            Some(value) => SqlJoinOnPolicy::parse(value).ok_or_else(|| {
                pyo3::exceptions::PyValueError::new_err(format!(
                    "unknown join_on_policy {value:?} (expected \"off\", \"structural\", or \"fk_declared\")"
                ))
            })?,
        };
        let addressing = match column_addressing {
            None => SqlColumnAddressing::Flat,
            Some(value) => SqlColumnAddressing::parse(value).ok_or_else(|| {
                pyo3::exceptions::PyValueError::new_err(format!(
                    "unknown column_addressing {value:?} (expected \"flat\" or \"factored\")"
                ))
            })?,
        };
        let visit_all = visit_all_fields.unwrap_or(false);
        let inner = runtime::SqlSchemaLinkingRuntime::from_catalog_json_bytes_with_sql_policies(
            catalog_json.as_bytes(),
            catalog_sha256,
            db_id,
            policy,
            addressing,
            visit_all,
        )
        .map_err(sql_runtime_error)?;
        Ok(Self { inner })
    }

    /// Join-ON masking policy this runtime was compiled under.
    #[getter]
    fn join_on_policy(&self) -> &'static str {
        self.inner.join_on_policy().as_str()
    }

    /// Column-reference addressing policy this runtime was compiled under.
    #[getter]
    fn column_addressing(&self) -> &'static str {
        self.inner.column_addressing().as_str()
    }

    /// Rank-1 visit-all remaining fields policy this runtime was compiled under.
    #[getter]
    fn visit_all_fields(&self) -> bool {
        self.inner.visit_all_fields()
    }

    /// Policy-tagged canonical elided stream contract identifier.
    #[getter]
    fn stream_contract(&self) -> &'static str {
        self.inner.stream_contract()
    }

    /// Load the in-repo join fixture catalog (tests / local smoke only).
    #[staticmethod]
    fn fixture() -> PyResult<Self> {
        let inner = runtime::SqlSchemaLinkingRuntime::fixture().map_err(sql_runtime_error)?;
        Ok(Self { inner })
    }

    #[getter]
    fn db_id(&self) -> &str {
        self.inner.db_id()
    }

    #[getter]
    fn catalog_sha256(&self) -> &str {
        self.inner.catalog_sha256()
    }

    #[getter]
    fn vocab_size(&self) -> u32 {
        self.inner.vocab_size()
    }

    #[getter]
    fn pad_token(&self) -> u32 {
        self.inner.pad_token()
    }

    /// Capability report as a plain dict (A5 true; A1/A2 false + blocker).
    fn capability_report<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyDict>> {
        let report = self.inner.capability_report();
        let dict = PyDict::new(py);
        dict.set_item("surface_id", report.surface_id)?;
        dict.set_item("claim_boundary", report.claim_boundary)?;
        dict.set_item("sql_text_contract", report.sql_text_contract)?;
        dict.set_item("a5_encode_decode", report.a5_encode_decode)?;
        dict.set_item(
            "a5_incremental_tct_decode",
            report.a5_incremental_tct_decode,
        )?;
        dict.set_item(
            "a1_text_structural_constraint",
            report.a1_text_structural_constraint,
        )?;
        dict.set_item(
            "a2_text_dynamic_scope_constraint",
            report.a2_text_dynamic_scope_constraint,
        )?;
        dict.set_item("join_on_policy", report.join_on_policy)?;
        dict.set_item("column_addressing", report.column_addressing)?;
        if let Some(blocker) = report.text_constraint_blocker {
            let b = PyDict::new(py);
            b.set_item("code", blocker.code)?;
            b.set_item("summary", blocker.summary)?;
            b.set_item("detail", blocker.detail)?;
            b.set_item("reusable", blocker.reusable)?;
            dict.set_item("text_constraint_blocker", b)?;
        } else {
            dict.set_item("text_constraint_blocker", py.None())?;
        }
        Ok(dict)
    }

    /// Parse + encode SQL to the expanded raw TCT stream (A5).
    fn encode(&self, py: Python<'_>, sql: &str) -> PyResult<Vec<u32>> {
        let inner = self.inner.clone();
        let sql = sql.to_string();
        py.detach(move || inner.encode(&sql))
            .map_err(sql_runtime_error)
    }

    /// Parse + encode SQL to the canonical **elided** TCT stream (forced tokens dropped).
    fn encode_elided(&self, py: Python<'_>, sql: &str) -> PyResult<Vec<u32>> {
        let inner = self.inner.clone();
        let sql = sql.to_string();
        py.detach(move || inner.encode_elided(&sql))
            .map_err(sql_runtime_error)
    }

    /// Drop mask-forced tokens from a complete expanded stream (pure transform).
    fn elide_forced_tokens(&self, py: Python<'_>, tokens: Vec<u32>) -> PyResult<Vec<u32>> {
        let inner = self.inner.clone();
        py.detach(move || inner.elide_forced_tokens(&tokens))
            .map_err(sql_runtime_error)
    }

    /// Expand a canonical elided stream back to the native expanded stream.
    fn expand_elided_tokens(&self, py: Python<'_>, tokens: Vec<u32>) -> PyResult<Vec<u32>> {
        let inner = self.inner.clone();
        py.detach(move || inner.expand_elided_tokens(&tokens))
            .map_err(sql_runtime_error)
    }

    /// Strict decode of a complete elided stream → rendered SQL.
    fn strict_decode_complete_elided(&self, py: Python<'_>, tokens: Vec<u32>) -> PyResult<String> {
        let inner = self.inner.clone();
        py.detach(move || inner.strict_decode_complete_elided(&tokens))
            .map_err(sql_runtime_error)
    }

    /// Encode A5 tokens and the menu trace from the **same** parse path.
    ///
    /// Returns ``{"tokens": [...], "menu_trace": [...], "provenance":
    /// "tct_encode_with_menu_trace"}``. Ordinals are AST menu indices resolved
    /// by the TCT parser (same as A5 encode) — not a models-local SQL mirror.
    fn encode_with_menu_trace<'py>(
        &self,
        py: Python<'py>,
        sql: &str,
    ) -> PyResult<Bound<'py, PyDict>> {
        let inner = self.inner.clone();
        let sql = sql.to_string();
        let (tokens, trace) = py
            .detach(move || inner.encode_with_menu_trace(&sql))
            .map_err(sql_runtime_error)?;
        let dict = PyDict::new(py);
        dict.set_item("tokens", tokens)?;
        let sites = PyList::empty(py);
        for site in trace {
            let s = PyDict::new(py);
            s.set_item("clause", site.clause)?;
            s.set_item("kind", site.kind)?;
            s.set_item("ordinal", site.ordinal)?;
            s.set_item("name", site.name)?;
            if let Some(ref qn) = site.qualifier_name {
                s.set_item("qualifier_name", qn)?;
            }
            if let Some(qo) = site.qualifier_ordinal {
                s.set_item("qualifier_ordinal", qo)?;
            }
            sites.append(s)?;
        }
        dict.set_item("menu_trace", sites)?;
        dict.set_item("provenance", "tct_encode_with_menu_trace")?;
        dict.set_item("a3_menu_provenance", "tct_encode_with_menu_trace")?;
        Ok(dict)
    }

    /// Strict decode of a complete expanded stream → rendered SQL (A5).
    fn decode(&self, py: Python<'_>, tokens: Vec<u32>) -> PyResult<String> {
        let inner = self.inner.clone();
        py.detach(move || inner.decode(&tokens))
            .map_err(sql_runtime_error)
    }

    /// Begin incremental A5 TCT decode over the expanded stream.
    fn begin_tct_decode(&self) -> PySqlTctDecodeState {
        PySqlTctDecodeState {
            inner: self.inner.begin_tct_decode(),
        }
    }

    /// Begin incremental A5 TCT decode over the **elided** stream.
    ///
    /// The handle auto-applies mask-forced tokens, so the model only sees
    /// free-choice positions (no forward pass on forced steps).
    fn begin_elided_tct_decode(&self) -> PyResult<PySqlElidedTctDecodeState> {
        let inner = self
            .inner
            .begin_elided_tct_decode()
            .map_err(sql_runtime_error)?;
        Ok(PySqlElidedTctDecodeState { inner })
    }

    /// Begin incremental elided-stream decode in **BPE token space**.
    ///
    /// Loads a BPE merge table from a **filesystem path** (same loader as
    /// `tct-sql-elided-bpe-validate --bpe-merges` / `BpeMergeTable::from_file`)
    /// and truncates to `merge_count` merges via
    /// [`elided_bpe_mask::bpe_codec_for_merges`]. The returned handle exposes
    /// the exact corpus-validated admissible BPE mask
    /// ([`elided_bpe_mask::bpe_token_is_admissible`] /
    /// [`elided_bpe_mask::try_apply_bpe_token`]) — no second mask
    /// implementation.
    ///
    /// Layout constants on the handle (`base_vocab_size`, `bpe_start`,
    /// `total_vocab_size`, `pad_token`) are for consumer pin validation only;
    /// this constructor does **not** change the SQL token layout.
    #[pyo3(signature = (merge_table_path, merge_count))]
    fn begin_elided_bpe_tct_decode(
        &self,
        merge_table_path: &str,
        merge_count: usize,
    ) -> PyResult<PySqlElidedBpeTctDecodeState> {
        let table = BpeMergeTable::from_file(merge_table_path).map_err(|e| {
            PyIOError::new_err(format!(
                "Failed to load SQL elided BPE merge table from '{merge_table_path}': {e}"
            ))
        })?;
        let (_encoder, decoder, bpe_vocab_size) =
            elided_bpe_mask::bpe_codec_for_merges(&table, merge_count)
                .map_err(sql_runtime_error)?;
        let state = self
            .inner
            .begin_elided_tct_decode()
            .map_err(sql_runtime_error)?;
        let base_vocab_size = table.base_vocab_size;
        let bpe_start = table.bpe_start();
        // PAD is a base-layer reserved id (layout pin); never a free-choice
        // decision and never BPE-admissible.
        let pad_token = SQL_SYNTAX_LAYOUT_PAD_TOKEN;
        Ok(PySqlElidedBpeTctDecodeState {
            state,
            // Shared across beam clones: decoder is immutable for the life of
            // the decode, so clone() is a refcount bump rather than deep-copying
            // both merge hash maps per beam per step.
            decoder: Arc::new(decoder),
            bpe_vocab_size,
            base_vocab_size,
            bpe_start,
            total_vocab_size: bpe_vocab_size,
            pad_token,
            consumed_bpe_tokens: 0,
        })
    }

    /// A1/A2 text-prefix constraints — always fail closed (see capability_report).
    ///
    /// `token_pieces` is a dict mapping base token id → UTF-8 string (or bytes).
    /// `tokenizer_hash` must be an exact 64-hex SHA-256 digest of the bound
    /// conventional tokenizer. `dynamic_scope=False` is A1; `True` is A2. Both
    /// raise with the reusable surface-order blocker code after piece-map
    /// validation.
    #[pyo3(signature = (token_pieces, tokenizer_hash, dynamic_scope=false))]
    fn begin_text_constraint(
        &self,
        token_pieces: Bound<'_, PyDict>,
        tokenizer_hash: &str,
        dynamic_scope: bool,
    ) -> PyResult<()> {
        let mut pairs: Vec<(u32, Vec<u8>)> = Vec::new();
        for (key, value) in token_pieces.iter() {
            let id: u32 = key.extract().map_err(|_| {
                PyValueError::new_err("token_pieces keys must be unsigned base token ids")
            })?;
            let bytes: Vec<u8> = if let Ok(s) = value.extract::<String>() {
                s.into_bytes()
            } else if let Ok(b) = value.extract::<Vec<u8>>() {
                b
            } else if let Ok(b) = value.extract::<Bound<'_, PyBytes>>() {
                b.as_bytes().to_vec()
            } else {
                return Err(PyValueError::new_err(
                    "token_pieces values must be str or bytes",
                ));
            };
            pairs.push((id, bytes));
        }
        let pieces = TextTokenPieceMap::new(tokenizer_hash, pairs).map_err(sql_runtime_error)?;
        match self.inner.begin_text_constraint(&pieces, dynamic_scope) {
            Ok(_) => Err(PyRuntimeError::new_err(
                "begin_text_constraint returned Ok unexpectedly; A1/A2 remain unavailable",
            )),
            Err(err) => Err(sql_runtime_error(err)),
        }
    }
}

/// Incremental A5 TCT decode state (expanded stream).
/// Shared shape for the `active_site()` snapshot exposed on every decode state.
///
/// Role classification must read `active_rule` / `active_field` rather than
/// infer a role from the token id's band: under the collapsed
/// shared-payload-pool layout `utf8_byte`, `u8_integer` and `menu_index`
/// occupy coincident ranges, so band carries no role information.
fn active_site_dict(
    py: Python<'_>,
    site: runtime::SqlTctActiveSite,
) -> PyResult<Bound<'_, PyDict>> {
    let dict = PyDict::new(py);
    dict.set_item("active_rule", site.active_rule)?;
    dict.set_item("active_field", site.active_field)?;
    dict.set_item("status", site.status)?;
    dict.set_item("stack_depth", site.stack_depth)?;
    dict.set_item("eof_valid", site.eof_valid)?;
    Ok(dict)
}

fn active_menu_dict(py: Python<'_>, menu: runtime::SqlActiveMenu) -> PyResult<Bound<'_, PyDict>> {
    let dict = PyDict::new(py);
    dict.set_item("kind", menu.kind)?;
    dict.set_item("digits_seen", menu.digits_seen)?;
    dict.set_item("width", menu.width)?;
    dict.set_item("resolved", menu.resolved)?;
    dict.set_item("addressing", menu.addressing)?;
    let referents = PyList::empty(py);
    for referent in menu.referents {
        let row = PyDict::new(py);
        row.set_item("ordinal", referent.ordinal)?;
        row.set_item("name", referent.name)?;
        referents.append(row)?;
    }
    dict.set_item("referents", referents)?;
    Ok(dict)
}

fn active_menu_bindings_dict(
    py: Python<'_>,
    menu: runtime::SqlActiveMenuBindings,
) -> PyResult<Bound<'_, PyDict>> {
    let dict = active_menu_dict(py, menu.menu)?;
    let bindings = PyList::empty(py);
    for binding in menu.bindings {
        let row = PyDict::new(py);
        row.set_item("token_id", binding.token_id)?;
        row.set_item("ordinal", binding.ordinal)?;
        row.set_item("table_ordinal", binding.table_ordinal)?;
        row.set_item("table_name", binding.table_name)?;
        row.set_item("column_ordinal", binding.column_ordinal)?;
        row.set_item("column_name", binding.column_name)?;
        bindings.append(row)?;
    }
    dict.set_item("bindings", bindings)?;
    Ok(dict)
}

#[pyclass(name = "SqlTctDecodeState")]
pub struct PySqlTctDecodeState {
    inner: runtime::SqlTctDecodeState,
}

#[pymethods]
impl PySqlTctDecodeState {
    fn allowed_token_ids(&self) -> PyResult<Vec<u32>> {
        self.inner.allowed_token_ids().map_err(sql_runtime_error)
    }

    fn apply(&mut self, token: u32) -> PyResult<()> {
        self.inner.apply(token).map_err(sql_runtime_error)
    }

    fn is_complete(&self) -> PyResult<bool> {
        self.inner.is_complete().map_err(sql_runtime_error)
    }

    fn consumed_tokens(&self) -> usize {
        self.inner.consumed_tokens()
    }

    fn render_sql(&self) -> PyResult<String> {
        self.inner.render_sql().map_err(sql_runtime_error)
    }

    fn tokens(&self) -> Vec<u32> {
        self.inner.tokens().to_vec()
    }

    /// Active rule/field at the current position, for role classification.
    fn active_site<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyDict>> {
        active_site_dict(py, self.inner.active_site().map_err(sql_runtime_error)?)
    }

    /// Named menu at the current free-choice position (`kind` + `referents`).
    fn active_menu<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyDict>> {
        active_menu_dict(py, self.inner.active_menu().map_err(sql_runtime_error)?)
    }

    /// Current candidates with catalog table/column identity and base token ID.
    fn active_menu_bindings<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyDict>> {
        active_menu_bindings_dict(
            py,
            self.inner
                .active_menu_bindings()
                .map_err(sql_runtime_error)?,
        )
    }
}

/// Incremental A5 TCT decode state over the **elided** stream.
///
/// Auto-advances mask-forced tokens so the model only samples free choices.
#[pyclass(name = "SqlElidedTctDecodeState")]
pub struct PySqlElidedTctDecodeState {
    inner: runtime::SqlElidedTctDecodeState,
}

#[pymethods]
impl PySqlElidedTctDecodeState {
    /// Independent copy of this decode state (for speculative base-space probes).
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
        }
    }

    fn allowed_token_ids(&self) -> PyResult<Vec<u32>> {
        self.inner.allowed_token_ids().map_err(sql_runtime_error)
    }

    /// Apply one elided token. Returns the auto-applied forced tokens after it.
    fn apply(&mut self, token: u32) -> PyResult<Vec<u32>> {
        self.inner.apply(token).map_err(sql_runtime_error)
    }

    fn is_complete(&self) -> PyResult<bool> {
        self.inner.is_complete().map_err(sql_runtime_error)
    }

    fn consumed_elided_tokens(&self) -> usize {
        self.inner.consumed_elided_tokens()
    }

    fn initial_forced_run(&self) -> Vec<u32> {
        self.inner.initial_forced_run().to_vec()
    }

    fn last_forced_run(&self) -> Vec<u32> {
        self.inner.last_forced_run().to_vec()
    }

    fn expanded_tokens(&self) -> Vec<u32> {
        self.inner.expanded_tokens().to_vec()
    }

    fn render_sql(&self) -> PyResult<String> {
        self.inner.render_sql().map_err(sql_runtime_error)
    }

    /// Active rule/field at the current free-choice position.
    fn active_site<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyDict>> {
        active_site_dict(py, self.inner.active_site().map_err(sql_runtime_error)?)
    }

    /// Named menu at the current free-choice position (`kind` + `referents`).
    fn active_menu<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyDict>> {
        active_menu_dict(py, self.inner.active_menu().map_err(sql_runtime_error)?)
    }

    /// Qualified candidates, including parents already consumed by forced elision.
    fn active_menu_bindings<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyDict>> {
        active_menu_bindings_dict(
            py,
            self.inner
                .active_menu_bindings()
                .map_err(sql_runtime_error)?,
        )
    }
}

/// Incremental A5 TCT decode in **BPE space** over the elided SQL stream.
///
/// Delegates admissibility and advance to
/// [`elided_bpe_mask::bpe_token_is_admissible`] /
/// [`elided_bpe_mask::try_apply_bpe_token`]. PAD is never admissible. This is a
/// binding only: no new mask semantics and no SQL token-layout change.
#[pyclass(name = "SqlElidedBpeTctDecodeState")]
pub struct PySqlElidedBpeTctDecodeState {
    state: runtime::SqlElidedTctDecodeState,
    /// Shared merge table: immutable for the life of a decode. Beam clones
    /// share one `Arc` so `clone()` is a refcount bump, not a deep hash-map copy.
    decoder: Arc<BpeDecoder>,
    bpe_vocab_size: u32,
    base_vocab_size: u32,
    bpe_start: u32,
    total_vocab_size: u32,
    pad_token: u32,
    consumed_bpe_tokens: usize,
}

#[pymethods]
impl PySqlElidedBpeTctDecodeState {
    /// Independent copy of this decode state (for mask-constrained beam search).
    ///
    /// Clones the mutable walker and progress counters; the BPE decoder is
    /// shared via `Arc` (immutable for the life of the decode).
    fn clone(&self) -> Self {
        Self {
            state: self.state.clone(),
            decoder: Arc::clone(&self.decoder),
            bpe_vocab_size: self.bpe_vocab_size,
            base_vocab_size: self.base_vocab_size,
            bpe_start: self.bpe_start,
            total_vocab_size: self.total_vocab_size,
            pad_token: self.pad_token,
            consumed_bpe_tokens: self.consumed_bpe_tokens,
        }
    }

    /// BPE-local ids admissible from the current elided free-choice state.
    ///
    /// Scans `0..bpe_vocab_size` via
    /// [`elided_bpe_mask::bpe_token_is_admissible`]. PAD is included in the
    /// scan range but is never returned (empty expansion / illegal probe).
    fn allowed_token_ids(&self) -> PyResult<Vec<u32>> {
        let mut allowed = Vec::new();
        for bpe_id in 0..self.bpe_vocab_size {
            let ok = elided_bpe_mask::bpe_token_is_admissible(&self.state, bpe_id, &self.decoder)
                .map_err(sql_runtime_error)?;
            if ok {
                allowed.push(bpe_id);
            }
        }
        Ok(allowed)
    }

    /// Apply one BPE token. Returns the base elided expansion that was applied.
    ///
    /// On rejection raises a typed error naming the rejected BPE id
    /// (`Ok(None)` from [`elided_bpe_mask::try_apply_bpe_token`]).
    fn apply(&mut self, bpe_id: u32) -> PyResult<Vec<u32>> {
        match elided_bpe_mask::try_apply_bpe_token(&self.state, bpe_id, &self.decoder) {
            Ok(Some(new_state)) => {
                let expansion =
                    elided_bpe_mask::expand_bpe_token_to_elided_base(&self.decoder, bpe_id)
                        .map_err(sql_runtime_error)?;
                self.state = new_state;
                self.consumed_bpe_tokens += 1;
                Ok(expansion)
            }
            Ok(None) => Err(PyValueError::new_err(format!(
                "BPE token {bpe_id} is not admissible at the current elided free-choice state \
                 (rejected_bpe_id={bpe_id})"
            ))),
            Err(err) => Err(sql_runtime_error(err)),
        }
    }

    fn is_complete(&self) -> PyResult<bool> {
        self.state.is_complete().map_err(sql_runtime_error)
    }

    fn consumed_bpe_tokens(&self) -> usize {
        self.consumed_bpe_tokens
    }

    /// Expanded-layer tokens consumed so far (elided inputs + auto-applied forced).
    fn expanded_tokens(&self) -> Vec<u32> {
        self.state.expanded_tokens().to_vec()
    }

    fn render_sql(&self) -> PyResult<String> {
        self.state.render_sql().map_err(sql_runtime_error)
    }

    /// Active rule/field at the current free-choice position.
    ///
    /// A BPE token spans several base decisions, so this reports the site the
    /// *next* base decision sits at; attribute a merge across the sites it
    /// spans rather than to this one alone.
    fn active_site<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyDict>> {
        active_site_dict(py, self.state.active_site().map_err(sql_runtime_error)?)
    }

    /// Named menu at the current free-choice position (`kind` + `referents`).
    ///
    /// A BPE token spans several base decisions, so this names the menu the
    /// *next* base decision sits at (meaningful between BPE tokens only);
    /// `referents` is populated only when `resolved` is true.
    fn active_menu<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyDict>> {
        active_menu_dict(py, self.state.active_menu().map_err(sql_runtime_error)?)
    }

    /// Qualified candidates for the next base decision, between BPE tokens.
    ///
    /// `token_id` is a base token, not a BPE ID. A merge can cross multiple
    /// sites; this snapshot cannot label the whole merge or its later choices.
    fn active_menu_bindings<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyDict>> {
        active_menu_bindings_dict(
            py,
            self.state
                .active_menu_bindings()
                .map_err(sql_runtime_error)?,
        )
    }

    #[getter]
    fn base_vocab_size(&self) -> u32 {
        self.base_vocab_size
    }

    #[getter]
    fn bpe_start(&self) -> u32 {
        self.bpe_start
    }

    #[getter]
    fn total_vocab_size(&self) -> u32 {
        self.total_vocab_size
    }

    #[getter]
    fn pad_token(&self) -> u32 {
        self.pad_token
    }

    #[getter]
    fn bpe_vocab_size(&self) -> u32 {
        self.bpe_vocab_size
    }
}

/// Register the SQL schema-linking classes and constants on the `tct` module.
pub(super) fn register(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<PySqlSchemaLinkingRuntime>()?;
    m.add_class::<PySqlTctDecodeState>()?;
    m.add_class::<PySqlElidedTctDecodeState>()?;
    m.add_class::<PySqlElidedBpeTctDecodeState>()?;
    m.add("SQL_RUNTIME_SURFACE_ID", SQL_RUNTIME_SURFACE_ID)?;
    m.add("SQL_RUNTIME_CLAIM_BOUNDARY", SQL_RUNTIME_CLAIM_BOUNDARY)?;
    m.add(
        "SQL_TEXT_CONSTRAINT_BLOCKER_CODE",
        SQL_TEXT_CONSTRAINT_BLOCKER_CODE,
    )?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bpe::{BpeMerge, BpeMetadata, encoder::SequentialBpeEncoder};
    use crate::python::tests::with_python;

    #[test]
    fn sql_schema_linking_runtime_encode_decode_and_tct_state() {
        let rt = PySqlSchemaLinkingRuntime::fixture().expect("fixture runtime");
        with_python(|py| {
            let tokens = rt
                .encode(py, "SELECT name, city FROM customers")
                .expect("encode");
            assert!(!tokens.is_empty());
            let rendered = rt.decode(py, tokens.clone()).expect("decode");
            assert_eq!(rendered, r#"SELECT "name", "city" FROM "customers""#);

            let elided = rt
                .encode_elided(py, "SELECT name, city FROM customers")
                .expect("encode_elided");
            assert!(elided.len() < tokens.len(), "elided shorter than expanded");
            let reexpanded = rt.expand_elided_tokens(py, elided.clone()).expect("expand");
            assert_eq!(reexpanded, tokens);
            let via_elide = rt.elide_forced_tokens(py, tokens.clone()).expect("elide");
            assert_eq!(via_elide, elided);
            assert_eq!(
                rt.strict_decode_complete_elided(py, elided.clone())
                    .expect("strict elided"),
                rendered
            );
            let mut elided_state = rt.begin_elided_tct_decode().expect("begin elided");
            for &token in &elided {
                elided_state.apply(token).expect("apply elided");
            }
            assert!(elided_state.is_complete().expect("complete elided"));
            assert_eq!(elided_state.render_sql().expect("render elided"), rendered);

            let mut state = rt.begin_tct_decode();
            for &token in &tokens {
                let allowed = state.allowed_token_ids().expect("mask");
                assert!(allowed.contains(&token));
                state.apply(token).expect("apply");
            }
            assert!(state.is_complete().expect("complete"));
            assert_eq!(state.render_sql().expect("render"), rendered);

            let caps = rt.capability_report(py).expect("caps");
            assert!(
                caps.get_item("a5_encode_decode")
                    .unwrap()
                    .unwrap()
                    .extract::<bool>()
                    .unwrap()
            );
            assert!(
                !caps
                    .get_item("a1_text_structural_constraint")
                    .unwrap()
                    .unwrap()
                    .extract::<bool>()
                    .unwrap()
            );

            let packed = rt
                .encode_with_menu_trace(py, "SELECT name FROM customers")
                .expect("encode_with_menu_trace");
            let prov: String = packed
                .get_item("provenance")
                .unwrap()
                .unwrap()
                .extract()
                .unwrap();
            assert_eq!(prov, "tct_encode_with_menu_trace");
            let tr_tokens: Vec<u32> = packed
                .get_item("tokens")
                .unwrap()
                .unwrap()
                .extract()
                .unwrap();
            assert!(!tr_tokens.is_empty());
            let trace = packed.get_item("menu_trace").unwrap().unwrap();
            let trace_list = trace.cast::<PyList>().unwrap();
            assert!(trace_list.len() >= 2);
            let first = trace_list.get_item(0).unwrap();
            let first_d = first.cast::<PyDict>().unwrap();
            let kind: String = first_d
                .get_item("kind")
                .unwrap()
                .unwrap()
                .extract()
                .unwrap();
            assert_eq!(kind, "table");

            let pieces = pyo3::types::PyDict::new(py);
            pieces.set_item(1u32, "SELECT").unwrap();
            let err = rt
                .begin_text_constraint(pieces, &"a".repeat(64), false)
                .expect_err("A1 blocked");
            let msg = err.to_string();
            assert!(
                msg.contains(SQL_TEXT_CONSTRAINT_BLOCKER_CODE) || msg.contains("text-prefix"),
                "{msg}"
            );
        });
    }

    #[test]
    fn sql_elided_tct_decode_state_clone_is_independent() {
        let rt = PySqlSchemaLinkingRuntime::fixture().expect("fixture runtime");
        with_python(|py| {
            let elided = rt
                .encode_elided(py, "SELECT name FROM customers")
                .expect("encode_elided");
            assert!(!elided.is_empty());
            let mut base = rt.begin_elided_tct_decode().expect("begin");
            base.apply(elided[0]).expect("apply first");
            let mut probe = base.clone();
            // Advance only the clone; base must stay put.
            if elided.len() > 1 {
                probe.apply(elided[1]).expect("apply on clone");
            }
            assert_eq!(base.consumed_elided_tokens(), 1);
            assert!(probe.consumed_elided_tokens() >= 1);
            if elided.len() > 1 {
                assert_eq!(probe.consumed_elided_tokens(), 2);
                assert_ne!(
                    base.expanded_tokens().len(),
                    probe.expanded_tokens().len(),
                    "clone must be an independent walker"
                );
            }
        });
    }

    #[test]
    fn active_site_reports_role_independent_of_token_band() {
        // Under the collapsed layout utf8_byte / u8_integer / menu_index share
        // one payload pool, so a decision's role cannot be read off its token
        // id. active_site() is the supported way to recover it.
        use crate::sql_schema_linking::catalog::load_fixture_bundle;
        use crate::sql_schema_linking::layout::{
            SQL_SHARED_PAYLOAD_POOL_WIDTH, SQL_SHARED_PAYLOAD_START,
        };
        use crate::sql_schema_linking::runtime::SqlSchemaLinkingRuntime;
        use serde::Deserialize;

        #[derive(Debug, Deserialize)]
        struct GoldFile {
            queries: Vec<GoldQuery>,
        }
        #[derive(Debug, Deserialize)]
        struct GoldQuery {
            sql: String,
        }

        let bundle = load_fixture_bundle().expect("bundle");
        let file: GoldFile =
            serde_json::from_slice(&bundle.gold_queries_bytes).expect("parse gold");
        let rt = SqlSchemaLinkingRuntime::fixture().expect("fixture rt");

        let mut saw_rule = false;
        let mut saw_payload_band_decision = false;
        for q in file.queries.iter().take(8) {
            let gold = rt.encode_elided(&q.sql).expect("elided");
            let mut st = rt.begin_elided_tct_decode().expect("begin");
            for &tok in &gold {
                let site = st.active_site().expect("active site");
                // A free position always reports a partial parse with a rule.
                if site.active_rule.is_some() {
                    saw_rule = true;
                }
                assert_eq!(site.status, "partial");
                let pool_end = SQL_SHARED_PAYLOAD_START + SQL_SHARED_PAYLOAD_POOL_WIDTH;
                if (SQL_SHARED_PAYLOAD_START..pool_end).contains(&tok) {
                    saw_payload_band_decision = true;
                }
                st.apply(tok).expect("apply");
            }
            assert!(st.is_complete().expect("complete"));
        }
        assert!(
            saw_rule,
            "expected at least one active_rule on a free position"
        );
        assert!(
            saw_payload_band_decision,
            "expected a decision in the shared payload pool, where band is role-ambiguous"
        );
    }

    /// Python dictionaries retain qualified indices after an elided parent.
    #[test]
    fn sql_active_menu_bindings_pyo3_retains_implicit_parent_and_legacy_shape() {
        use crate::provenance::sha256_hex;
        use crate::sql_schema_linking::layout::SQL_SHARED_PAYLOAD_START;

        let catalogs =
            include_str!("../../tests/fixtures/sql_schema_linking/qualified_menu/tables.json");
        let rt = PySqlSchemaLinkingRuntime::new(
            catalogs,
            &sha256_hex(catalogs.as_bytes()),
            "qualified_menu_permuted",
            None,
            Some("factored"),
            None,
        )
        .expect("fixture runtime");
        with_python(|py| {
            let tokens = rt
                .encode_elided(py, "SELECT Name FROM Alpha")
                .expect("encode");
            let mut state = rt.begin_elided_tct_decode().expect("begin");
            let mut saw_column = false;
            for token in tokens {
                let menu = state.active_menu_bindings(py).expect("qualified dict");
                let serialized: String = py
                    .import("json")
                    .expect("json")
                    .call_method1("dumps", (&menu,))
                    .expect("serialize")
                    .extract()
                    .expect("JSON string");
                let value: serde_json::Value = serde_json::from_str(&serialized).expect("JSON");
                assert_eq!(menu.len(), 7);
                assert_eq!(state.active_menu(py).expect("legacy dict").len(), 6);
                if value["kind"] == "column" {
                    saw_column = true;
                    assert_eq!(value["digits_seen"], 1);
                    assert_eq!(
                        value["referents"],
                        serde_json::json!([
                            {"ordinal": 0, "name": "Id"}, {"ordinal": 1, "name": "Name"}
                        ])
                    );
                    assert_eq!(
                        value["bindings"],
                        serde_json::json!([
                            {"token_id": SQL_SHARED_PAYLOAD_START, "ordinal": 0,
                             "table_ordinal": 2, "table_name": "Alpha",
                             "column_ordinal": 0, "column_name": "Id"},
                            {"token_id": SQL_SHARED_PAYLOAD_START + 1, "ordinal": 1,
                             "table_ordinal": 2, "table_name": "Alpha",
                             "column_ordinal": 1, "column_name": "Name"}
                        ])
                    );
                }
                state.apply(token).expect("apply");
            }
            assert!(saw_column);
        });
    }

    /// A nonempty BPE merge ends on a parent choice with an explicit next identity.
    #[test]
    fn sql_active_menu_bindings_bpe_merge_crosses_parent_then_names_columns() {
        use crate::provenance::sha256_hex;
        use crate::sql_schema_linking::layout::SQL_SHARED_PAYLOAD_START;

        let catalogs =
            include_str!("../../tests/fixtures/sql_schema_linking/qualified_menu/tables.json");
        let rt = PySqlSchemaLinkingRuntime::new(
            catalogs,
            &sha256_hex(catalogs.as_bytes()),
            "qualified_menu_permuted",
            None,
            Some("factored"),
            None,
        )
        .expect("fixture runtime");
        with_python(|py| {
            let sql = "SELECT Beta.Name FROM Alpha JOIN Beta ON Alpha.Id = Beta.Id";
            let tokens = rt.encode_elided(py, sql).expect("encode");
            let mut probe = rt.inner.begin_elided_tct_decode().expect("probe");
            let mut parent_position = None;
            for (position, &token) in tokens.iter().enumerate() {
                if probe.active_menu().expect("menu site").kind == "column_table" {
                    parent_position = Some(position);
                    break;
                }
                probe.apply(token).expect("probe apply");
            }
            let parent_position = parent_position.expect("join exposes a free parent choice");
            let merge_start = parent_position.checked_sub(1).expect("preceding choice");
            let pair = [tokens[merge_start], tokens[parent_position]];
            // A fixed source witness: the first column is ON-lhs Alpha.Id and
            // Alpha is catalog table 2. The merge crosses the structural
            // expression choice and consumes that parent-table menu digit.
            assert!(pair[0] < SQL_SHARED_PAYLOAD_START);
            assert_eq!(pair[1], SQL_SHARED_PAYLOAD_START + 2);
            let merge_id = rt.vocab_size();
            let table = BpeMergeTable {
                schema: "qualified-menu-parent-crossing-bpe".to_owned(),
                base_vocab_size: rt.vocab_size(),
                merges: vec![BpeMerge {
                    pair,
                    new_token: merge_id,
                    frequency: 1,
                }],
                metadata: BpeMetadata {
                    trained_on_instances: 1,
                    training_date: "qualified-menu-fixture".to_owned(),
                    max_merges: 1,
                    dict_size: None,
                },
            };
            let file = tempfile::NamedTempFile::new().expect("merge file");
            std::fs::write(file.path(), serde_json::to_vec(&table).expect("merge JSON"))
                .expect("write deterministic merge");
            let mut state = rt
                .begin_elided_bpe_tct_decode(file.path().to_str().expect("path"), 1)
                .expect("BPE walker");
            for &token in &tokens[..merge_start] {
                state.apply(token).expect("prefix apply");
            }
            assert!(
                state
                    .allowed_token_ids()
                    .expect("BPE mask")
                    .contains(&merge_id)
            );
            assert_eq!(state.apply(merge_id).expect("actual merge apply"), pair);
            assert_eq!(state.consumed_bpe_tokens(), merge_start + 1);
            assert_eq!(
                state.state.allowed_token_ids().expect("next base mask"),
                [SQL_SHARED_PAYLOAD_START, SQL_SHARED_PAYLOAD_START + 1]
            );
            let menu = state.active_menu_bindings(py).expect("post-merge bindings");
            let serialized: String = py
                .import("json")
                .expect("json")
                .call_method1("dumps", (&menu,))
                .expect("serialize")
                .extract()
                .expect("JSON string");
            let value: serde_json::Value = serde_json::from_str(&serialized).expect("JSON");
            assert_eq!(value["kind"], "column");
            assert_eq!(value["digits_seen"], 1);
            assert_eq!(value["width"], 2);
            assert_eq!(value["resolved"], true);
            // Independent catalog oracle: a nonempty table-2 Id/Name menu,
            // rather than equality between two calls sharing the resolver.
            assert_eq!(
                value["bindings"],
                serde_json::json!([
                    {"token_id": SQL_SHARED_PAYLOAD_START, "ordinal": 0,
                     "table_ordinal": 2, "table_name": "Alpha",
                     "column_ordinal": 0, "column_name": "Id"},
                    {"token_id": SQL_SHARED_PAYLOAD_START + 1, "ordinal": 1,
                     "table_ordinal": 2, "table_name": "Alpha",
                     "column_ordinal": 1, "column_name": "Name"}
                ])
            );
            for &token in &tokens[parent_position + 1..] {
                state.apply(token).expect("suffix apply");
            }
            assert!(state.is_complete().expect("complete"));
        });
    }

    /// Shared merge-table fixture for elided-BPE pyclass tests (tempfile path).
    fn elided_bpe_merge_fixture() -> (
        tempfile::NamedTempFile,
        SequentialBpeEncoder,
        usize,
        Vec<(String, String)>,
    ) {
        use crate::sql_schema_linking::catalog::load_fixture_bundle;
        use crate::sql_schema_linking::elided_bpe_mask::{
            bpe_codec_for_merges, train_elided_bpe_table,
        };
        use crate::sql_schema_linking::layout::SQL_SYNTAX_LAYOUT_VOCAB_SIZE;
        use crate::sql_schema_linking::runtime::SqlSchemaLinkingRuntime;
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

        let bundle = load_fixture_bundle().expect("bundle");
        let file: GoldFile =
            serde_json::from_slice(&bundle.gold_queries_bytes).expect("parse gold");
        let rust_rt = SqlSchemaLinkingRuntime::fixture().expect("fixture rt");
        // These tests compare PyO3 with Rust's current canonical SQL text,
        // while the fixture source remains unchanged as encoder input.
        let queries: Vec<(String, String)> = file
            .queries
            .into_iter()
            .map(|q| {
                let tokens = rust_rt.encode(&q.sql).expect("encode fixture source");
                (
                    q.id,
                    rust_rt.decode(&tokens).expect("canonical fixture SQL"),
                )
            })
            .collect();
        assert!(!queries.is_empty());
        let sequences: Vec<Vec<u32>> = queries
            .iter()
            .map(|(_, sql)| rust_rt.encode_elided(sql).expect("elided"))
            .collect();
        let merge_count = 16usize;
        let table = train_elided_bpe_table(
            &sequences,
            SQL_SYNTAX_LAYOUT_VOCAB_SIZE,
            merge_count,
            "sql-fixture-elided-binding",
        );
        let (encoder, _decoder, _bpe_vocab_size) =
            bpe_codec_for_merges(&table, merge_count).expect("codec");
        let merge_file = tempfile::NamedTempFile::new().expect("tmp");
        std::fs::write(merge_file.path(), serde_json::to_vec(&table).expect("ser"))
            .expect("write merges");
        (merge_file, encoder, merge_count, queries)
    }

    #[test]
    fn sql_elided_bpe_tct_decode_state_fixture_gold_path_and_pad() {
        use crate::sql_schema_linking::layout::SQL_SYNTAX_LAYOUT_VOCAB_SIZE;

        let (merge_file, encoder, merge_count, queries) = elided_bpe_merge_fixture();
        let merge_path = merge_file.path().to_str().expect("utf8 path");
        // Reload table only for scalar pin checks against the constructor.
        let table = BpeMergeTable::from_file(merge_path).expect("reload table");
        let bpe_vocab_size = table.bpe_start() + merge_count as u32;

        let py_rt = PySqlSchemaLinkingRuntime::fixture().expect("py fixture");
        with_python(|py| {
            let sql = &queries[0].1;
            let elided = py_rt.encode_elided(py, sql).expect("encode_elided");
            let bpe_stream = encoder.encode(&elided);

            let mut st = py_rt
                .begin_elided_bpe_tct_decode(merge_path, merge_count)
                .expect("begin bpe");
            assert_eq!(st.base_vocab_size(), SQL_SYNTAX_LAYOUT_VOCAB_SIZE);
            assert_eq!(st.pad_token(), SQL_SYNTAX_LAYOUT_PAD_TOKEN);
            assert_eq!(st.bpe_start(), table.bpe_start());
            assert_eq!(st.total_vocab_size(), bpe_vocab_size);
            assert_eq!(st.bpe_vocab_size(), bpe_vocab_size);

            let mut accumulated = Vec::new();
            let mut base = py_rt.begin_elided_tct_decode().expect("base walker");
            for &gold_bpe in &bpe_stream {
                // Non-empty merges can span sites; query the new API only at
                // merge boundaries and compare to independently replayed base steps.
                let bpe_menu = st.active_menu_bindings(py).expect("BPE bindings");
                let base_menu = base.active_menu_bindings(py).expect("base bindings");
                assert!(bpe_menu.eq(&base_menu).expect("dict equality"));
                let allowed = st.allowed_token_ids().expect("mask");
                assert!(
                    allowed.contains(&gold_bpe),
                    "gold BPE {gold_bpe} must be admissible; allowed={allowed:?}"
                );
                assert!(
                    !allowed.contains(&SQL_SYNTAX_LAYOUT_PAD_TOKEN),
                    "PAD must never be admissible"
                );
                let expansion = st.apply(gold_bpe).expect("apply gold");
                for &token in &expansion {
                    base.apply(token).expect("base apply");
                }
                accumulated.extend(expansion);
            }
            assert!(
                st.active_menu_bindings(py)
                    .expect("BPE EOF")
                    .eq(base.active_menu_bindings(py).expect("base EOF"))
                    .expect("EOF equality")
            );
            assert!(st.is_complete().expect("complete"));
            assert_eq!(accumulated, elided, "base expansion must match elided gold");
            assert_eq!(st.consumed_bpe_tokens(), bpe_stream.len());
            assert_eq!(st.render_sql().expect("render"), *sql);

            // Reject an out-of-mask identity when possible.
            let mut fresh = py_rt
                .begin_elided_bpe_tct_decode(merge_path, merge_count)
                .expect("begin again");
            let allowed = fresh.allowed_token_ids().expect("mask");
            let reject =
                (0..64u32).find(|id| *id != SQL_SYNTAX_LAYOUT_PAD_TOKEN && !allowed.contains(id));
            if let Some(bad) = reject {
                let err = fresh.apply(bad).expect_err("must reject");
                let msg = err.to_string();
                assert!(
                    msg.contains(&bad.to_string()) && msg.contains("not admissible"),
                    "reject must name the id: {msg}"
                );
            }
        });
    }

    #[test]
    fn sql_elided_bpe_tct_decode_state_clone_is_independent() {
        use crate::sql_schema_linking::layout::SQL_SYNTAX_LAYOUT_VOCAB_SIZE;

        let (merge_file, encoder, merge_count, queries) = elided_bpe_merge_fixture();
        let merge_path = merge_file.path().to_str().expect("utf8 path");
        let table = BpeMergeTable::from_file(merge_path).expect("reload table");
        let bpe_vocab_size = table.bpe_start() + merge_count as u32;

        let py_rt = PySqlSchemaLinkingRuntime::fixture().expect("py fixture");
        with_python(|py| {
            // Prefer a multi-token BPE stream so mid-stream clone is meaningful.
            let bpe_stream = queries
                .iter()
                .map(|(_, qsql)| {
                    let elided = py_rt.encode_elided(py, qsql).expect("encode_elided");
                    encoder.encode(&elided)
                })
                .find(|stream| stream.len() >= 3)
                .expect("need a gold query with >= 3 BPE tokens");

            let mut base = py_rt
                .begin_elided_bpe_tct_decode(merge_path, merge_count)
                .expect("begin bpe");

            // Drive past the first token so consumed_bpe_tokens is non-zero.
            base.apply(bpe_stream[0]).expect("apply first");
            assert_eq!(base.consumed_bpe_tokens(), 1);

            let snapshot_allowed = base.allowed_token_ids().expect("allowed before clone");
            let snapshot_expanded = base.expanded_tokens();
            let snapshot_consumed = base.consumed_bpe_tokens();

            // (1) clone preserves admissible set
            let mut clone_a = base.clone();
            assert_eq!(
                clone_a.allowed_token_ids().expect("clone allowed"),
                snapshot_allowed,
                "clone must start with the same admissible set"
            );

            // (3) scalar fields survive
            assert_eq!(clone_a.base_vocab_size(), SQL_SYNTAX_LAYOUT_VOCAB_SIZE);
            assert_eq!(clone_a.base_vocab_size(), base.base_vocab_size());
            assert_eq!(clone_a.bpe_start(), table.bpe_start());
            assert_eq!(clone_a.bpe_start(), base.bpe_start());
            assert_eq!(clone_a.bpe_vocab_size(), bpe_vocab_size);
            assert_eq!(clone_a.bpe_vocab_size(), base.bpe_vocab_size());
            assert_eq!(clone_a.total_vocab_size(), bpe_vocab_size);
            assert_eq!(clone_a.total_vocab_size(), base.total_vocab_size());
            assert_eq!(clone_a.pad_token(), SQL_SYNTAX_LAYOUT_PAD_TOKEN);
            assert_eq!(clone_a.pad_token(), base.pad_token());

            // (4) consumed_bpe_tokens is preserved, not reset
            assert_eq!(
                clone_a.consumed_bpe_tokens(),
                snapshot_consumed,
                "clone must carry decode progress, not reset it"
            );
            assert_eq!(clone_a.expanded_tokens(), snapshot_expanded);

            // (2) independence: advance only the clone
            clone_a.apply(bpe_stream[1]).expect("apply on clone_a");
            assert_eq!(clone_a.consumed_bpe_tokens(), snapshot_consumed + 1);
            assert_eq!(
                base.consumed_bpe_tokens(),
                snapshot_consumed,
                "advancing clone must not change original consumed_bpe_tokens"
            );
            assert_eq!(
                base.expanded_tokens(),
                snapshot_expanded,
                "advancing clone must not change original expanded_tokens"
            );
            assert_eq!(
                base.allowed_token_ids()
                    .expect("base allowed after clone apply"),
                snapshot_allowed,
                "advancing clone must not change original allowed_token_ids"
            );
            assert_ne!(
                clone_a.allowed_token_ids().expect("clone_a allowed"),
                snapshot_allowed,
                "after apply, clone admissible set should diverge from the pre-apply snapshot"
            );

            // Converse: advance only the original; clone_a must stay put.
            let clone_a_allowed = clone_a.allowed_token_ids().expect("clone_a mask");
            let clone_a_expanded = clone_a.expanded_tokens();
            let clone_a_consumed = clone_a.consumed_bpe_tokens();
            base.apply(bpe_stream[1]).expect("apply on base");
            assert_eq!(
                clone_a.consumed_bpe_tokens(),
                clone_a_consumed,
                "advancing original must not change clone consumed_bpe_tokens"
            );
            assert_eq!(clone_a.expanded_tokens(), clone_a_expanded);
            assert_eq!(
                clone_a
                    .allowed_token_ids()
                    .expect("clone_a after base apply"),
                clone_a_allowed
            );

            // (5) mid-stream fork down different admissible gold branches →
            // different render_sql(), neither corrupted by the other.
            // Prefer two gold streams that share a non-empty BPE prefix so the
            // fork is genuinely mid-stream; fall back to empty-prefix root.
            let gold_streams: Vec<(String, Vec<u32>)> = queries
                .iter()
                .map(|(_, qsql)| {
                    let elided = py_rt.encode_elided(py, qsql).expect("encode_elided");
                    (qsql.clone(), encoder.encode(&elided))
                })
                .filter(|(_, stream)| !stream.is_empty())
                .collect();
            assert!(
                gold_streams.len() >= 2,
                "need at least two non-empty gold BPE streams"
            );

            let (sql_a, stream_a) = &gold_streams[0];
            let (sql_b, stream_b) = gold_streams
                .iter()
                .find(|(s, stream)| s != sql_a && stream.as_slice() != stream_a.as_slice())
                .expect("need a second distinct gold SQL/BPE stream");
            assert_ne!(sql_a, sql_b, "distinct gold SQL for branch divergence");

            let shared_prefix: usize = stream_a
                .iter()
                .zip(stream_b.iter())
                .take_while(|(a, b)| a == b)
                .count();

            let mut fork_root = py_rt
                .begin_elided_bpe_tct_decode(merge_path, merge_count)
                .expect("begin fork root");
            for &tok in &stream_a[..shared_prefix] {
                fork_root.apply(tok).expect("apply shared prefix");
            }
            assert_eq!(fork_root.consumed_bpe_tokens(), shared_prefix);

            let mut beam_a = fork_root.clone();
            let mut beam_b = fork_root.clone();
            // Snapshot after fork to prove later that finishing one beam
            // does not touch the other.
            let beam_b_allowed_at_fork = beam_b.allowed_token_ids().expect("b mask at fork");
            let beam_b_expanded_at_fork = beam_b.expanded_tokens();
            let beam_b_consumed_at_fork = beam_b.consumed_bpe_tokens();

            for &tok in &stream_a[shared_prefix..] {
                beam_a.apply(tok).expect("beam_a apply");
            }
            assert!(beam_a.is_complete().expect("beam_a complete"));
            let rendered_a = beam_a.render_sql().expect("beam_a render");
            assert_eq!(rendered_a, *sql_a);

            // beam_b must still sit at the fork point (independence).
            assert_eq!(beam_b.consumed_bpe_tokens(), beam_b_consumed_at_fork);
            assert_eq!(beam_b.expanded_tokens(), beam_b_expanded_at_fork);
            assert_eq!(
                beam_b.allowed_token_ids().expect("b mask after a finished"),
                beam_b_allowed_at_fork
            );

            for &tok in &stream_b[shared_prefix..] {
                beam_b.apply(tok).expect("beam_b apply");
            }
            assert!(beam_b.is_complete().expect("beam_b complete"));
            let rendered_b = beam_b.render_sql().expect("beam_b render");
            assert_eq!(rendered_b, *sql_b);
            assert_ne!(
                rendered_a, rendered_b,
                "different gold branches must yield different SQL"
            );
            // beam_a still intact after beam_b finished.
            assert_eq!(beam_a.render_sql().expect("a still intact"), rendered_a);
            assert_eq!(beam_a.consumed_bpe_tokens(), stream_a.len());
            assert_eq!(beam_b.consumed_bpe_tokens(), stream_b.len());
        });
    }
}
