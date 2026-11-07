//! Raw syntax-token state machine.
//!
//! This module is the syntax-level boundary that later Go and C# tokenizer
//! generators should own directly: a compiled `StructuralModel` plus raw token
//! IDs. It intentionally does not consume route reports, case IDs,
//! source-instance segment sidecars, BPE provenance, GoIR, or ArenaIR.

// Rich diagnostic context on `RawSyntaxError` (rule/field/payload class) is
// intentional for strict decode accounting; keep Result<(), RawSyntaxError>
// ergonomic rather than boxing every return site.
#![allow(clippy::result_large_err)]

use crate::go_ast_schema_inventory::{
    GO_AST_SCHEMA_INVENTORY_FIXTURE_PATH, GO_STRUCTURAL_TOKEN_ASSIGNMENT_APPEND_RULE,
};
use crate::grammar_ir_structural_model::{
    GrammarStructuralModelSidecar, GrammarStructuralTokenLayout,
};
use crate::shared_alphabet_layout::{
    SHARED_ALPHABET_MAX_BRANCH_ARITY, SyntaxTokenLayoutMode, shared_alphabet_choice_token_id,
    shared_alphabet_token_layout, shared_alphabet_unpack_choice_offset,
};
use crate::syntax_backend_boundary::SyntaxBackendBoundaryError;
use crate::syntax_go_source_instance_projection::{
    ProjectionTokenLayoutRangeSummary, ProjectionTokenLayoutSummary,
    RAW_TOKEN_ALTERNATIVE_NODE_START_OFFSET, node_start_alternative_token_ids_from_sidecar,
};
use serde::Serialize;
use std::collections::{BTreeMap, BTreeSet};

mod encode;
mod factored_menu_codec;
mod integer_codec;
mod menu_codec;
mod model_facts;
mod raw_replay;

/// Encode an integer to canonical multi-digit form on the shared 256 band.
///
/// Public for subset-framework / tooling that emit integer payloads without
/// going through the full raw-syntax tree encode path.
pub fn integer_codec_encode_digits(value: u32) -> Result<Vec<u8>, String> {
    integer_codec::encode_integer_digits(value)
}

/// Inclusive max integer value encodable on the 256 band (variable-width).
pub const INTEGER_PAYLOAD_CAPACITY_MAX: u32 = integer_codec::INTEGER_CAPACITY_MAX;

pub(crate) use self::factored_menu_codec::FACTORED_COLUMN_MENU_DIGITS;
pub(crate) use self::menu_codec::menu_digit_width;
use self::model_facts::{ModelFacts, terminal_class_name};
use self::raw_replay::{CompletionMode, RawSyntaxStateMachine};
pub use self::raw_replay::{RawSyntaxLiveState, singleton_admissible_raw_token};

pub const RAW_NODE_START_OFFSET: u32 = 0;
pub const RAW_NODE_END_OFFSET: u32 = 1;
pub const RAW_FIELD_START_OFFSET: u32 = 2;
pub const RAW_FIELD_END_OFFSET: u32 = 3;
pub const RAW_FIELD_MISSING_OFFSET: u32 = 4;
pub const RAW_REPEATED_EMPTY_OFFSET: u32 = 5;
pub const RAW_TERMINAL_OFFSET: u32 = 6;
pub const RAW_PAYLOAD_END_OFFSET: u32 = 7;
pub const RAW_BOOLEAN_FALSE_OFFSET: u32 = 8;
pub const RAW_BOOLEAN_TRUE_OFFSET: u32 = 9;
pub const RAW_SYNTAX_STATE_MACHINE_CORE_BOUNDARY_ID: &str = "raw-syntax-state-machine-core-v1";
pub const RAW_SYNTAX_STATE_MACHINE_CORE_BOUNDARY: &str =
    "raw token prefix -> state -> admissible tokens -> transition/decode result";

/// Upper bound on one eager mask-forced auto-apply run during elided-stream
/// expansion. The cap is a totality guard, not a tuning knob: a grammar
/// with unbounded required recursion would otherwise make the drain loop
/// diverge, while the checked corpora stay far below it (record streams
/// peak at stack depth 16 and forced runs are short close/open chains).
/// Exceeding the cap fails with the typed `forced_run_limit_exceeded`
/// error instead of looping.
pub const ELIDED_FORCED_RUN_LIMIT: usize = 65_536;

pub trait RawSyntaxStateMachineCore {
    fn boundary_id(&self) -> &'static str {
        RAW_SYNTAX_STATE_MACHINE_CORE_BOUNDARY_ID
    }

    fn boundary_contract(&self) -> &'static str {
        RAW_SYNTAX_STATE_MACHINE_CORE_BOUNDARY
    }

    fn token_roles(&self) -> &SyntaxTokenRoles;

    fn state_after_raw_prefix(
        &self,
        raw_token_ids: &[u32],
    ) -> Result<RawSyntaxState, RawSyntaxError>;

    fn admissible_next_raw_tokens(
        &self,
        raw_token_ids: &[u32],
    ) -> Result<RawSyntaxAdmissibleTokens, RawSyntaxError> {
        Ok(self.state_after_raw_prefix(raw_token_ids)?.admissible)
    }

    fn candidate_raw_token_transition(
        &self,
        raw_prefix_token_ids: &[u32],
        candidate_raw_token_id: u32,
    ) -> RawSyntaxTransition;

    fn strict_decode_complete(
        &self,
        raw_token_ids: &[u32],
    ) -> Result<RawSyntaxState, RawSyntaxError>;

    fn decode_complete_tree(
        &self,
        raw_token_ids: &[u32],
    ) -> Result<RawSyntaxDecodedTree, RawSyntaxError>;

    fn selected_completion_trace(
        &self,
        raw_token_ids: &[u32],
        sample_steps: &BTreeSet<usize>,
    ) -> Result<RawSyntaxSelectedCompletionTrace, RawSyntaxError>;

    /// Incremental `advance()` handle positioned at the empty raw-token
    /// prefix, for implementations that expose one. The default is `None`
    /// so trait implementations are not forced to expose engine internals;
    /// callers that consume the handle must keep a replay-shaped fallback
    /// for cores without incremental access. `CompiledSyntaxModel`
    /// overrides this with its `begin_stream` handle.
    fn begin_raw_stream(&self) -> Option<RawSyntaxStream<'_>> {
        None
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompiledSyntaxModel {
    facts: ModelFacts,
    roles: SyntaxTokenRoles,
    /// Menu-index site bindings keyed by `(owner_rule, owner_field)` (**P3**/**P4**).
    /// Empty by default: menu code paths are unreachable (**P10**).
    menu_bindings: MenuSiteBindings,
}

/// How a bound menu site resolves its admissible ordinal range (**P3**/**P4**/**P5**).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MenuSiteKind {
    /// Fixed menu (table sites): ordinals `0..cardinality`; completing a value
    /// pushes the ordinal onto the nearest scope-root frame's `scope_tables`.
    Fixed {
        cardinality: u32,
        /// When true, accepted ordinals are appended to the active scope (**P5**).
        pushes_scope: bool,
    },
    /// Column sites: scope-relative ordinals into the concatenation, over
    /// tables currently in scope (FROM/JOIN order), of each table's columns
    /// in catalog order (**P4**). `columns_per_table[i]` is the column count
    /// for catalog table index `i`.
    ScopeRelativeColumns { columns_per_table: Vec<u32> },
    /// Factored column sites: the same column reference, addressed as **two
    /// positional menu digits** instead of one radix code over the flat
    /// scope-relative concatenation.
    ///
    /// - digit 0 is the **catalog** table index, masked to the tables
    ///   currently in scope (so the digit alphabet is the same one the FROM
    ///   clause's table menu already uses, and a column's address does not
    ///   move with FROM order);
    /// - digit 1 is the column index **inside that table**, in catalog order.
    ///
    /// The second digit's cardinality is a function of the first digit's
    /// value, which the radix engine (one flat cardinality per site) cannot
    /// express — hence a distinct variant rather than a wider
    /// `ScopeRelativeColumns`. The *decoded value* is still the flat
    /// scope-relative ordinal, so the AST, render, join-ON frame memory, and
    /// every downstream consumer are unchanged; only the token stream differs.
    FactoredScopeColumns { columns_per_table: Vec<u32> },
}

impl MenuSiteKind {
    /// Upper bound used for band-overflow checks at bind time (not the live mask).
    pub fn band_limit(&self) -> u32 {
        match self {
            Self::Fixed { cardinality, .. } => *cardinality,
            Self::ScopeRelativeColumns { columns_per_table }
            | Self::FactoredScopeColumns { columns_per_table } => columns_per_table
                .iter()
                .copied()
                .fold(0u32, u32::saturating_add),
        }
    }

    /// Per-table column counts for either column-site kind.
    pub fn columns_per_table(&self) -> Option<&[u32]> {
        match self {
            Self::Fixed { .. } => None,
            Self::ScopeRelativeColumns { columns_per_table }
            | Self::FactoredScopeColumns { columns_per_table } => Some(columns_per_table),
        }
    }

    /// True when the site emits a factored `(table, column)` digit pair.
    pub fn is_factored_columns(&self) -> bool {
        matches!(self, Self::FactoredScopeColumns { .. })
    }
}

/// Live menu-site cursor reconstructed from owned decode state.
///
/// SQL (and any other menu frontend) uses this to name the referents the
/// current free menu decision is choosing among. Role cannot be recovered
/// from a token-id band under the collapsed shared payload pool.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LiveMenuCursor {
    pub owner_rule: String,
    pub owner_field: String,
    pub site_kind: MenuSiteKind,
    /// Catalog table ordinals in FROM/JOIN order on the nearest scope root.
    pub scope_tables: Vec<u32>,
    pub digits_seen: u32,
    /// Big-endian partial ordinal (factored digit 0 is the catalog table).
    pub menu_partial: u32,
    pub menu_cardinality: Option<u32>,
    /// Digit width cached by the engine once the first digit is consumed
    /// (`None` before that; recompute from cardinality and band width).
    pub menu_width: Option<u32>,
}

/// Structural join-ON constraint binding (SQL join-key masking).
///
/// When bound, column-menu sites whose `column_rule` frame is a **direct**
/// child of a `comparison_rule` frame that itself sits under an open
/// `join_rule.on_field` (within the same scope root) are constrained: after
/// a left-side column from table `T_l` and an `eq_op_rule` operator, the
/// right-side column menu admits only columns of the just-joined table when
/// `T_l` is an earlier table, and only earlier-table columns when `T_l` is
/// the just-joined table. Nested columns (aggregates, subqueries) and
/// non-equality comparisons are deliberately unconstrained; gold compatibility
/// was censused on plain `column = column` ON pairs only.
///
/// `None` by default: every new code path is unreachable without a binding,
/// so menu-less models (Go/C#) stay bit-identical (**P10**).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JoinOnConstraint {
    /// Join rule whose `on_field` scopes the constraint (e.g. `join_clause`).
    pub join_rule: String,
    /// The join rule's ON field name (e.g. `on`).
    pub on_field: String,
    /// Comparison rule carrying `left`/`op`/`right` (e.g. `comparison`).
    pub comparison_rule: String,
    /// Comparison field holding the left expression (e.g. `left`).
    pub comparison_left_field: String,
    /// Comparison field holding the operator alternative (e.g. `op`).
    pub comparison_op_field: String,
    /// Comparison field holding the right expression (e.g. `right`).
    pub comparison_right_field: String,
    /// Operator alternative rule that activates the pair constraint (`op_eq`).
    pub eq_op_rule: String,
    /// Column menu owner rule (e.g. `column_ref`).
    pub column_rule: String,
    /// Column menu owner field (e.g. `menu_index`).
    pub column_field: String,
    /// Pure multi-alt wrapper rule sitting between `comparison` fields and
    /// their concrete children (e.g. `expression`). Only frames of exactly
    /// this rule are skipped when resolving a column's comparison ancestor,
    /// so columns nested under aggregates or subqueries stay unconstrained.
    pub expression_wrapper_rule: String,
}

/// One catalog foreign-key edge as menu ordinals (table index, per-table column).
///
/// Bound only under SQL `fk_declared` join-ON policy. Empty keeps the
/// unique-FK shrink unreachable so menu-less models stay bit-identical (**P10**).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JoinOnFkPair {
    pub from_table: u32,
    pub from_column: u32,
    pub to_table: u32,
    pub to_column: u32,
}

/// Scope-relative menu ordinal bindings for IntegerPayload menu sites (**P3**/**P4**).
///
/// Empty bindings keep menu mask/apply/encode branches unreachable so models
/// compiled without a menu binding stay bit-identical (**P10**).
///
/// `max_menu_digits` caps how many base-B digits a site may use when encoding
/// a menu ordinal (`capacity = band_width.pow(max_menu_digits)`). Default is
/// **1** (historical single-token menus). A derived `Default` would zero this
/// field and break bind-time fail-closed tests; keep the manual impl.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MenuSiteBindings {
    /// `(owner_rule, owner_field)` → menu site kind.
    sites: BTreeMap<(String, String), MenuSiteKind>,
    /// Rules whose frames carry `scope_tables` (**P5**). Typically the select root.
    scope_root_rules: BTreeSet<String>,
    /// Maximum radix digits per menu ordinal (T-B). Default **1**.
    pub max_menu_digits: u32,
    /// Structural join-ON constraint; `None` keeps all join-ON paths dead (**P10**).
    join_on: Option<JoinOnConstraint>,
    /// Declared FK pairs; empty keeps the unique-FK shrink dead (**P10**).
    join_on_fk: Vec<JoinOnFkPair>,
    /// When true, `node_end` is illegal while any field remains unvisited.
    ///
    /// Encode already walks every field (optional missing → `field_missing`).
    /// Historical decode also admits `node_end` as a second spelling of
    /// "skip remaining trailing optionals", which is a free H=0 decision
    /// (`rule._`) immediately before the real field decision. SQL rank-1
    /// merge forces visitation so that skip-vs-visit is mask-forced/elided
    /// and the subsequent field keeps its grammar alternatives.
    visit_all_remaining_fields: bool,
}

impl Default for MenuSiteBindings {
    fn default() -> Self {
        Self {
            sites: BTreeMap::new(),
            scope_root_rules: BTreeSet::new(),
            max_menu_digits: 1,
            join_on: None,
            join_on_fk: Vec::new(),
            visit_all_remaining_fields: false,
        }
    }
}

impl MenuSiteBindings {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn is_empty(&self) -> bool {
        self.sites.is_empty()
    }

    /// Set the per-binding digit budget used by bind-time capacity checks and
    /// by the multi-digit engine (T-C). SQL opts in to `2` at T-D; default `1`
    /// keeps historical single-token menus. Callers must not pass `0`
    /// (capacity would collapse).
    pub fn set_max_menu_digits(&mut self, max_menu_digits: u32) {
        self.max_menu_digits = max_menu_digits;
    }

    /// Bind a fixed menu site (does not push scope). Used by table-only probes.
    pub fn insert(
        &mut self,
        owner_rule: impl Into<String>,
        owner_field: impl Into<String>,
        cardinality: u32,
    ) {
        self.sites.insert(
            (owner_rule.into(), owner_field.into()),
            MenuSiteKind::Fixed {
                cardinality,
                pushes_scope: false,
            },
        );
    }

    /// Bind a fixed menu site that pushes accepted ordinals onto the scope stack.
    pub fn insert_table_site(
        &mut self,
        owner_rule: impl Into<String>,
        owner_field: impl Into<String>,
        cardinality: u32,
    ) {
        self.sites.insert(
            (owner_rule.into(), owner_field.into()),
            MenuSiteKind::Fixed {
                cardinality,
                pushes_scope: true,
            },
        );
    }

    /// Bind a scope-relative column menu site (**P4**/**P5**).
    pub fn insert_scope_column_site(
        &mut self,
        owner_rule: impl Into<String>,
        owner_field: impl Into<String>,
        columns_per_table: Vec<u32>,
    ) {
        self.sites.insert(
            (owner_rule.into(), owner_field.into()),
            MenuSiteKind::ScopeRelativeColumns { columns_per_table },
        );
    }

    /// Bind a **factored** column menu site: the same scope-relative column
    /// reference emitted as `(catalog table, column in that table)` digits.
    ///
    /// Requires `max_menu_digits >= 2` and both digit alphabets to fit the menu
    /// band; [`CompiledSyntaxModel::with_menu_bindings`] enforces that and
    /// fails closed otherwise.
    pub fn insert_factored_column_site(
        &mut self,
        owner_rule: impl Into<String>,
        owner_field: impl Into<String>,
        columns_per_table: Vec<u32>,
    ) {
        self.sites.insert(
            (owner_rule.into(), owner_field.into()),
            MenuSiteKind::FactoredScopeColumns { columns_per_table },
        );
    }

    /// Per-table column counts for a bound column site (flat or factored).
    pub fn column_site_columns_per_table(
        &self,
        owner_rule: &str,
        owner_field: &str,
    ) -> Option<&[u32]> {
        self.site_kind(owner_rule, owner_field)?.columns_per_table()
    }

    /// True when this site emits a factored `(table, column)` digit pair.
    ///
    /// False for every unbound field, so factored mask/apply/encode branches
    /// stay unreachable for menu-less models (**P10**).
    pub fn is_factored_column_site(&self, owner_rule: &str, owner_field: &str) -> bool {
        self.site_kind(owner_rule, owner_field)
            .is_some_and(MenuSiteKind::is_factored_columns)
    }

    /// Declare a grammar rule as a scope root (its frames hold `scope_tables`).
    pub fn mark_scope_root(&mut self, rule: impl Into<String>) {
        self.scope_root_rules.insert(rule.into());
    }

    pub fn is_scope_root(&self, rule: &str) -> bool {
        self.scope_root_rules.contains(rule)
    }

    /// Site kind if this field is a menu site.
    pub fn site_kind(&self, owner_rule: &str, owner_field: &str) -> Option<&MenuSiteKind> {
        self.sites
            .get(&(owner_rule.to_string(), owner_field.to_string()))
    }

    /// Fixed-site exclusive cardinality, if the site is a fixed menu.
    ///
    /// Scope-relative column sites return [`None`] here; use
    /// [`Self::resolve_cardinality`] with the live scope instead.
    pub fn cardinality(&self, owner_rule: &str, owner_field: &str) -> Option<u32> {
        match self.site_kind(owner_rule, owner_field)? {
            MenuSiteKind::Fixed { cardinality, .. } => Some(*cardinality),
            MenuSiteKind::ScopeRelativeColumns { .. }
            | MenuSiteKind::FactoredScopeColumns { .. } => None,
        }
    }

    /// Resolve live exclusive cardinality for a menu site given the current scope.
    ///
    /// Returns `None` when the field is not a menu site (empty-binding path).
    pub fn resolve_cardinality(
        &self,
        owner_rule: &str,
        owner_field: &str,
        scope_tables: &[u32],
    ) -> Option<u32> {
        match self.site_kind(owner_rule, owner_field)? {
            MenuSiteKind::Fixed { cardinality, .. } => Some(*cardinality),
            // Factored sites keep the same live *value* space (the flat
            // scope-relative ordinal); only the digit encoding differs, so the
            // empty-scope fail-closed check and the decoded payload value are
            // shared with the flat kind.
            MenuSiteKind::ScopeRelativeColumns { columns_per_table }
            | MenuSiteKind::FactoredScopeColumns { columns_per_table } => {
                let mut total = 0u32;
                for &table_idx in scope_tables {
                    let n = columns_per_table
                        .get(table_idx as usize)
                        .copied()
                        .unwrap_or(0);
                    total = total.saturating_add(n);
                }
                Some(total)
            }
        }
    }

    pub fn iter(&self) -> impl Iterator<Item = (&(String, String), &MenuSiteKind)> {
        self.sites.iter()
    }

    /// Bind the structural join-ON constraint (SQL join-key masking).
    pub fn bind_join_on_structural(&mut self, constraint: JoinOnConstraint) {
        self.join_on = Some(constraint);
    }

    /// Active join-ON constraint, if bound.
    pub fn join_on(&self) -> Option<&JoinOnConstraint> {
        self.join_on.as_ref()
    }

    /// Bind declared FK pairs for the unique-FK join-ON shrink.
    pub fn bind_join_on_fk_pairs(&mut self, pairs: Vec<JoinOnFkPair>) {
        self.join_on_fk = pairs;
    }

    /// Declared FK pairs, if any.
    pub fn join_on_fk_pairs(&self) -> &[JoinOnFkPair] {
        &self.join_on_fk
    }

    /// Require every remaining field to be visited before `node_end`.
    ///
    /// Default `false` keeps Go/C#/toy grammars and historical SQL streams
    /// bit-identical. SQL rank-1 sets this on the binding.
    pub fn set_visit_all_remaining_fields(&mut self, visit_all_remaining_fields: bool) {
        self.visit_all_remaining_fields = visit_all_remaining_fields;
    }

    /// Whether remaining fields must be visited before `node_end`.
    pub fn visit_all_remaining_fields(&self) -> bool {
        self.visit_all_remaining_fields
    }

    /// Unique FK column of `right_table` that pairs with `left_table`, if any.
    ///
    /// `None` when the tables share zero or two-or-more FK columns; callers
    /// keep the structural slice in those cases.
    pub fn unique_fk_right_column(&self, left_table: u32, right_table: u32) -> Option<u32> {
        let mut hits: BTreeSet<u32> = BTreeSet::new();
        for pair in &self.join_on_fk {
            if pair.from_table == left_table && pair.to_table == right_table {
                hits.insert(pair.to_column);
            } else if pair.to_table == left_table && pair.from_table == right_table {
                hits.insert(pair.from_column);
            }
        }
        if hits.len() == 1 {
            hits.iter().next().copied()
        } else {
            None
        }
    }
}

impl CompiledSyntaxModel {
    pub fn from_sidecar(
        sidecar: &GrammarStructuralModelSidecar,
    ) -> Result<Self, SyntaxBackendBoundaryError> {
        Self::from_sidecar_with_mode(sidecar, SyntaxTokenLayoutMode::PerGrammar)
    }

    /// Compile a structural-model sidecar under an explicit layout mode.
    ///
    /// `PerGrammar` is bit-identical to historical `from_sidecar` behavior.
    /// `SharedAlphabet` uses the fixed shared decision-token alphabet layout
    /// and domain-stable `CHOICE(n,k)` role assignment (I5 Decision 2 / 4);
    /// default absolute pins and production paths stay on `PerGrammar`.
    pub fn from_sidecar_with_mode(
        sidecar: &GrammarStructuralModelSidecar,
        mode: SyntaxTokenLayoutMode,
    ) -> Result<Self, SyntaxBackendBoundaryError> {
        match mode {
            SyntaxTokenLayoutMode::PerGrammar => {
                let layout =
                    ProjectionTokenLayoutSummary::from(&sidecar.structural_model.token_ranges);
                Self::from_sidecar_and_layout_with_mode(sidecar, &layout, mode)
            }
            SyntaxTokenLayoutMode::SharedAlphabet => {
                let shared = shared_alphabet_token_layout();
                let layout = ProjectionTokenLayoutSummary::from(&shared);
                Self::from_sidecar_and_layout_with_mode(sidecar, &layout, mode)
            }
        }
    }

    pub fn from_sidecar_and_layout(
        sidecar: &GrammarStructuralModelSidecar,
        layout: &ProjectionTokenLayoutSummary,
    ) -> Result<Self, SyntaxBackendBoundaryError> {
        Self::from_sidecar_and_layout_with_mode(sidecar, layout, SyntaxTokenLayoutMode::PerGrammar)
    }

    /// Like [`Self::from_sidecar_and_layout`], with an explicit layout mode.
    ///
    /// Under `SharedAlphabet`, `layout` must match
    /// [`shared_alphabet_token_layout`] ranges (vocab/pad/structural band);
    /// alternative tokens are assigned from domain-stable ordinals, not from
    /// free-slot scan or the Go pin table.
    pub fn from_sidecar_and_layout_with_mode(
        sidecar: &GrammarStructuralModelSidecar,
        layout: &ProjectionTokenLayoutSummary,
        mode: SyntaxTokenLayoutMode,
    ) -> Result<Self, SyntaxBackendBoundaryError> {
        let facts = ModelFacts::from_sidecar(sidecar)?;
        match mode {
            SyntaxTokenLayoutMode::PerGrammar => {
                // Section C of the shared-SM mixed-payload contract: bare
                // IntegerPayload on a mixed field is allowed only when the
                // token layout declares GrammarIntegerValueRange.
                facts.require_mixed_integer_value_range_declared(
                    &sidecar.structural_model.token_ranges,
                )?;
                let mut roles = SyntaxTokenRoles::from_layout(layout, sidecar)?;
                // Mixed fields need a structural choice token for every
                // alternative, including bare payload leaves. Homogeneous
                // multi-alt Nonterminal fields already receive tokens from
                // the layout helper; this only fills missing payload (and any
                // other) node kinds on mixed fields. Go-prefixed grammar IDs
                // must never free-slot invent here: that would bypass the
                // pinned-table guard in node_start_alternative_token_ids_from_sidecar.
                ensure_mixed_field_choice_tokens(&sidecar.input.grammar_id, &facts, &mut roles)?;
                Ok(Self {
                    facts,
                    roles,
                    menu_bindings: MenuSiteBindings::new(),
                })
            }
            SyntaxTokenLayoutMode::SharedAlphabet => {
                let shared = shared_alphabet_token_layout();
                // Integer-value-range gate uses the shared layout (Decision 1
                // payload bands), not the sidecar's per-grammar ranges.
                facts.require_mixed_integer_value_range_declared(&shared)?;
                require_layout_matches_shared_alphabet(layout)?;
                let roles = SyntaxTokenRoles::from_shared_alphabet_layout(layout, &facts)?;
                Ok(Self {
                    facts,
                    roles,
                    menu_bindings: MenuSiteBindings::new(),
                })
            }
        }
    }

    /// Attach menu-index bindings. Cardinality must fit the menu band (**P4**/**P9**).
    ///
    /// - **PerGrammar:** requires a dedicated `menu_index_payload` layout role.
    /// - **SharedAlphabet:** menus map onto the existing `u8_integer_payload`
    ///   band with a 256 ceiling and fail closed above it (**P9**). No shared
    ///   alphabet constants are revised; the dedicated menu band is not added.
    pub fn with_menu_bindings(
        mut self,
        bindings: MenuSiteBindings,
    ) -> Result<Self, SyntaxBackendBoundaryError> {
        if !bindings.is_empty() {
            if self.roles.menu_range.is_none() {
                if self.roles.layout_mode == SyntaxTokenLayoutMode::SharedAlphabet {
                    // P9: SharedAlphabet reuses u8_integer_payload as the menu
                    // ordinal band (single-token menu emission, 256 ceiling).
                    self.roles.menu_range = Some(self.roles.integer_range.clone());
                } else {
                    return Err(SyntaxBackendBoundaryError::new(
                        "syntax_state_machine.menu_bindings.layout",
                        "menu site bindings require a menu_index_payload layout role",
                    ));
                }
            }
            let Some(menu_range) = self.roles.menu_range.as_ref() else {
                return Err(SyntaxBackendBoundaryError::new(
                    "syntax_state_machine.menu_bindings.layout",
                    "menu band was not materialised after SharedAlphabet alias",
                ));
            };
            let width = menu_range
                .end_exclusive
                .checked_sub(menu_range.start)
                .ok_or_else(|| {
                    SyntaxBackendBoundaryError::new(
                        "syntax_state_machine.menu_bindings.layout",
                        "menu band range is inverted",
                    )
                })?;
            let max_digits = bindings.max_menu_digits;
            if max_digits == 0 {
                return Err(SyntaxBackendBoundaryError::new(
                    "syntax_state_machine.menu_bindings.digits",
                    "menu site bindings require max_menu_digits >= 1",
                ));
            }
            // Capacity = base^max_menu_digits over the live menu band width.
            // Default max_menu_digits=1 preserves the historical single-token
            // ceiling (limit > width fails closed).
            let capacity = (width as u64).checked_pow(max_digits).ok_or_else(|| {
                SyntaxBackendBoundaryError::new(
                    "syntax_state_machine.menu_bindings.capacity",
                    format!("menu capacity base^{max_digits} overflows u64 for band width {width}"),
                )
            })?;
            for ((owner_rule, owner_field), kind) in bindings.iter() {
                let limit = kind.band_limit();
                if limit == 0 {
                    return Err(SyntaxBackendBoundaryError::new(
                        "syntax_state_machine.menu_bindings.cardinality",
                        format!("menu site {owner_rule}.{owner_field} has zero band limit"),
                    ));
                }
                if (limit as u64) > capacity {
                    return Err(SyntaxBackendBoundaryError::new(
                        "syntax_state_machine.menu_bindings.overflow",
                        format!(
                            "menu site {owner_rule}.{owner_field} band limit {limit} exceeds menu capacity {capacity} (band width {width}, max_menu_digits {max_digits})"
                        ),
                    ));
                }
                // Factored sites do not use the radix code: each of the two
                // positional digits must fit the band on its own, so the flat
                // capacity check above is not sufficient. Fail closed at bind
                // time rather than emitting a digit the band cannot carry.
                if let MenuSiteKind::FactoredScopeColumns { columns_per_table } = kind {
                    if max_digits < 2 {
                        return Err(SyntaxBackendBoundaryError::new(
                            "syntax_state_machine.menu_bindings.factored_digits",
                            format!(
                                "factored column menu site {owner_rule}.{owner_field} needs max_menu_digits >= 2, got {max_digits}"
                            ),
                        ));
                    }
                    let table_count = columns_per_table.len() as u64;
                    if table_count > width as u64 {
                        return Err(SyntaxBackendBoundaryError::new(
                            "syntax_state_machine.menu_bindings.factored_overflow",
                            format!(
                                "factored column menu site {owner_rule}.{owner_field} addresses {table_count} tables, which exceeds the menu band width {width}"
                            ),
                        ));
                    }
                    let widest = columns_per_table.iter().copied().max().unwrap_or(0);
                    if widest as u64 > width as u64 {
                        return Err(SyntaxBackendBoundaryError::new(
                            "syntax_state_machine.menu_bindings.factored_overflow",
                            format!(
                                "factored column menu site {owner_rule}.{owner_field} has a table with {widest} columns, which exceeds the menu band width {width}"
                            ),
                        ));
                    }
                }
            }
        }
        self.menu_bindings = bindings;
        Ok(self)
    }

    pub fn token_roles(&self) -> &SyntaxTokenRoles {
        &self.roles
    }

    /// Menu site bindings attached to this model (empty when unbound).
    pub fn menu_bindings(&self) -> &MenuSiteBindings {
        &self.menu_bindings
    }

    /// Layout mode used when this model was compiled.
    pub fn layout_mode(&self) -> SyntaxTokenLayoutMode {
        self.roles.layout_mode
    }

    pub fn state_from_prefix(
        &self,
        raw_token_ids: &[u32],
    ) -> Result<RawSyntaxState, RawSyntaxError> {
        self.machine()
            .replay(raw_token_ids, CompletionMode::AllowPrefix)
            .map(|(_, state)| state)
    }

    pub fn decode_complete(&self, raw_token_ids: &[u32]) -> Result<RawSyntaxState, RawSyntaxError> {
        self.machine()
            .replay(raw_token_ids, CompletionMode::RequireComplete)
            .map(|(_, state)| state)
    }

    pub fn decode_complete_tree(
        &self,
        raw_token_ids: &[u32],
    ) -> Result<RawSyntaxDecodedTree, RawSyntaxError> {
        let (machine, state) = self
            .machine()
            .replay(raw_token_ids, CompletionMode::RequireComplete)?;
        Ok(RawSyntaxDecodedTree {
            state,
            events: machine.into_live().take_decoded_events(),
        })
    }

    /// Empty owned live decoder state (no event recording; O(1) step advance).
    ///
    /// Pair with [`Self::advance_live_state`], [`Self::admissible_live_state`],
    /// and [`Self::live_state_snapshot`]. Mutable state is owned separately from
    /// this immutable model so callers can store a long-lived cursor without
    /// self-referential borrows (and without replaying the whole prefix).
    pub fn begin_live_state(&self) -> RawSyntaxLiveState {
        RawSyntaxLiveState::without_event_recording()
    }

    /// Apply one raw token to owned live state in O(one transition).
    ///
    /// Error-atomic: on `Err` the live state is unchanged (same contract as
    /// [`RawSyntaxStream::advance`]).
    pub fn advance_live_state(
        &self,
        live: &mut RawSyntaxLiveState,
        raw_token_id: u32,
    ) -> Result<RawSyntaxAdvanceDelta, RawSyntaxError> {
        let mut machine = RawSyntaxStateMachine::from_live(
            &self.facts,
            &self.roles,
            &self.menu_bindings,
            std::mem::take(live),
        );
        let result = machine.advance_one(raw_token_id);
        *live = machine.into_live();
        result
    }

    /// Admissible next tokens for an owned live prefix (no re-walk of tokens).
    pub fn admissible_live_state(&self, live: &RawSyntaxLiveState) -> RawSyntaxAdmissibleTokens {
        let machine = RawSyntaxStateMachine::from_live(
            &self.facts,
            &self.roles,
            &self.menu_bindings,
            live.clone(),
        );
        machine.admissible_next_raw_tokens()
    }

    /// Full [`RawSyntaxState`] snapshot for owned live state.
    pub fn live_state_snapshot(&self, live: &RawSyntaxLiveState) -> RawSyntaxState {
        let machine = RawSyntaxStateMachine::from_live(
            &self.facts,
            &self.roles,
            &self.menu_bindings,
            live.clone(),
        );
        machine.prefix_state()
    }

    /// Menu-site cursor at the current free position, if a menu payload is open.
    ///
    /// `None` when no payload is open or the open payload is not a bound menu
    /// site (literals, integers, menu-less models). Empty bindings stay
    /// unreachable (**P10**).
    pub fn live_menu_cursor(&self, live: &RawSyntaxLiveState) -> Option<LiveMenuCursor> {
        let machine = RawSyntaxStateMachine::from_live(
            &self.facts,
            &self.roles,
            &self.menu_bindings,
            live.clone(),
        );
        machine.menu_cursor()
    }

    /// Whether owned live state is EOF-valid (complete root closed).
    pub fn live_state_eof_valid(&self, live: &RawSyntaxLiveState) -> bool {
        let machine = RawSyntaxStateMachine::from_live(
            &self.facts,
            &self.roles,
            &self.menu_bindings,
            live.clone(),
        );
        machine.eof_valid()
    }

    /// Encode a complete model-conforming decoded-node tree to an expanded raw
    /// token stream. Inverse of [`Self::decode_complete_tree`] →
    /// [`RawSyntaxDecodedTree::decoded_node_tree`].
    ///
    /// Model-faithful: emits exactly the stream the shared SM accepts for this
    /// compiled model. Non-conforming trees fail closed with a typed error
    /// naming rule / field / reason (never best-effort partial emission).
    /// See `docs/generic-codec-layer-contract.md`.
    pub fn encode_complete_tree(
        &self,
        root: &RawSyntaxDecodedNode,
    ) -> Result<Vec<u32>, RawSyntaxError> {
        encode::encode_complete_tree(&self.facts, &self.roles, &self.menu_bindings, root)
    }

    pub fn admissible_next_raw_tokens(
        &self,
        raw_token_ids: &[u32],
    ) -> Result<RawSyntaxAdmissibleTokens, RawSyntaxError> {
        Ok(self.state_from_prefix(raw_token_ids)?.admissible)
    }

    pub fn transition(
        &self,
        raw_prefix_token_ids: &[u32],
        candidate_raw_token_id: u32,
    ) -> RawSyntaxTransition {
        let mut token_ids = Vec::with_capacity(raw_prefix_token_ids.len() + 1);
        token_ids.extend_from_slice(raw_prefix_token_ids);
        token_ids.push(candidate_raw_token_id);

        match self.state_from_prefix(&token_ids) {
            Ok(state) => RawSyntaxTransition {
                candidate_raw_token_id,
                accepted: true,
                state: Some(state),
                error: None,
            },
            Err(error) => RawSyntaxTransition {
                candidate_raw_token_id,
                accepted: false,
                state: None,
                error: Some(error),
            },
        }
    }

    pub fn selected_completion_trace(
        &self,
        raw_token_ids: &[u32],
        sample_steps: &BTreeSet<usize>,
    ) -> Result<RawSyntaxSelectedCompletionTrace, RawSyntaxError> {
        self.machine()
            .selected_completion_trace(raw_token_ids, sample_steps)
    }

    /// Incremental handle positioned at the empty raw-token prefix.
    ///
    /// The handle is observationally equivalent to replaying the consumed
    /// tokens through `state_from_prefix`: both paths run the same engine,
    /// and the transition-acceptance property test compares them
    /// field-for-field over the checked corpora. The replay API remains the
    /// reference implementation.
    pub fn begin_stream(&self) -> RawSyntaxStream<'_> {
        RawSyntaxStream {
            machine: RawSyntaxStateMachine::without_event_recording(
                &self.facts,
                &self.roles,
                &self.menu_bindings,
            ),
        }
    }

    /// Incremental handle over the elided canonical stream contract,
    /// positioned past the initial mask-forced run (see
    /// `ElidedRawSyntaxStream`). Construction eagerly drains the initial
    /// forced run, so it fails when the drain hits the
    /// `forced_run_limit_exceeded` totality guard or the
    /// `expansion_stuck` engine-corruption guard.
    pub fn begin_elided_stream(&self) -> Result<ElidedRawSyntaxStream<'_>, RawSyntaxError> {
        ElidedRawSyntaxStream::from_stream(self.begin_stream())
    }

    /// Elides every mask-forced token from a complete expanded raw stream:
    /// the canonical elided stream keeps exactly the tokens whose
    /// pre-consumption admissible mask has cardinality >= 2 (the census
    /// walk). A token sitting on a singleton mask must equal the forced
    /// token (the stream would be invalid otherwise) and is dropped; the
    /// stream must reach an EOF-valid state. The expanded stream remains
    /// the state machine's native semantics — this is a total, pure,
    /// target-neutral transform on top of it, inverted exactly by
    /// `expand_elided_raw_tokens`.
    pub fn elide_forced_raw_tokens(&self, expanded: &[u32]) -> Result<Vec<u32>, RawSyntaxError> {
        let mut walker = self.begin_stream();
        let mut elided = Vec::with_capacity(expanded.len());
        for &token in expanded {
            let forced = singleton_admissible_raw_token(&walker.admissible_next_raw_tokens());
            // A singleton mask admits only the forced token, so `advance`
            // rejects any non-matching token with the engine's typed error.
            walker.advance(token)?;
            if forced != Some(token) {
                elided.push(token);
            }
        }
        if !walker.eof_valid() {
            return Err(premature_eof_error(&walker.state_snapshot()));
        }
        Ok(elided)
    }

    /// Expands a canonical elided raw stream back into the state machine's
    /// native expanded stream: auto-applies mask-forced tokens while the
    /// live mask is a singleton (eager forced-run drain, bounded by
    /// `ELIDED_FORCED_RUN_LIMIT`), consumes the next elided input token at
    /// the resulting free-choice position, drains the trailing forced run
    /// after the last input token, and requires EOF validity.
    ///
    /// Strictness: expansion accepts exactly the image of
    /// `elide_forced_raw_tokens`; an input that spells out a mask-forced
    /// token fails with the typed `forced_token_in_elided_stream` error
    /// (see `ElidedRawSyntaxStream::advance`).
    pub fn expand_elided_raw_tokens(&self, elided: &[u32]) -> Result<Vec<u32>, RawSyntaxError> {
        let mut walker = self.begin_elided_stream()?;
        let mut expanded = Vec::with_capacity(elided.len().saturating_mul(2));
        expanded.extend(
            walker
                .initial_forced_run()
                .iter()
                .map(|forced| forced.raw_token_id),
        );
        for &token in elided {
            let advance = walker.advance(token)?;
            expanded.push(token);
            expanded.extend(
                advance
                    .forced_tokens_applied
                    .iter()
                    .map(|forced| forced.raw_token_id),
            );
        }
        if !walker.eof_valid() {
            return Err(premature_eof_error(&walker.state_snapshot()));
        }
        Ok(expanded)
    }

    /// Strict decode of a complete canonical elided stream: expands via
    /// `expand_elided_raw_tokens`, then runs the expanded-path reference
    /// decoder, so the replay engine stays the single source of truth.
    pub fn strict_decode_complete_elided(
        &self,
        elided: &[u32],
    ) -> Result<RawSyntaxState, RawSyntaxError> {
        let expanded = self.expand_elided_raw_tokens(elided)?;
        self.decode_complete(&expanded)
    }

    /// Decoded-tree variant of `strict_decode_complete_elided`. Decoded
    /// events keep expanded-layer `raw_token_offset` semantics: the elided
    /// decode produces events over the expansion, which is the consistent
    /// choice for every expanded-layer consumer.
    pub fn decode_complete_tree_elided(
        &self,
        elided: &[u32],
    ) -> Result<RawSyntaxDecodedTree, RawSyntaxError> {
        let expanded = self.expand_elided_raw_tokens(elided)?;
        self.decode_complete_tree(&expanded)
    }

    fn machine(&self) -> RawSyntaxStateMachine<'_> {
        RawSyntaxStateMachine::new(&self.facts, &self.roles, &self.menu_bindings)
    }
}

/// Mirror of the replay engine's `premature_eof` rejection for the
/// elide/expand entry points, built from a prefix-state snapshot so the
/// typed context (consumed tokens, stack depth, active rule/field) matches
/// the reference path.
fn premature_eof_error(state: &RawSyntaxState) -> RawSyntaxError {
    let message = match state.active_payload_kind.as_deref() {
        Some(class) => {
            format!("EOF reached mid-payload (expected class {class}) before root closed")
        }
        None => "EOF reached before root closed".to_string(),
    };
    RawSyntaxError {
        kind: "premature_eof",
        message,
        consumed_raw_tokens: state.consumed_raw_tokens,
        failing_token: None,
        stack_depth: state.stack_depth,
        active_rule: state.active_rule.clone(),
        active_field: state.active_field.clone(),
        expected_payload_class: state.active_payload_kind.clone(),
    }
}

impl RawSyntaxStateMachineCore for CompiledSyntaxModel {
    fn token_roles(&self) -> &SyntaxTokenRoles {
        CompiledSyntaxModel::token_roles(self)
    }

    fn state_after_raw_prefix(
        &self,
        raw_token_ids: &[u32],
    ) -> Result<RawSyntaxState, RawSyntaxError> {
        CompiledSyntaxModel::state_from_prefix(self, raw_token_ids)
    }

    fn candidate_raw_token_transition(
        &self,
        raw_prefix_token_ids: &[u32],
        candidate_raw_token_id: u32,
    ) -> RawSyntaxTransition {
        CompiledSyntaxModel::transition(self, raw_prefix_token_ids, candidate_raw_token_id)
    }

    fn strict_decode_complete(
        &self,
        raw_token_ids: &[u32],
    ) -> Result<RawSyntaxState, RawSyntaxError> {
        CompiledSyntaxModel::decode_complete(self, raw_token_ids)
    }

    fn decode_complete_tree(
        &self,
        raw_token_ids: &[u32],
    ) -> Result<RawSyntaxDecodedTree, RawSyntaxError> {
        CompiledSyntaxModel::decode_complete_tree(self, raw_token_ids)
    }

    fn selected_completion_trace(
        &self,
        raw_token_ids: &[u32],
        sample_steps: &BTreeSet<usize>,
    ) -> Result<RawSyntaxSelectedCompletionTrace, RawSyntaxError> {
        CompiledSyntaxModel::selected_completion_trace(self, raw_token_ids, sample_steps)
    }

    fn begin_raw_stream(&self) -> Option<RawSyntaxStream<'_>> {
        Some(self.begin_stream())
    }
}

impl From<&GrammarStructuralTokenLayout> for ProjectionTokenLayoutSummary {
    fn from(layout: &GrammarStructuralTokenLayout) -> Self {
        Self {
            vocab_size: layout.vocab_size,
            pad_token: layout.pad_token,
            ranges: layout
                .ranges
                .iter()
                .map(|range| ProjectionTokenLayoutRangeSummary {
                    id: range.id.clone(),
                    role: range.role.clone(),
                    start: range.start,
                    end_exclusive: range.end_exclusive,
                })
                .collect(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RawSyntaxTransition {
    pub candidate_raw_token_id: u32,
    pub accepted: bool,
    pub state: Option<RawSyntaxState>,
    pub error: Option<RawSyntaxError>,
}

/// Live incremental state over a borrowed compiled model.
///
/// `Clone` is the supported fork primitive for exploring accepted branches:
/// the mutable state is depth-bounded (the checked record stream peaks at
/// stack depth 16), so a fork copies a short frame stack plus an optional
/// payload byte buffer. Decoded-event recording is suppressed; callers that
/// need decoded events use `decode_complete_tree`. `advance` is error-atomic:
/// a handle that returned `Err` is unchanged and remains usable, so rejected
/// probes need no fork — `try token -> Err -> state unchanged` is
/// backtrack-on-reject for free, and `admits` answers acceptance without
/// mutation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RawSyntaxStream<'model> {
    machine: RawSyntaxStateMachine<'model>,
}

/// Live incremental state over the elided canonical stream contract: a
/// wrapper over `RawSyntaxStream` that eagerly auto-applies mask-forced
/// tokens at construction and after every successful `advance`, so its
/// observable position is always a free choice. Consequently
/// `admissible_next_raw_tokens` on a usable handle is never a singleton —
/// it is exactly the mask a model-side sampler must see — and the
/// auto-applied tokens are returned in each advance result so re-expansion
/// can be recorded.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ElidedRawSyntaxStream<'model> {
    stream: RawSyntaxStream<'model>,
    consumed_elided_tokens: usize,
    initial_forced_run: Vec<ElidedForcedRawToken>,
    last_forced_run: Vec<ElidedForcedRawToken>,
}

/// One mask-forced raw token auto-applied by the eager forced-run drain,
/// with its position in the expanded stream.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct ElidedForcedRawToken {
    pub expanded_raw_token_offset: usize,
    pub raw_token_id: u32,
}

/// Result of `ElidedRawSyntaxStream::advance`: the expanded-layer delta for
/// the consumed elided input token plus the mask-forced run auto-applied
/// immediately after it (in expanded-stream order).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ElidedRawSyntaxAdvance {
    pub consumed: RawSyntaxAdvanceDelta,
    pub forced_tokens_applied: Vec<ElidedForcedRawToken>,
}

/// Per-token state delta returned by `RawSyntaxStream::advance`.
///
/// Deliberately allocation-light: no admissible mask and no `String`
/// context. Callers that need the full diagnostic state use
/// `RawSyntaxStream::state_snapshot`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct RawSyntaxAdvanceDelta {
    pub applied_raw_token_id: u32,
    pub consumed_raw_tokens: usize,
    pub event_kind: RawSyntaxDecodedEventKind,
    pub status: RawSyntaxStatus,
    pub eof_valid: bool,
    pub stack_depth: usize,
    pub payload_active: bool,
    pub payload_byte_count: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct RawSyntaxState {
    pub status: RawSyntaxStatus,
    pub consumed_raw_tokens: usize,
    pub total_raw_tokens: usize,
    pub eof_valid: bool,
    pub stack_depth: usize,
    pub active_rule: Option<String>,
    pub active_field: Option<String>,
    pub active_payload_kind: Option<String>,
    pub active_payload_node_kind: Option<String>,
    pub active_payload_byte_count: usize,
    pub admissible: RawSyntaxAdmissibleTokens,
    pub diagnostic: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct RawSyntaxDecodedTree {
    pub state: RawSyntaxState,
    pub events: Vec<RawSyntaxDecodedEvent>,
}

impl RawSyntaxDecodedTree {
    pub fn node_start_count(&self) -> usize {
        self.events
            .iter()
            .filter(|event| event.event_kind == RawSyntaxDecodedEventKind::NodeStart)
            .count()
    }

    pub fn field_start_count(&self) -> usize {
        self.events
            .iter()
            .filter(|event| event.event_kind == RawSyntaxDecodedEventKind::FieldStart)
            .count()
    }

    pub fn payload_event_count(&self) -> usize {
        self.events
            .iter()
            .filter(|event| {
                matches!(
                    event.event_kind,
                    RawSyntaxDecodedEventKind::TerminalStart
                        | RawSyntaxDecodedEventKind::PayloadToken
                        | RawSyntaxDecodedEventKind::PayloadEnd
                        | RawSyntaxDecodedEventKind::BooleanPayload
                )
            })
            .count()
    }

    pub fn payload_token_byte_count(&self) -> usize {
        self.events
            .iter()
            .filter(|event| event.payload_token_byte.is_some())
            .count()
    }

    pub fn payload_value_count(&self) -> usize {
        self.events
            .iter()
            .filter(|event| event.payload_value.is_some())
            .count()
    }

    pub fn root_rule(&self) -> Option<&str> {
        self.events
            .iter()
            .find(|event| event.event_kind == RawSyntaxDecodedEventKind::NodeStart)
            .and_then(|event| event.rule.as_deref())
    }

    pub fn root_node_kind(&self) -> Option<&str> {
        self.events
            .iter()
            .find(|event| event.event_kind == RawSyntaxDecodedEventKind::NodeStart)
            .and_then(|event| event.node_kind.as_deref())
    }

    pub fn decoded_node_tree(&self) -> Result<RawSyntaxDecodedNode, SyntaxBackendBoundaryError> {
        let mut builder = RawSyntaxDecodedNodeTreeBuilder::default();
        for event in &self.events {
            builder.apply(event)?;
        }
        builder.finish()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct RawSyntaxDecodedEvent {
    pub raw_token_offset: usize,
    pub raw_token_id: u32,
    pub event_kind: RawSyntaxDecodedEventKind,
    pub rule: Option<String>,
    pub field: Option<String>,
    pub node_kind: Option<String>,
    pub payload_kind: Option<String>,
    pub payload_byte_count: usize,
    pub payload_token_byte: Option<u8>,
    pub payload_value: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum RawSyntaxDecodedEventKind {
    NodeStart,
    NodeEnd,
    FieldStart,
    FieldEnd,
    FieldMissing,
    RepeatedEmpty,
    TerminalStart,
    PayloadToken,
    PayloadEnd,
    BooleanPayload,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct RawSyntaxDecodedNode {
    pub rule_name: String,
    pub node_kind: String,
    pub fields: Vec<RawSyntaxDecodedField>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct RawSyntaxDecodedField {
    pub name: String,
    pub values: Vec<RawSyntaxDecodedValue>,
    pub missing: bool,
    pub repeated_empty: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub enum RawSyntaxDecodedValue {
    Node(Box<RawSyntaxDecodedNode>),
    Terminal(RawSyntaxDecodedTerminal),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct RawSyntaxDecodedTerminal {
    pub node_kind: String,
    pub terminal: String,
    pub payload: String,
}

impl RawSyntaxDecodedNode {
    pub fn field(&self, name: &str) -> Option<&RawSyntaxDecodedField> {
        self.fields.iter().find(|field| field.name == name)
    }

    pub fn node_kind_count(&self, node_kind: &str) -> usize {
        usize::from(self.node_kind == node_kind)
            + self
                .fields
                .iter()
                .flat_map(|field| field.values.iter())
                .filter_map(|value| match value {
                    RawSyntaxDecodedValue::Node(node) => Some(node.node_kind_count(node_kind)),
                    RawSyntaxDecodedValue::Terminal(_) => None,
                })
                .sum::<usize>()
    }
}

#[derive(Debug, Default, Clone, PartialEq, Eq)]
struct RawSyntaxDecodedNodeTreeBuilder {
    stack: Vec<RawSyntaxDecodedNodeFrame>,
    root: Option<RawSyntaxDecodedNode>,
    pending_terminal: Option<RawSyntaxDecodedTerminalStart>,
}

impl RawSyntaxDecodedNodeTreeBuilder {
    fn apply(&mut self, event: &RawSyntaxDecodedEvent) -> Result<(), SyntaxBackendBoundaryError> {
        match event.event_kind {
            RawSyntaxDecodedEventKind::NodeStart => self.apply_node_start(event),
            RawSyntaxDecodedEventKind::NodeEnd => self.apply_node_end(event),
            RawSyntaxDecodedEventKind::FieldStart => self.apply_field_start(event),
            RawSyntaxDecodedEventKind::FieldEnd => self.apply_field_end(event),
            RawSyntaxDecodedEventKind::FieldMissing => self.apply_field_missing(event),
            RawSyntaxDecodedEventKind::RepeatedEmpty => self.apply_repeated_empty(event),
            RawSyntaxDecodedEventKind::TerminalStart => self.apply_terminal_start(event),
            RawSyntaxDecodedEventKind::PayloadToken => self.apply_payload_token(event),
            RawSyntaxDecodedEventKind::PayloadEnd | RawSyntaxDecodedEventKind::BooleanPayload => {
                self.apply_payload_complete(event)
            }
        }
    }

    fn finish(self) -> Result<RawSyntaxDecodedNode, SyntaxBackendBoundaryError> {
        if self.pending_terminal.is_some() {
            return Err(decoded_tree_error(
                "terminal",
                "decoded raw syntax event stream ended with an unfinished terminal payload",
            ));
        }
        if !self.stack.is_empty() {
            return Err(decoded_tree_error(
                "stack",
                "decoded raw syntax event stream ended with open nodes",
            ));
        }
        self.root.ok_or_else(|| {
            decoded_tree_error(
                "root",
                "decoded raw syntax event stream did not produce a root node",
            )
        })
    }

    fn apply_node_start(
        &mut self,
        event: &RawSyntaxDecodedEvent,
    ) -> Result<(), SyntaxBackendBoundaryError> {
        self.require_no_pending_terminal(event, "node_start")?;
        if self.root.is_some() && self.stack.is_empty() {
            return Err(decoded_tree_event_error(
                event,
                "node_start",
                "node_start appeared after the root node was already closed",
            ));
        }
        if let Some(parent) = self.stack.last() {
            let Some(open_field) = parent.open_field.as_ref() else {
                return Err(decoded_tree_event_error(
                    event,
                    "node_start",
                    "child node_start appeared outside an open field",
                ));
            };
            if event.field.as_deref() != Some(open_field.name.as_str()) {
                return Err(decoded_tree_event_error(
                    event,
                    "node_start",
                    format!(
                        "child node_start field {:?} does not match open field {}",
                        event.field, open_field.name
                    ),
                ));
            }
        } else if event.field.is_some() {
            return Err(decoded_tree_event_error(
                event,
                "node_start",
                "root node_start must not name a parent field",
            ));
        }

        let rule_name = required_event_value(event, "node_start.rule", event.rule.as_deref())?;
        let node_kind =
            required_event_value(event, "node_start.node_kind", event.node_kind.as_deref())?;
        self.stack.push(RawSyntaxDecodedNodeFrame {
            node: RawSyntaxDecodedNode {
                rule_name: rule_name.to_string(),
                node_kind: node_kind.to_string(),
                fields: Vec::new(),
            },
            open_field: None,
        });
        Ok(())
    }

    fn apply_node_end(
        &mut self,
        event: &RawSyntaxDecodedEvent,
    ) -> Result<(), SyntaxBackendBoundaryError> {
        self.require_no_pending_terminal(event, "node_end")?;
        let frame = self.stack.pop().ok_or_else(|| {
            decoded_tree_event_error(event, "node_end", "node_end appeared with no open node")
        })?;
        if frame.open_field.is_some() {
            return Err(decoded_tree_event_error(
                event,
                "node_end",
                "node_end appeared while a field was still open",
            ));
        }
        if event.rule.as_deref() != Some(frame.node.rule_name.as_str()) {
            return Err(decoded_tree_event_error(
                event,
                "node_end",
                format!(
                    "node_end rule {:?} does not match open node {}",
                    event.rule, frame.node.rule_name
                ),
            ));
        }
        if event.node_kind.as_deref() != Some(frame.node.node_kind.as_str()) {
            return Err(decoded_tree_event_error(
                event,
                "node_end",
                format!(
                    "node_end node kind {:?} does not match open node {}",
                    event.node_kind, frame.node.node_kind
                ),
            ));
        }

        if let Some(parent) = self.stack.last_mut() {
            let Some(open_field) = parent.open_field.as_mut() else {
                return Err(decoded_tree_event_error(
                    event,
                    "node_end",
                    "completed child node had no parent open field",
                ));
            };
            open_field
                .values
                .push(RawSyntaxDecodedValue::Node(Box::new(frame.node)));
        } else if self.root.replace(frame.node).is_some() {
            return Err(decoded_tree_event_error(
                event,
                "node_end",
                "decoded event stream produced more than one root node",
            ));
        }
        Ok(())
    }

    fn apply_field_start(
        &mut self,
        event: &RawSyntaxDecodedEvent,
    ) -> Result<(), SyntaxBackendBoundaryError> {
        self.require_no_pending_terminal(event, "field_start")?;
        let frame = self.stack.last_mut().ok_or_else(|| {
            decoded_tree_event_error(
                event,
                "field_start",
                "field_start appeared with no open node",
            )
        })?;
        if frame.open_field.is_some() {
            return Err(decoded_tree_event_error(
                event,
                "field_start",
                "field_start appeared while another field was open",
            ));
        }
        if event.rule.as_deref() != Some(frame.node.rule_name.as_str()) {
            return Err(decoded_tree_event_error(
                event,
                "field_start",
                format!(
                    "field_start rule {:?} does not match open node {}",
                    event.rule, frame.node.rule_name
                ),
            ));
        }
        let field_name = required_event_value(event, "field_start.field", event.field.as_deref())?;
        frame.open_field = Some(RawSyntaxDecodedField {
            name: field_name.to_string(),
            values: Vec::new(),
            missing: false,
            repeated_empty: false,
        });
        Ok(())
    }

    fn apply_field_end(
        &mut self,
        event: &RawSyntaxDecodedEvent,
    ) -> Result<(), SyntaxBackendBoundaryError> {
        self.require_no_pending_terminal(event, "field_end")?;
        let frame = self.stack.last_mut().ok_or_else(|| {
            decoded_tree_event_error(event, "field_end", "field_end appeared with no open node")
        })?;
        if event.rule.as_deref() != Some(frame.node.rule_name.as_str()) {
            return Err(decoded_tree_event_error(
                event,
                "field_end",
                format!(
                    "field_end rule {:?} does not match open node {}",
                    event.rule, frame.node.rule_name
                ),
            ));
        }
        let field = frame.open_field.take().ok_or_else(|| {
            decoded_tree_event_error(event, "field_end", "field_end appeared with no open field")
        })?;
        if event.field.as_deref() != Some(field.name.as_str()) {
            return Err(decoded_tree_event_error(
                event,
                "field_end",
                format!(
                    "field_end field {:?} does not match open field {}",
                    event.field, field.name
                ),
            ));
        }
        frame.node.fields.push(field);
        Ok(())
    }

    fn apply_field_missing(
        &mut self,
        event: &RawSyntaxDecodedEvent,
    ) -> Result<(), SyntaxBackendBoundaryError> {
        let field = self.open_field_mut(event, "field_missing")?;
        if !field.values.is_empty() || field.repeated_empty {
            return Err(decoded_tree_event_error(
                event,
                "field_missing",
                "field_missing cannot appear after values or repeated_empty",
            ));
        }
        field.missing = true;
        Ok(())
    }

    fn apply_repeated_empty(
        &mut self,
        event: &RawSyntaxDecodedEvent,
    ) -> Result<(), SyntaxBackendBoundaryError> {
        let field = self.open_field_mut(event, "repeated_empty")?;
        if !field.values.is_empty() || field.missing {
            return Err(decoded_tree_event_error(
                event,
                "repeated_empty",
                "repeated_empty cannot appear after values or field_missing",
            ));
        }
        field.repeated_empty = true;
        Ok(())
    }

    fn apply_terminal_start(
        &mut self,
        event: &RawSyntaxDecodedEvent,
    ) -> Result<(), SyntaxBackendBoundaryError> {
        self.require_no_pending_terminal(event, "terminal_start")?;
        let field = self.open_field_mut(event, "terminal_start")?;
        if field.missing || field.repeated_empty {
            return Err(decoded_tree_event_error(
                event,
                "terminal_start",
                "terminal_start cannot appear after field_missing or repeated_empty",
            ));
        }
        let node_kind = required_event_value(
            event,
            "terminal_start.node_kind",
            event.node_kind.as_deref(),
        )?;
        let terminal = required_event_value(
            event,
            "terminal_start.payload_kind",
            event.payload_kind.as_deref(),
        )?;
        self.pending_terminal = Some(RawSyntaxDecodedTerminalStart {
            node_kind: node_kind.to_string(),
            terminal: terminal.to_string(),
        });
        Ok(())
    }

    fn apply_payload_token(
        &mut self,
        event: &RawSyntaxDecodedEvent,
    ) -> Result<(), SyntaxBackendBoundaryError> {
        if self.pending_terminal.is_none() {
            return Err(decoded_tree_event_error(
                event,
                "payload_token",
                "payload_token appeared without a terminal_start",
            ));
        }
        Ok(())
    }

    fn apply_payload_complete(
        &mut self,
        event: &RawSyntaxDecodedEvent,
    ) -> Result<(), SyntaxBackendBoundaryError> {
        let terminal_start = self.pending_terminal.take().ok_or_else(|| {
            decoded_tree_event_error(
                event,
                "payload_complete",
                "payload completion appeared without a terminal_start",
            )
        })?;
        let node_kind = event
            .node_kind
            .as_deref()
            .unwrap_or(terminal_start.node_kind.as_str())
            .to_string();
        let terminal = event
            .payload_kind
            .as_deref()
            .unwrap_or(terminal_start.terminal.as_str())
            .to_string();
        let payload = event.payload_value.clone().ok_or_else(|| {
            decoded_tree_event_error(
                event,
                "payload_complete",
                "payload completion event must materialize a payload value",
            )
        })?;
        let field = self.open_field_mut(event, "payload_complete")?;
        field
            .values
            .push(RawSyntaxDecodedValue::Terminal(RawSyntaxDecodedTerminal {
                node_kind,
                terminal,
                payload,
            }));
        Ok(())
    }

    fn open_field_mut(
        &mut self,
        event: &RawSyntaxDecodedEvent,
        path: &'static str,
    ) -> Result<&mut RawSyntaxDecodedField, SyntaxBackendBoundaryError> {
        let frame = self.stack.last_mut().ok_or_else(|| {
            decoded_tree_event_error(event, path, "event appeared with no open node")
        })?;
        if event.rule.as_deref() != Some(frame.node.rule_name.as_str()) {
            return Err(decoded_tree_event_error(
                event,
                path,
                format!(
                    "event rule {:?} does not match open node {}",
                    event.rule, frame.node.rule_name
                ),
            ));
        }
        let field = frame.open_field.as_mut().ok_or_else(|| {
            decoded_tree_event_error(event, path, "event appeared with no open field")
        })?;
        if event.field.as_deref() != Some(field.name.as_str()) {
            return Err(decoded_tree_event_error(
                event,
                path,
                format!(
                    "event field {:?} does not match open field {}",
                    event.field, field.name
                ),
            ));
        }
        Ok(field)
    }

    fn require_no_pending_terminal(
        &self,
        event: &RawSyntaxDecodedEvent,
        path: &'static str,
    ) -> Result<(), SyntaxBackendBoundaryError> {
        if self.pending_terminal.is_some() {
            return Err(decoded_tree_event_error(
                event,
                path,
                "structural event appeared before active terminal payload completed",
            ));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct RawSyntaxDecodedNodeFrame {
    node: RawSyntaxDecodedNode,
    open_field: Option<RawSyntaxDecodedField>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct RawSyntaxDecodedTerminalStart {
    node_kind: String,
    terminal: String,
}

fn required_event_value<'a>(
    event: &RawSyntaxDecodedEvent,
    path: &'static str,
    value: Option<&'a str>,
) -> Result<&'a str, SyntaxBackendBoundaryError> {
    value.ok_or_else(|| decoded_tree_event_error(event, path, "decoded event is missing metadata"))
}

fn decoded_tree_event_error(
    event: &RawSyntaxDecodedEvent,
    path: &'static str,
    message: impl Into<String>,
) -> SyntaxBackendBoundaryError {
    SyntaxBackendBoundaryError::new(
        "syntax_state_machine.decoded_node_tree",
        format!(
            "{path}: {} at raw token offset {}",
            message.into(),
            event.raw_token_offset
        ),
    )
}

fn decoded_tree_error(
    path: &'static str,
    message: impl Into<String>,
) -> SyntaxBackendBoundaryError {
    SyntaxBackendBoundaryError::new(
        "syntax_state_machine.decoded_node_tree",
        format!("{path}: {}", message.into()),
    )
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum RawSyntaxStatus {
    Partial,
    Complete,
}

impl RawSyntaxStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Partial => "partial",
            Self::Complete => "complete",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct RawSyntaxAdmissibleTokens {
    pub finite_raw_token_ids: Vec<u32>,
    pub raw_token_ranges: Vec<ProjectionTokenLayoutRangeSummary>,
    pub payload_end_admissible: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RawSyntaxSelectedCompletionTrace {
    pub transition_count: usize,
    pub admissible_mask_query_count: usize,
    pub finite_admissible_raw_token_mask_available: bool,
    pub every_selected_raw_token_admitted: bool,
    pub selected_transition_accepted_count: usize,
    pub final_state: RawSyntaxState,
    pub step_samples: Vec<RawSyntaxSelectedCompletionStep>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RawSyntaxSelectedCompletionStep {
    pub step_index: usize,
    pub prefix_raw_token_count: usize,
    pub selected_raw_token_id: u32,
    pub finite_admissible_raw_token_count: usize,
    pub selected_raw_token_admitted: bool,
    pub selected_transition_accepted: bool,
    pub transition_state: RawSyntaxState,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RawSyntaxError {
    pub kind: &'static str,
    pub message: String,
    pub consumed_raw_tokens: usize,
    pub failing_token: Option<u32>,
    pub stack_depth: usize,
    pub active_rule: Option<String>,
    pub active_field: Option<String>,
    /// When the failure is inside (or immediately after opening) a payload
    /// substate, the expected payload class (StructuralModel `payload_kind`,
    /// e.g. `"integer"` / `"string"`). None when no payload body is open.
    /// Contract: strict mid-payload EOF names the expected class.
    pub expected_payload_class: Option<String>,
}

/// One ordered alternative under a multi-alt / mixed field (I5 Decision 2).
///
/// Under `SharedAlphabet`, `token_id` is `CHOICE(n, k)` for this field's arity
/// `n` and domain-stable ordinal `k`. PerGrammar leaves
/// [`SyntaxTokenRoles::field_choice_table`] empty and keeps the historical
/// global `node_start_alternative_tokens` map.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FieldChoiceAlternative {
    pub node_kind: String,
    pub terminal_class: String,
    pub token_id: u32,
}

/// Field key for field-relative choice tables: `(owner_rule, owner_field)`.
pub type FieldChoiceKey = (String, String);

/// Result of `shared_alphabet_choice_assignments`.
///
/// - `.0` — residual global first-wins `node_kind → token` map (assertion /
///   residual surface only; not a sound reverse identity under SharedAlphabet).
/// - `.1` — load-bearing field-relative table keyed by [`FieldChoiceKey`], used
///   for encode/decode and mask population under SharedAlphabet.
pub type SharedAlphabetChoiceAssignments = (
    BTreeMap<String, u32>,
    BTreeMap<FieldChoiceKey, Vec<FieldChoiceAlternative>>,
);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SyntaxTokenRoles {
    pub vocab_size: u32,
    pub pad_token: u32,
    pub structural_range: ProjectionTokenLayoutRangeSummary,
    pub utf8_range: ProjectionTokenLayoutRangeSummary,
    pub integer_range: ProjectionTokenLayoutRangeSummary,
    /// Dedicated menu-index band when the layout declares `menu_index_payload`
    /// (**P4**). Absent for Go/C#/SharedAlphabet default layouts.
    pub menu_range: Option<ProjectionTokenLayoutRangeSummary>,
    pub node_start: u32,
    pub node_end: u32,
    pub field_start: u32,
    pub field_end: u32,
    pub field_missing: u32,
    pub repeated_empty: u32,
    pub terminal: u32,
    pub payload_end: u32,
    pub boolean_false: u32,
    pub boolean_true: u32,
    pub node_start_alternative_tokens: BTreeMap<String, u32>,
    /// Field-relative choice table for SharedAlphabet encode/decode
    /// (`I-DECODE-SHARED` / Decision 2–3). Empty under PerGrammar.
    ///
    /// Keyed by `(owner_rule, owner_field)`; each row is domain-stable ordered
    /// alternatives with `token_id == CHOICE(n, k)` for ordinal `k`.
    pub field_choice_table: BTreeMap<FieldChoiceKey, Vec<FieldChoiceAlternative>>,
    /// Layout mode that produced these roles (I5 Decision 4).
    ///
    /// Under `SharedAlphabet`, the same absolute `CHOICE(n,k)` token may be
    /// assigned to different node kinds on different fields; reverse-map
    /// uniqueness is therefore not enforced (I-DECODE-SHARED is field-relative).
    pub layout_mode: SyntaxTokenLayoutMode,
}

impl SyntaxTokenRoles {
    pub fn from_layout(
        layout: &ProjectionTokenLayoutSummary,
        sidecar: &GrammarStructuralModelSidecar,
    ) -> Result<Self, SyntaxBackendBoundaryError> {
        let structural_range = layout_range_by_role(layout, "structural_decisions")?;
        let utf8_range = layout_range_by_role(layout, "utf8_byte_payload")?;
        let integer_range =
            layout_range_by_role_any(layout, &["integer_lexeme_payload", "u8_integer_payload"])?;
        // Optional dedicated menu band (**P4**); absent layouts leave it None.
        let menu_range = optional_layout_range_by_role(layout, "menu_index_payload");
        let node_start_alternative_tokens = node_start_alternative_token_ids_from_sidecar(sidecar)?;
        let roles = Self {
            vocab_size: layout.vocab_size,
            pad_token: layout.pad_token,
            node_start: structural_range.start + RAW_NODE_START_OFFSET,
            node_end: structural_range.start + RAW_NODE_END_OFFSET,
            field_start: structural_range.start + RAW_FIELD_START_OFFSET,
            field_end: structural_range.start + RAW_FIELD_END_OFFSET,
            field_missing: structural_range.start + RAW_FIELD_MISSING_OFFSET,
            repeated_empty: structural_range.start + RAW_REPEATED_EMPTY_OFFSET,
            terminal: structural_range.start + RAW_TERMINAL_OFFSET,
            payload_end: structural_range.start + RAW_PAYLOAD_END_OFFSET,
            boolean_false: structural_range.start + RAW_BOOLEAN_FALSE_OFFSET,
            boolean_true: structural_range.start + RAW_BOOLEAN_TRUE_OFFSET,
            node_start_alternative_tokens,
            // PerGrammar keeps the historical global map only; field-relative
            // tables are SharedAlphabet (Decision 2 / I-DECODE-SHARED).
            field_choice_table: BTreeMap::new(),
            structural_range,
            utf8_range,
            integer_range,
            menu_range,
            layout_mode: SyntaxTokenLayoutMode::PerGrammar,
        };
        roles.validate()?;
        Ok(roles)
    }

    /// Build roles from the shared decision-token alphabet layout.
    ///
    /// Multi-alt and mixed-payload fields receive `CHOICE(n,k)` tokens by
    /// domain-stable ordinal (Decision 2). Single-alt and root opens keep the
    /// fixed `node_start` bracket role. Encode/decode use
    /// [`Self::field_choice_table`] (field-relative); the global
    /// `node_start_alternative_tokens` map remains first-wins for residual
    /// consumers and pin-style assertions.
    fn from_shared_alphabet_layout(
        layout: &ProjectionTokenLayoutSummary,
        facts: &ModelFacts,
    ) -> Result<Self, SyntaxBackendBoundaryError> {
        require_layout_matches_shared_alphabet(layout)?;
        let structural_range = layout_range_by_role(layout, "structural_decisions")?;
        let utf8_range = layout_range_by_role(layout, "utf8_byte_payload")?;
        let integer_range =
            layout_range_by_role_any(layout, &["integer_lexeme_payload", "u8_integer_payload"])?;
        // SharedAlphabet maps menus onto u8_integer_payload later (P9); no dedicated band.
        let menu_range = optional_layout_range_by_role(layout, "menu_index_payload");
        let (node_start_alternative_tokens, field_choice_table) =
            shared_alphabet_choice_assignments(facts, structural_range.start)?;
        let roles = Self {
            vocab_size: layout.vocab_size,
            pad_token: layout.pad_token,
            node_start: structural_range.start + RAW_NODE_START_OFFSET,
            node_end: structural_range.start + RAW_NODE_END_OFFSET,
            field_start: structural_range.start + RAW_FIELD_START_OFFSET,
            field_end: structural_range.start + RAW_FIELD_END_OFFSET,
            field_missing: structural_range.start + RAW_FIELD_MISSING_OFFSET,
            repeated_empty: structural_range.start + RAW_REPEATED_EMPTY_OFFSET,
            terminal: structural_range.start + RAW_TERMINAL_OFFSET,
            payload_end: structural_range.start + RAW_PAYLOAD_END_OFFSET,
            boolean_false: structural_range.start + RAW_BOOLEAN_FALSE_OFFSET,
            boolean_true: structural_range.start + RAW_BOOLEAN_TRUE_OFFSET,
            node_start_alternative_tokens,
            field_choice_table,
            structural_range,
            utf8_range,
            integer_range,
            menu_range,
            layout_mode: SyntaxTokenLayoutMode::SharedAlphabet,
        };
        roles.validate()?;
        Ok(roles)
    }

    /// Encode-side choice token for multi-alt / mixed field alternative.
    ///
    /// SharedAlphabet resolves via the field-keyed table (Decision 2);
    /// PerGrammar uses the historical global `node_kind → token` map.
    pub fn alternative_token_for_field(
        &self,
        owner_rule: &str,
        owner_field: &str,
        node_kind: &str,
    ) -> Option<u32> {
        if self.layout_mode == SyntaxTokenLayoutMode::SharedAlphabet {
            let key = (owner_rule.to_string(), owner_field.to_string());
            return self
                .field_choice_table
                .get(&key)
                .and_then(|row| row.iter().find(|alt| alt.node_kind == node_kind))
                .map(|alt| alt.token_id);
        }
        self.node_start_alternative_tokens.get(node_kind).copied()
    }

    /// Whether `token` is some field's assigned alternative/choice open token.
    pub fn is_alternative_choice_token(&self, token: u32) -> bool {
        if self.layout_mode == SyntaxTokenLayoutMode::SharedAlphabet {
            return self
                .field_choice_table
                .values()
                .any(|row| row.iter().any(|alt| alt.token_id == token));
        }
        self.node_start_alternative_tokens
            .values()
            .any(|alternative_token| *alternative_token == token)
    }

    /// Ordered choice row for a field, when present.
    pub fn field_choice_row(
        &self,
        owner_rule: &str,
        owner_field: &str,
    ) -> Option<&[FieldChoiceAlternative]> {
        self.field_choice_table
            .get(&(owner_rule.to_string(), owner_field.to_string()))
            .map(Vec::as_slice)
    }

    /// I-DECODE-SHARED reverse: open field F + token → alternative node_kind.
    ///
    /// SharedAlphabet: unpack `CHOICE(n',k)`; require field arity `n' == n` and
    /// `0 <= k < n`; return `A_F[k].node_kind`. Arity mismatch / unknown
    /// ordinal yield `None` (callers emit contextual errors).
    ///
    /// PerGrammar: historical global reverse map (first match).
    pub fn node_kind_for_field_choice_token(
        &self,
        owner_rule: &str,
        owner_field: &str,
        token: u32,
    ) -> Option<&str> {
        if self.layout_mode == SyntaxTokenLayoutMode::SharedAlphabet {
            let row = self.field_choice_row(owner_rule, owner_field)?;
            let n = row.len() as u32;
            if n < 2 {
                return None;
            }
            let offset = token.checked_sub(self.structural_range.start)?;
            let (arity, ordinal) = shared_alphabet_unpack_choice_offset(offset).ok()?;
            if arity != n {
                return None;
            }
            let alt = row.get(ordinal as usize)?;
            if alt.token_id != token {
                return None;
            }
            return Some(alt.node_kind.as_str());
        }
        self.node_start_alternative_tokens
            .iter()
            .find(|(_, alternative_token)| **alternative_token == token)
            .map(|(node_kind, _)| node_kind.as_str())
    }

    /// SharedAlphabet-only diagnostics: unpack `(arity, ordinal)` when `token`
    /// lies in the choice band; `None` for non-choice structural ids.
    pub fn unpack_shared_choice_token(&self, token: u32) -> Option<(u32, u32)> {
        if self.layout_mode != SyntaxTokenLayoutMode::SharedAlphabet {
            return None;
        }
        let offset = token.checked_sub(self.structural_range.start)?;
        shared_alphabet_unpack_choice_offset(offset).ok()
    }

    fn validate(&self) -> Result<(), SyntaxBackendBoundaryError> {
        for range in [
            &self.structural_range,
            &self.utf8_range,
            &self.integer_range,
        ] {
            if range.start >= range.end_exclusive {
                return Err(SyntaxBackendBoundaryError::new(
                    "syntax_state_machine.token_layout.range",
                    format!(
                        "token range {} is empty or inverted ({}..{})",
                        range.role, range.start, range.end_exclusive
                    ),
                ));
            }
            if range.end_exclusive > self.vocab_size {
                return Err(SyntaxBackendBoundaryError::new(
                    "syntax_state_machine.token_layout.range",
                    format!(
                        "token range {} ends at {} beyond vocab size {}",
                        range.role, range.end_exclusive, self.vocab_size
                    ),
                ));
            }
            if self.pad_token >= range.start && self.pad_token < range.end_exclusive {
                return Err(SyntaxBackendBoundaryError::new(
                    "syntax_state_machine.token_layout.pad_token",
                    format!(
                        "pad token {} falls inside token range {} ({}..{}); the replay \
                         engine rejects pad before payload dispatch, so a pad inside a \
                         payload range would diverge from the admissible mask",
                        self.pad_token, range.role, range.start, range.end_exclusive
                    ),
                ));
            }
        }
        if self.pad_token >= self.vocab_size {
            return Err(SyntaxBackendBoundaryError::new(
                "syntax_state_machine.token_layout.pad_token",
                format!(
                    "pad token {} is not below vocab size {}",
                    self.pad_token, self.vocab_size
                ),
            ));
        }
        for token in [
            self.node_start,
            self.node_end,
            self.field_start,
            self.field_end,
            self.field_missing,
            self.repeated_empty,
            self.terminal,
            self.payload_end,
            self.boolean_false,
            self.boolean_true,
        ] {
            if token < self.structural_range.start || token >= self.structural_range.end_exclusive {
                return Err(SyntaxBackendBoundaryError::new(
                    "syntax_state_machine.token_layout.structural_roles",
                    "structural event token offset falls outside structural token range",
                ));
            }
        }
        let first_alternative_token =
            self.structural_range.start + RAW_TOKEN_ALTERNATIVE_NODE_START_OFFSET;
        // PerGrammar: values must be unique because reverse identity is the
        // first global map match. SharedAlphabet intentionally reuses CHOICE(n,k)
        // across fields/node kinds; uniqueness is not required (Decision 3) and
        // reverse identity is field-relative via field_choice_table.
        let enforce_unique_choice_tokens = self.layout_mode == SyntaxTokenLayoutMode::PerGrammar;
        let mut token_owners: BTreeMap<u32, &str> = BTreeMap::new();
        for (node_kind, token) in &self.node_start_alternative_tokens {
            if *token < self.structural_range.start || *token >= self.structural_range.end_exclusive
            {
                return Err(SyntaxBackendBoundaryError::new(
                    "syntax_state_machine.token_layout.alternative_node_start_roles",
                    "alternative node_start token falls outside structural token range",
                ));
            }
            if *token < first_alternative_token {
                return Err(SyntaxBackendBoundaryError::new(
                    "syntax_state_machine.token_layout.alternative_node_start_roles",
                    format!(
                        "alternative node_start token {token} collides with the fixed \
                         structural role offsets below {first_alternative_token}"
                    ),
                ));
            }
            if enforce_unique_choice_tokens
                && let Some(existing) = token_owners.insert(*token, node_kind.as_str())
            {
                return Err(SyntaxBackendBoundaryError::new(
                    "syntax_state_machine.token_layout.alternative_node_start_unique",
                    format!(
                        "alternative node_start token {token} is assigned to both \
                         node_kind {existing:?} and node_kind {:?}; choice-token \
                         values must be unique across node kinds",
                        node_kind
                    ),
                ));
            }
        }
        Ok(())
    }
}

/// Assign shared-alphabet `CHOICE(n,k)` tokens for multi-alt and mixed fields.
///
/// Order key (Decision 2): sort each field's alternatives by
/// `(node_kind, terminal_class_name)` ascending. Map ordinal `k` to
/// `CHOICE(n, k)`. Arity above [`SHARED_ALPHABET_MAX_BRANCH_ARITY`] hard-errors.
///
/// Returns [`SharedAlphabetChoiceAssignments`]: residual global first-wins map
/// plus the load-bearing field-relative table.
fn shared_alphabet_choice_assignments(
    facts: &ModelFacts,
    structural_range_start: u32,
) -> Result<SharedAlphabetChoiceAssignments, SyntaxBackendBoundaryError> {
    let mut choice_fields: BTreeSet<(String, String)> = facts
        .multi_alternative_nonterminal_fields
        .iter()
        .cloned()
        .collect();
    choice_fields.extend(facts.mixed_payload_fields.iter().cloned());

    let mut global_first_wins: BTreeMap<String, u32> = BTreeMap::new();
    let mut field_choice_table: BTreeMap<FieldChoiceKey, Vec<FieldChoiceAlternative>> =
        BTreeMap::new();
    for (owner_rule, owner_field) in choice_fields {
        let Some(alts) = facts
            .alternatives_by_field
            .get(&(owner_rule.clone(), owner_field.clone()))
        else {
            continue;
        };
        let n = alts.len() as u32;
        if n < 2 {
            continue;
        }
        if n > SHARED_ALPHABET_MAX_BRANCH_ARITY {
            return Err(SyntaxBackendBoundaryError::new(
                "shared_alphabet.arity_overflow",
                format!(
                    "owner_rule={owner_rule} field={owner_field}: multi-alt arity {n} exceeds \
                     SHARED_ALPHABET_MAX_BRANCH_ARITY={SHARED_ALPHABET_MAX_BRANCH_ARITY}; \
                     arity overflow hard-errors (I5 Decision 2) — do not grow the shared pool \
                     per grammar"
                ),
            ));
        }
        let mut ordered: Vec<_> = alts.iter().collect();
        ordered.sort_by(|left, right| {
            left.node_kind
                .cmp(&right.node_kind)
                .then(terminal_class_name(left.terminal).cmp(terminal_class_name(right.terminal)))
        });
        let mut row = Vec::with_capacity(ordered.len());
        for (ordinal, alt) in ordered.into_iter().enumerate() {
            let token = shared_alphabet_choice_token_id(structural_range_start, n, ordinal as u32)?;
            row.push(FieldChoiceAlternative {
                node_kind: alt.node_kind.clone(),
                terminal_class: terminal_class_name(alt.terminal).to_string(),
                token_id: token,
            });
            // Residual global map: first-wins across fields (not used for
            // SharedAlphabet encode/decode once field_choice_table is wired).
            global_first_wins
                .entry(alt.node_kind.clone())
                .or_insert(token);
        }
        field_choice_table.insert((owner_rule, owner_field), row);
    }
    Ok((global_first_wins, field_choice_table))
}

fn require_layout_matches_shared_alphabet(
    layout: &ProjectionTokenLayoutSummary,
) -> Result<(), SyntaxBackendBoundaryError> {
    let expected = ProjectionTokenLayoutSummary::from(&shared_alphabet_token_layout());
    if layout.vocab_size != expected.vocab_size
        || layout.pad_token != expected.pad_token
        || layout.ranges != expected.ranges
    {
        return Err(SyntaxBackendBoundaryError::new(
            "shared_alphabet.layout_mismatch",
            format!(
                "SharedAlphabet mode requires the fixed shared layout \
                 (vocab_size={}, pad_token={}); got vocab_size={}, pad_token={}",
                expected.vocab_size, expected.pad_token, layout.vocab_size, layout.pad_token
            ),
        ));
    }
    Ok(())
}

/// Assign structural choice tokens for mixed-field alternatives that the
/// layout helper does not cover (payload node kinds on mixed fields).
///
/// Tokens live in the structural_decisions band at or above
/// `RAW_TOKEN_ALTERNATIVE_NODE_START_OFFSET`, reusing any existing
/// `node_start_alternative_tokens` entry for the same node kind so Nonterminal
/// and payload identities stay target-neutral and collision-free.
///
/// Go-prefixed grammar IDs (`grammar_id.starts_with("go_")`, same condition as
/// `node_start_alternative_token_ids_from_sidecar`) must not free-slot invent:
/// unpinned mixed alternatives hard-error so operators extend the pin table
/// deliberately via [`GO_STRUCTURAL_TOKEN_ASSIGNMENT_APPEND_RULE`].
fn ensure_mixed_field_choice_tokens(
    grammar_id: &str,
    facts: &ModelFacts,
    roles: &mut SyntaxTokenRoles,
) -> Result<(), SyntaxBackendBoundaryError> {
    if facts.mixed_payload_fields.is_empty() {
        return Ok(());
    }
    let pinned_layout_target = grammar_id.starts_with("go_");
    let first_alternative = roles.structural_range.start + RAW_TOKEN_ALTERNATIVE_NODE_START_OFFSET;
    let mut used: BTreeSet<u32> = roles
        .node_start_alternative_tokens
        .values()
        .copied()
        .collect();
    let mut next = first_alternative;
    for field_key in &facts.mixed_payload_fields {
        let Some(alts) = facts.alternatives_by_field.get(field_key) else {
            continue;
        };
        for alt in alts {
            if roles
                .node_start_alternative_tokens
                .contains_key(&alt.node_kind)
            {
                continue;
            }
            if pinned_layout_target {
                return Err(SyntaxBackendBoundaryError::new(
                    "syntax_state_machine.mixed_payload.choice_token",
                    format!(
                        "owner_rule={} field={} node_kind={} terminal_class={:?}: \
                         Go mixed-field alternative has no pinned alternative \
                         node_start token in {GO_AST_SCHEMA_INVENTORY_FIXTURE_PATH}; \
                         {GO_STRUCTURAL_TOKEN_ASSIGNMENT_APPEND_RULE}",
                        field_key.0, field_key.1, alt.node_kind, alt.terminal
                    ),
                ));
            }
            while used.contains(&next) || next < first_alternative {
                next = next.checked_add(1).ok_or_else(|| {
                    SyntaxBackendBoundaryError::new(
                        "syntax_state_machine.mixed_payload.choice_token",
                        format!(
                            "owner_rule={} field={} node_kind={} terminal_class={:?}: \
                             structural alternative token space exhausted",
                            field_key.0, field_key.1, alt.node_kind, alt.terminal
                        ),
                    )
                })?;
            }
            if next >= roles.structural_range.end_exclusive {
                return Err(SyntaxBackendBoundaryError::new(
                    "syntax_state_machine.mixed_payload.choice_token",
                    format!(
                        "owner_rule={} field={} node_kind={} terminal_class={:?}: \
                         no free structural choice token in range {}..{}",
                        field_key.0,
                        field_key.1,
                        alt.node_kind,
                        alt.terminal,
                        roles.structural_range.start,
                        roles.structural_range.end_exclusive
                    ),
                ));
            }
            roles
                .node_start_alternative_tokens
                .insert(alt.node_kind.clone(), next);
            used.insert(next);
            next += 1;
        }
    }
    roles.validate()?;
    Ok(())
}

fn layout_range_by_role(
    layout: &ProjectionTokenLayoutSummary,
    role: &str,
) -> Result<ProjectionTokenLayoutRangeSummary, SyntaxBackendBoundaryError> {
    layout
        .ranges
        .iter()
        .find(|range| range.role == role)
        .cloned()
        .ok_or_else(|| {
            SyntaxBackendBoundaryError::new(
                "syntax_state_machine.token_layout.range",
                format!("missing token layout role {role}"),
            )
        })
}

fn layout_range_by_role_any(
    layout: &ProjectionTokenLayoutSummary,
    roles: &[&str],
) -> Result<ProjectionTokenLayoutRangeSummary, SyntaxBackendBoundaryError> {
    for role in roles {
        if let Ok(range) = layout_range_by_role(layout, role) {
            return Ok(range);
        }
    }
    Err(SyntaxBackendBoundaryError::new(
        "syntax_state_machine.token_layout.range",
        format!("missing any token layout role from {}", roles.join(", ")),
    ))
}

/// Optional layout role lookup (menu band is additive and not required).
fn optional_layout_range_by_role(
    layout: &ProjectionTokenLayoutSummary,
    role: &str,
) -> Option<ProjectionTokenLayoutRangeSummary> {
    layout
        .ranges
        .iter()
        .find(|range| range.role == role)
        .cloned()
}

#[cfg(test)]
mod mixed_field_choice_token_pin_tests {
    use super::CompiledSyntaxModel;
    use crate::go_ast_schema_inventory::{
        GO_AST_SCHEMA_INVENTORY_FIXTURE_PATH, GO_STRUCTURAL_TOKEN_ASSIGNMENT_APPEND_RULE,
    };
    use crate::grammar_ir::{
        GrammarField, GrammarFieldAlternative, GrammarIr, GrammarIrProvenance, GrammarRule,
        GrammarTerminalKind,
    };
    use crate::grammar_ir_structural_model::{
        GrammarStructuralTokenLayout, lower_grammar_ir_to_structural_model,
    };
    use crate::syntax_go_source_instance_projection::RAW_TOKEN_ALTERNATIVE_NODE_START_OFFSET;
    use crate::toy_grammar_frontend::ToyGrammarSpec;

    /// Mixed field with a Nonterminal that is already in the Go pin table
    /// (`EmptyStmt`) plus an IntegerPayload leaf that is not. Layout assignment
    /// therefore succeeds for the Nonterminal, and only
    /// `ensure_mixed_field_choice_tokens` would invent a free-slot token for
    /// `number` — the hole this test closes.
    fn mixed_field_with_pinned_nt_and_unpinned_payload(grammar_id: &str) -> GrammarIr {
        let mut grammar = GrammarIr::new(
            grammar_id,
            GrammarIrProvenance {
                source_kind: "synthetic_go_mixed_field_pin_probe".to_string(),
                source_path: Some("src/syntax_state_machine.rs".to_string()),
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
                        node_kind: "EmptyStmt".to_string(),
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
            name: "EmptyStmt".to_string(),
            node_kind: "EmptyStmt".to_string(),
            fields: vec![],
        });
        grammar
    }

    fn compile_mixed_probe(grammar_id: &str) -> Result<CompiledSyntaxModel, String> {
        let layout = ToyGrammarSpec::assignment_language_v1().layout;
        let sidecar = lower_grammar_ir_to_structural_model(
            &mixed_field_with_pinned_nt_and_unpinned_payload(grammar_id),
            GrammarStructuralTokenLayout::from_toy_assignment(&layout),
        )
        .map_err(|e| e.to_string())?;
        CompiledSyntaxModel::from_sidecar(&sidecar).map_err(|e| e.to_string())
    }

    #[test]
    fn go_prefixed_grammar_rejects_unpinned_mixed_payload_choice_token() {
        let err = compile_mixed_probe("go_mixed_field_unpinned_payload_probe_v0")
            .expect_err("Go-prefixed mixed field must not free-slot invent choice tokens");
        assert!(
            err.contains("owner_rule=wrapper"),
            "diagnostic must name owner rule: {err}"
        );
        assert!(
            err.contains("field=value"),
            "diagnostic must name field: {err}"
        );
        assert!(
            err.contains("node_kind=number"),
            "diagnostic must name unpinned payload node kind: {err}"
        );
        assert!(
            err.contains("terminal_class=") || err.contains("IntegerPayload"),
            "diagnostic must name terminal class: {err}"
        );
        assert!(
            err.contains(GO_AST_SCHEMA_INVENTORY_FIXTURE_PATH),
            "diagnostic must point at the pin fixture path: {err}"
        );
        assert!(
            err.contains(GO_STRUCTURAL_TOKEN_ASSIGNMENT_APPEND_RULE),
            "diagnostic must name the append rule: {err}"
        );
        assert!(
            err.contains("mixed_payload.choice_token") || err.contains("no pinned alternative"),
            "diagnostic must identify the mixed-field pin guard: {err}"
        );
    }

    #[test]
    fn non_pinned_grammar_still_free_slot_assigns_mixed_payload_choice_tokens() {
        let model = compile_mixed_probe("shared_sm_mixed_field_free_slot_probe_v0")
            .expect("non-pinned grammar must still free-slot assign mixed choice tokens");
        let roles = model.token_roles();
        assert!(
            roles
                .node_start_alternative_tokens
                .contains_key("EmptyStmt"),
            "Nonterminal alternative should have a choice token"
        );
        assert!(
            roles.node_start_alternative_tokens.contains_key("number"),
            "IntegerPayload alternative should free-slot receive a choice token"
        );
        let number = roles.node_start_alternative_tokens["number"];
        assert!(number >= roles.structural_range.start + RAW_TOKEN_ALTERNATIVE_NODE_START_OFFSET);
        assert!(number < roles.structural_range.end_exclusive);
        assert_ne!(number, roles.pad_token);
    }

    #[test]
    fn two_mixed_fields_get_stable_distinct_choice_tokens() {
        use super::model_facts::{ModelFacts, mixed_payload_witness::two_mixed_fields_sidecar};

        let sidecar = two_mixed_fields_sidecar();
        let model = CompiledSyntaxModel::from_sidecar(&sidecar)
            .expect("two-mixed-fields witness should compile");
        let roles = model.token_roles();
        let map = &roles.node_start_alternative_tokens;

        // Every alternative on either field gets a structural choice token.
        for kind in [
            "left_child",
            "right_child",
            "number",
            "text",
            "true",
            "false",
        ] {
            assert!(
                map.contains_key(kind),
                "missing choice token for alternative node kind {kind}"
            );
            let token = map[kind];
            assert!(
                token >= roles.structural_range.start + RAW_TOKEN_ALTERNATIVE_NODE_START_OFFSET
            );
            assert!(token < roles.structural_range.end_exclusive);
            assert_ne!(token, roles.pad_token);
        }

        // Distinct node kinds get distinct tokens.
        assert_ne!(map["left_child"], map["right_child"]);
        assert_ne!(map["left_child"], map["number"]);
        assert_ne!(map["right_child"], map["text"]);
        assert_ne!(map["number"], map["text"]);
        assert_ne!(map["true"], map["false"]);

        // Shared node kinds deliberately share one token across both fields;
        // mixed_field_choice_for_token disambiguates via the active (rule, field).
        // The map is keyed only by node_kind, so a single entry is the share.
        assert_eq!(
            map.keys().filter(|k| *k == "number").count(),
            1,
            "shared IntegerPayload kind must have exactly one map entry"
        );
        assert_eq!(
            map.keys().filter(|k| *k == "true").count(),
            1,
            "shared true must have exactly one map entry"
        );
        assert_eq!(
            map.keys().filter(|k| *k == "false").count(),
            1,
            "shared false must have exactly one map entry"
        );

        // Both fields lower as mixed-payload fields; shared kinds remain
        // alternatives on each field (lookup is by active (rule, field)).
        let facts = ModelFacts::from_sidecar(&sidecar).expect("facts from two-mixed sidecar");
        assert!(
            facts
                .mixed_payload_fields
                .contains(&("wrapper".to_string(), "left".to_string()))
        );
        assert!(
            facts
                .mixed_payload_fields
                .contains(&("wrapper".to_string(), "right".to_string()))
        );
        for field in ["left", "right"] {
            for kind in ["number", "true", "false"] {
                assert!(
                    facts
                        .mixed_field_alternative("wrapper", field, kind)
                        .is_some(),
                    "shared kind {kind} must remain an alternative on wrapper.{field}"
                );
            }
        }
        // Field-local kinds must not leak across fields.
        assert!(
            facts
                .mixed_field_alternative("wrapper", "left", "right_child")
                .is_none()
        );
        assert!(
            facts
                .mixed_field_alternative("wrapper", "right", "left_child")
                .is_none()
        );
        assert!(
            facts
                .mixed_field_alternative("wrapper", "left", "text")
                .is_none()
        );
        assert!(
            facts
                .mixed_field_alternative("wrapper", "right", "text")
                .is_some()
        );
    }

    #[test]
    fn two_mixed_fields_choice_token_assignment_is_deterministic() {
        use super::model_facts::mixed_payload_witness::two_mixed_fields_sidecar;

        let first = CompiledSyntaxModel::from_sidecar(&two_mixed_fields_sidecar())
            .expect("first compile")
            .token_roles()
            .node_start_alternative_tokens
            .clone();
        let second = CompiledSyntaxModel::from_sidecar(&two_mixed_fields_sidecar())
            .expect("second compile")
            .token_roles()
            .node_start_alternative_tokens
            .clone();
        assert_eq!(
            first, second,
            "constructing the same two-mixed-fields model twice must yield \
             identical node_start_alternative_tokens"
        );
    }

    /// Tiny structural band so free-slot assignment runs out of alternative
    /// IDs: fixed roles occupy offsets 0..10, end_exclusive=12 leaves only
    /// two alternative slots; a mixed field with four unique kinds needs three
    /// free-slot payload tokens after the Nonterminal pre-assignment and must
    /// hard-error with the documented diagnostic.
    #[test]
    fn mixed_field_choice_token_exhaustion_errors_when_structural_band_is_full() {
        use crate::grammar_ir::{
            GrammarField, GrammarFieldAlternative, GrammarIr, GrammarIrProvenance, GrammarRule,
            GrammarTerminalKind,
        };
        use crate::grammar_ir_structural_model::{
            GrammarStructuralTokenLayout, lower_grammar_ir_to_structural_model,
        };
        use crate::toy_grammar_frontend::{TokenRange, ToyTokenLayout};

        let mut grammar = GrammarIr::new(
            "shared_sm_mixed_field_exhaustion_probe_v0",
            GrammarIrProvenance {
                source_kind: "synthetic_mixed_field_exhaustion_probe".to_string(),
                source_path: Some("src/syntax_state_machine.rs".to_string()),
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
                    GrammarFieldAlternative {
                        node_kind: "true".to_string(),
                        terminal: GrammarTerminalKind::BooleanPayload,
                    },
                    GrammarFieldAlternative {
                        node_kind: "false".to_string(),
                        terminal: GrammarTerminalKind::BooleanPayload,
                    },
                ],
            }],
        });
        grammar.rules.push(GrammarRule {
            name: "child".to_string(),
            node_kind: "child".to_string(),
            fields: vec![],
        });

        // Structural 0..12 → alternative band capacity 2 (tokens 10, 11).
        // Pre-assignment takes "child" → 10; free-slot fills "number" → 11;
        // "true" then hits end_exclusive and must error.
        let layout = ToyTokenLayout {
            vocab_size: 1024,
            pad_token: 1023,
            structural_tokens: TokenRange {
                start: 0,
                end_exclusive: 12,
            },
            utf8_byte_tokens: TokenRange {
                start: 12,
                end_exclusive: 268,
            },
            u8_integer_tokens: TokenRange {
                start: 268,
                end_exclusive: 524,
            },
        };
        let sidecar = lower_grammar_ir_to_structural_model(
            &grammar,
            GrammarStructuralTokenLayout::from_toy_assignment(&layout),
        )
        .expect("exhaustion probe should lower");
        let err = CompiledSyntaxModel::from_sidecar(&sidecar)
            .expect_err("undersized structural band must exhaust free-slot choice tokens");
        let msg = err.to_string();
        assert!(
            msg.contains("no free structural choice token in range"),
            "diagnostic must name the exhaustion branch: {msg}"
        );
        assert!(
            msg.contains("owner_rule=wrapper") && msg.contains("field=value"),
            "diagnostic must name owner field: {msg}"
        );
        assert!(
            msg.contains("mixed_payload.choice_token")
                || msg.contains("syntax_state_machine.mixed_payload.choice_token"),
            "diagnostic must identify the mixed-field choice_token phase: {msg}"
        );
    }

    #[test]
    fn alternative_node_start_token_values_must_be_unique_across_node_kinds() {
        use super::SyntaxTokenRoles;
        use crate::syntax_go_source_instance_projection::ProjectionTokenLayoutRangeSummary;
        use std::collections::BTreeMap;

        let structural = ProjectionTokenLayoutRangeSummary {
            id: "test.structural".to_string(),
            role: "structural_decisions".to_string(),
            start: 0,
            end_exclusive: 64,
        };
        let payload = ProjectionTokenLayoutRangeSummary {
            id: "test.utf8".to_string(),
            role: "utf8_byte_payload".to_string(),
            start: 64,
            end_exclusive: 320,
        };
        let integer = ProjectionTokenLayoutRangeSummary {
            id: "test.integer".to_string(),
            role: "u8_integer_payload".to_string(),
            start: 320,
            end_exclusive: 576,
        };
        let first_alt = structural.start + RAW_TOKEN_ALTERNATIVE_NODE_START_OFFSET;
        let mut roles = SyntaxTokenRoles {
            vocab_size: 577,
            pad_token: 576,
            structural_range: structural.clone(),
            utf8_range: payload,
            integer_range: integer,
            menu_range: None,
            node_start: structural.start,
            node_end: structural.start + 1,
            field_start: structural.start + 2,
            field_end: structural.start + 3,
            field_missing: structural.start + 4,
            repeated_empty: structural.start + 5,
            terminal: structural.start + 6,
            payload_end: structural.start + 7,
            boolean_false: structural.start + 8,
            boolean_true: structural.start + 9,
            node_start_alternative_tokens: BTreeMap::from([
                ("alpha".to_string(), first_alt),
                ("beta".to_string(), first_alt), // deliberate value collision
            ]),
            field_choice_table: BTreeMap::new(),
            layout_mode: crate::shared_alphabet_layout::SyntaxTokenLayoutMode::PerGrammar,
        };
        let err = roles
            .validate()
            .expect_err("duplicate alternative token values must be rejected");
        assert_eq!(
            err.field(),
            "syntax_state_machine.token_layout.alternative_node_start_unique"
        );
        let text = err.to_string();
        assert!(
            text.contains("alpha") && text.contains("beta"),
            "diagnostic must name both colliding node kinds: {text}"
        );
        assert!(
            text.contains(&first_alt.to_string()),
            "diagnostic must name the colliding token id: {text}"
        );

        // Distinct values still pass; two fields sharing one node kind is one
        // map entry (not two), so that intentional reuse is not a collision.
        roles.node_start_alternative_tokens = BTreeMap::from([
            ("alpha".to_string(), first_alt),
            ("beta".to_string(), first_alt + 1),
        ]);
        roles
            .validate()
            .expect("distinct alternative token values must validate");
    }
}

/// I5 checklist tranche 3: layout mode enum + SharedAlphabet role assignment.
#[cfg(test)]
mod shared_alphabet_role_assignment_tests {
    use super::CompiledSyntaxModel;
    use crate::grammar_ir::{
        GrammarField, GrammarFieldAlternative, GrammarIr, GrammarIrProvenance, GrammarRule,
        GrammarTerminalKind,
    };
    use crate::grammar_ir_structural_model::{
        GrammarStructuralTokenLayout, lower_grammar_ir_to_structural_model,
    };
    use crate::shared_alphabet_layout::{
        SHARED_ALPHABET_MAX_BRANCH_ARITY, SHARED_ALPHABET_VOCAB_SIZE, SyntaxTokenLayoutMode,
        shared_alphabet_choice_token_id, shared_alphabet_token_layout,
    };
    use crate::toy_grammar_frontend::ToyGrammarSpec;

    fn multi_alt_grammar(grammar_id: &str, alt_kinds: &[&str]) -> GrammarIr {
        let mut grammar = GrammarIr::new(
            grammar_id,
            GrammarIrProvenance {
                source_kind: "synthetic_i5_shared_alphabet_role_assignment".to_string(),
                source_path: Some("src/syntax_state_machine.rs".to_string()),
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
                name: "body".to_string(),
                required: true,
                multiple: false,
                alternatives: alt_kinds
                    .iter()
                    .map(|kind| GrammarFieldAlternative {
                        node_kind: (*kind).to_string(),
                        terminal: GrammarTerminalKind::Nonterminal,
                    })
                    .collect(),
            }],
        });
        for kind in alt_kinds {
            grammar.rules.push(GrammarRule {
                name: (*kind).to_string(),
                node_kind: (*kind).to_string(),
                fields: vec![],
            });
        }
        grammar
    }

    fn compile_per_grammar(grammar: &GrammarIr) -> CompiledSyntaxModel {
        let layout = ToyGrammarSpec::assignment_language_v1().layout;
        let sidecar = lower_grammar_ir_to_structural_model(
            grammar,
            GrammarStructuralTokenLayout::from_toy_assignment(&layout),
        )
        .expect("lower multi-alt grammar");
        CompiledSyntaxModel::from_sidecar(&sidecar).expect("PerGrammar compile")
    }

    fn compile_shared(grammar: &GrammarIr) -> CompiledSyntaxModel {
        let layout = ToyGrammarSpec::assignment_language_v1().layout;
        let sidecar = lower_grammar_ir_to_structural_model(
            grammar,
            GrammarStructuralTokenLayout::from_toy_assignment(&layout),
        )
        .expect("lower multi-alt grammar");
        CompiledSyntaxModel::from_sidecar_with_mode(&sidecar, SyntaxTokenLayoutMode::SharedAlphabet)
            .expect("SharedAlphabet compile")
    }

    #[test]
    fn per_grammar_mode_is_bit_identical_to_default_from_sidecar() {
        // Deliberately reverse IR order so alphabetical free-slot assignment
        // differs from IR enumeration order under PerGrammar.
        let grammar = multi_alt_grammar("i5_per_grammar_bit_identical_v0", &["zeta", "alpha"]);
        let via_default = compile_per_grammar(&grammar);
        let layout = ToyGrammarSpec::assignment_language_v1().layout;
        let sidecar = lower_grammar_ir_to_structural_model(
            &grammar,
            GrammarStructuralTokenLayout::from_toy_assignment(&layout),
        )
        .expect("lower");
        let via_explicit = CompiledSyntaxModel::from_sidecar_with_mode(
            &sidecar,
            SyntaxTokenLayoutMode::PerGrammar,
        )
        .expect("explicit PerGrammar");
        assert_eq!(via_default.layout_mode(), SyntaxTokenLayoutMode::PerGrammar);
        assert_eq!(
            via_explicit.layout_mode(),
            SyntaxTokenLayoutMode::PerGrammar
        );
        assert_eq!(
            via_default.token_roles(),
            via_explicit.token_roles(),
            "PerGrammar via from_sidecar_with_mode must match default from_sidecar"
        );
        // Alphabetical free-slot: alpha before zeta.
        let roles = via_default.token_roles();
        let alpha = roles.node_start_alternative_tokens["alpha"];
        let zeta = roles.node_start_alternative_tokens["zeta"];
        assert!(
            alpha < zeta,
            "PerGrammar free-slot assigns alphabetical order"
        );
        assert_ne!(roles.vocab_size, SHARED_ALPHABET_VOCAB_SIZE);
    }

    #[test]
    fn shared_alphabet_assigns_choice_tokens_by_domain_stable_ordinals_arity_2() {
        // IR order deliberately reverse of domain-stable sort so ordinals are
        // not accidental IR enumeration order.
        let grammar = multi_alt_grammar("i5_shared_alphabet_arity2_v0", &["zeta", "alpha"]);
        let model = compile_shared(&grammar);
        assert_eq!(model.layout_mode(), SyntaxTokenLayoutMode::SharedAlphabet);
        let roles = model.token_roles();
        assert_eq!(roles.layout_mode, SyntaxTokenLayoutMode::SharedAlphabet);
        assert_eq!(roles.vocab_size, SHARED_ALPHABET_VOCAB_SIZE);
        assert_eq!(roles.vocab_size, shared_alphabet_token_layout().vocab_size);

        // Domain-stable: (alpha, Nonterminal) then (zeta, Nonterminal).
        let alpha = roles.node_start_alternative_tokens["alpha"];
        let zeta = roles.node_start_alternative_tokens["zeta"];
        assert_eq!(
            alpha,
            shared_alphabet_choice_token_id(0, 2, 0).unwrap(),
            "alpha must be CHOICE(2,0)"
        );
        assert_eq!(
            zeta,
            shared_alphabet_choice_token_id(0, 2, 1).unwrap(),
            "zeta must be CHOICE(2,1)"
        );
        assert_eq!(alpha, 10);
        assert_eq!(zeta, 11);
    }

    #[test]
    fn shared_alphabet_assigns_choice_tokens_by_domain_stable_ordinals_arity_3() {
        let grammar = multi_alt_grammar(
            "i5_shared_alphabet_arity3_v0",
            &["charlie", "alpha", "bravo"],
        );
        let model = compile_shared(&grammar);
        let roles = model.token_roles();
        // Sorted: alpha, bravo, charlie → CHOICE(3, 0..2) at offsets 12,13,14.
        assert_eq!(
            roles.node_start_alternative_tokens["alpha"],
            shared_alphabet_choice_token_id(0, 3, 0).unwrap()
        );
        assert_eq!(
            roles.node_start_alternative_tokens["bravo"],
            shared_alphabet_choice_token_id(0, 3, 1).unwrap()
        );
        assert_eq!(
            roles.node_start_alternative_tokens["charlie"],
            shared_alphabet_choice_token_id(0, 3, 2).unwrap()
        );
        assert_eq!(roles.node_start_alternative_tokens["alpha"], 12);
        assert_eq!(roles.node_start_alternative_tokens["bravo"], 13);
        assert_eq!(roles.node_start_alternative_tokens["charlie"], 14);
    }

    #[test]
    fn shared_alphabet_arity_overflow_hard_errors_at_max_plus_one() {
        let overflow = (SHARED_ALPHABET_MAX_BRANCH_ARITY + 1) as usize;
        let kinds: Vec<String> = (0..overflow).map(|i| format!("alt_{i:02}")).collect();
        let kind_refs: Vec<&str> = kinds.iter().map(String::as_str).collect();
        let grammar = multi_alt_grammar("i5_shared_alphabet_overflow_v0", &kind_refs);
        let layout = ToyGrammarSpec::assignment_language_v1().layout;
        let sidecar = lower_grammar_ir_to_structural_model(
            &grammar,
            GrammarStructuralTokenLayout::from_toy_assignment(&layout),
        )
        .expect("lower overflow grammar");
        let err = CompiledSyntaxModel::from_sidecar_with_mode(
            &sidecar,
            SyntaxTokenLayoutMode::SharedAlphabet,
        )
        .expect_err("arity MAX+1 must hard-error under SharedAlphabet");
        let text = err.to_string();
        assert!(
            text.contains("shared_alphabet.arity_overflow") || text.contains("arity"),
            "diagnostic must identify arity overflow: {text}"
        );
        assert!(
            text.contains("owner_rule=wrapper"),
            "diagnostic must name owner rule: {text}"
        );
        assert!(
            text.contains("field=body"),
            "diagnostic must name field: {text}"
        );
        assert!(
            text.contains(&SHARED_ALPHABET_MAX_BRANCH_ARITY.to_string()),
            "diagnostic must name SHARED_ALPHABET_MAX_BRANCH_ARITY: {text}"
        );
        assert!(
            text.contains(&(SHARED_ALPHABET_MAX_BRANCH_ARITY + 1).to_string()),
            "diagnostic must name the overflowing arity: {text}"
        );
    }

    #[test]
    fn shared_alphabet_reuses_choice_band_across_two_arity2_fields() {
        // Two multi-alt fields with disjoint node kinds both use CHOICE(2,*).
        // Shared mode must allow the same absolute token for different kinds
        // (uniqueness is PerGrammar-only).
        let mut grammar = GrammarIr::new(
            "i5_shared_alphabet_two_fields_v0",
            GrammarIrProvenance {
                source_kind: "synthetic_i5_shared_alphabet_role_assignment".to_string(),
                source_path: Some("src/syntax_state_machine.rs".to_string()),
                source_sha256: None,
                metadata_path: None,
                metadata_sha256: None,
                parser: None,
            },
        );
        grammar.rules.push(GrammarRule {
            name: "wrapper".to_string(),
            node_kind: "wrapper".to_string(),
            fields: vec![
                GrammarField {
                    name: "left".to_string(),
                    required: true,
                    multiple: false,
                    alternatives: vec![
                        GrammarFieldAlternative {
                            node_kind: "la".to_string(),
                            terminal: GrammarTerminalKind::Nonterminal,
                        },
                        GrammarFieldAlternative {
                            node_kind: "lb".to_string(),
                            terminal: GrammarTerminalKind::Nonterminal,
                        },
                    ],
                },
                GrammarField {
                    name: "right".to_string(),
                    required: true,
                    multiple: false,
                    alternatives: vec![
                        GrammarFieldAlternative {
                            node_kind: "ra".to_string(),
                            terminal: GrammarTerminalKind::Nonterminal,
                        },
                        GrammarFieldAlternative {
                            node_kind: "rb".to_string(),
                            terminal: GrammarTerminalKind::Nonterminal,
                        },
                    ],
                },
            ],
        });
        for kind in ["la", "lb", "ra", "rb"] {
            grammar.rules.push(GrammarRule {
                name: kind.to_string(),
                node_kind: kind.to_string(),
                fields: vec![],
            });
        }
        let model = compile_shared(&grammar);
        let roles = model.token_roles();
        let choice20 = shared_alphabet_choice_token_id(0, 2, 0).unwrap();
        let choice21 = shared_alphabet_choice_token_id(0, 2, 1).unwrap();
        // Domain-stable within each field: la/lb and ra/rb.
        assert_eq!(roles.node_start_alternative_tokens["la"], choice20);
        assert_eq!(roles.node_start_alternative_tokens["lb"], choice21);
        assert_eq!(roles.node_start_alternative_tokens["ra"], choice20);
        assert_eq!(roles.node_start_alternative_tokens["rb"], choice21);
    }
}

/// I5 checklist tranches 4–5 — field-relative choice under SharedAlphabet.
///
/// Claim boundary: `compiler_shared_alphabet_layout_only`.
///
/// SharedAlphabet uses `roles.field_choice_table` (Decision 2) for encode and
/// `I-DECODE-SHARED` reverse (Decision 3). The residual global
/// `node_start_alternative_tokens` map remains first-wins and is not a sound
/// reverse identity across fields. PerGrammar stays on the global map and
/// bit-identical default pins. These tests opt into SharedAlphabet only.
#[cfg(test)]
mod shared_alphabet_field_relative_choice_tests {
    use super::{
        CompiledSyntaxModel, RawSyntaxDecodedField, RawSyntaxDecodedNode, RawSyntaxDecodedValue,
    };
    use crate::grammar_ir::{
        GrammarField, GrammarFieldAlternative, GrammarIr, GrammarIrProvenance, GrammarRule,
        GrammarTerminalKind,
    };
    use crate::grammar_ir_structural_model::{
        GrammarStructuralTokenLayout, lower_grammar_ir_to_structural_model,
    };
    use crate::shared_alphabet_layout::{
        SHARED_ALPHABET_CLAIM_BOUNDARY, SyntaxTokenLayoutMode, shared_alphabet_choice_token_id,
    };
    use crate::toy_grammar_frontend::ToyGrammarSpec;

    fn leaf(kind: &str) -> RawSyntaxDecodedNode {
        RawSyntaxDecodedNode {
            rule_name: kind.to_string(),
            node_kind: kind.to_string(),
            fields: vec![],
        }
    }

    fn field_node(name: &str, child: RawSyntaxDecodedNode) -> RawSyntaxDecodedField {
        RawSyntaxDecodedField {
            name: name.to_string(),
            values: vec![RawSyntaxDecodedValue::Node(Box::new(child))],
            missing: false,
            repeated_empty: false,
        }
    }

    fn compile_shared(grammar: &GrammarIr) -> CompiledSyntaxModel {
        let layout = ToyGrammarSpec::assignment_language_v1().layout;
        let sidecar = lower_grammar_ir_to_structural_model(
            grammar,
            GrammarStructuralTokenLayout::from_toy_assignment(&layout),
        )
        .expect("lower grammar for SharedAlphabet field-relative tests");
        CompiledSyntaxModel::from_sidecar_with_mode(&sidecar, SyntaxTokenLayoutMode::SharedAlphabet)
            .expect("SharedAlphabet compile")
    }

    /// (a) Two fields reuse the same absolute `CHOICE(2,k)` for *different*
    /// node kinds. Global reverse-map decode picks the first BTreeMap match
    /// and rejects (or mis-resolves) the open field's legal alternative.
    ///
    /// Field-relative decode must unpack `(n,k)` and index into that field's
    /// ordered table only.
    #[test]
    fn shared_alphabet_decode_shared_choice_token_is_field_relative() {
        assert_eq!(
            SHARED_ALPHABET_CLAIM_BOUNDARY,
            "compiler_shared_alphabet_layout_only"
        );

        let mut grammar = GrammarIr::new(
            "i5_shared_alphabet_decode_field_relative_v0",
            GrammarIrProvenance {
                source_kind: "synthetic_i5_shared_alphabet_field_relative".to_string(),
                source_path: Some("src/syntax_state_machine.rs".to_string()),
                source_sha256: None,
                metadata_path: None,
                metadata_sha256: None,
                parser: None,
            },
        );
        grammar.rules.push(GrammarRule {
            name: "wrapper".to_string(),
            node_kind: "wrapper".to_string(),
            fields: vec![
                GrammarField {
                    name: "left".to_string(),
                    required: true,
                    multiple: false,
                    alternatives: vec![
                        GrammarFieldAlternative {
                            node_kind: "la".to_string(),
                            terminal: GrammarTerminalKind::Nonterminal,
                        },
                        GrammarFieldAlternative {
                            node_kind: "lb".to_string(),
                            terminal: GrammarTerminalKind::Nonterminal,
                        },
                    ],
                },
                GrammarField {
                    name: "right".to_string(),
                    required: true,
                    multiple: false,
                    alternatives: vec![
                        GrammarFieldAlternative {
                            node_kind: "ra".to_string(),
                            terminal: GrammarTerminalKind::Nonterminal,
                        },
                        GrammarFieldAlternative {
                            node_kind: "rb".to_string(),
                            terminal: GrammarTerminalKind::Nonterminal,
                        },
                    ],
                },
            ],
        });
        for kind in ["la", "lb", "ra", "rb"] {
            grammar.rules.push(GrammarRule {
                name: kind.to_string(),
                node_kind: kind.to_string(),
                fields: vec![],
            });
        }

        let model = compile_shared(&grammar);
        let roles = model.token_roles();
        let choice20 = shared_alphabet_choice_token_id(0, 2, 0).unwrap();
        let choice21 = shared_alphabet_choice_token_id(0, 2, 1).unwrap();
        // Domain-stable: la/ra → CHOICE(2,0); lb/rb → CHOICE(2,1). Same absolute
        // ids across fields — the global reverse map cannot disambiguate.
        assert_eq!(
            roles.node_start_alternative_tokens.get("la"),
            Some(&choice20)
        );
        assert_eq!(
            roles.node_start_alternative_tokens.get("ra"),
            Some(&choice20)
        );
        assert_eq!(
            roles.node_start_alternative_tokens.get("lb"),
            Some(&choice21)
        );
        assert_eq!(
            roles.node_start_alternative_tokens.get("rb"),
            Some(&choice21)
        );

        // Tree uses the *second* field's first alternative (ra). Under a sound
        // field-relative map this is still CHOICE(2,0), but decode must recover
        // node_kind "ra" from open field wrapper.right — not "la".
        let tree = RawSyntaxDecodedNode {
            rule_name: "wrapper".to_string(),
            node_kind: "wrapper".to_string(),
            fields: vec![
                field_node("left", leaf("la")),
                field_node("right", leaf("ra")),
            ],
        };

        // Hand-build the expanded stream expected under SharedAlphabet so the
        // test does not depend on a still-broken encode path for defect (a).
        // Shape: node_start, field_start, CHOICE, node_end, field_end, (×2), node_end.
        let tokens = vec![
            roles.node_start,
            roles.field_start,
            choice20, // left → la
            roles.node_end,
            roles.field_end,
            roles.field_start,
            choice20, // right → ra (same absolute id; field gate disambiguates)
            roles.node_end,
            roles.field_end,
            roles.node_end,
        ];

        let decoded = model
            .decode_complete_tree(&tokens)
            .expect("I-DECODE-SHARED: shared CHOICE(2,0) must decode under open field right → ra")
            .decoded_node_tree()
            .expect("decoded event stream must rebuild a node tree");
        assert_eq!(
            decoded, tree,
            "round-trip tree must recover field-relative alternatives"
        );

        let right = decoded
            .field("right")
            .expect("decoded tree must retain right field");
        match right.values.as_slice() {
            [RawSyntaxDecodedValue::Node(child)] => {
                assert_eq!(
                    child.node_kind, "ra",
                    "open field wrapper.right + CHOICE(2,0) must resolve to ra, not the \
                     global reverse-map first match (la)"
                );
            }
            other => panic!("expected single Node value on right, got {other:?}"),
        }
    }

    /// (b) The same `node_kind` appears on two multi-alt fields with different
    /// arities / ordinals. Global first-wins keeps the token from the first
    /// field and encode emits the wrong `CHOICE(n,k)` for the second field.
    ///
    /// Field-relative encode must emit `CHOICE(n_field, k_field)`.
    #[test]
    fn shared_alphabet_encode_same_node_kind_uses_field_relative_choice() {
        assert_eq!(
            SHARED_ALPHABET_CLAIM_BOUNDARY,
            "compiler_shared_alphabet_layout_only"
        );

        let mut grammar = GrammarIr::new(
            "i5_shared_alphabet_encode_field_relative_v0",
            GrammarIrProvenance {
                source_kind: "synthetic_i5_shared_alphabet_field_relative".to_string(),
                source_path: Some("src/syntax_state_machine.rs".to_string()),
                source_sha256: None,
                metadata_path: None,
                metadata_sha256: None,
                parser: None,
            },
        );
        grammar.rules.push(GrammarRule {
            name: "wrapper".to_string(),
            node_kind: "wrapper".to_string(),
            fields: vec![
                // arity 2: common ordinal 0 under domain-stable sort
                GrammarField {
                    name: "left".to_string(),
                    required: true,
                    multiple: false,
                    alternatives: vec![
                        GrammarFieldAlternative {
                            node_kind: "common".to_string(),
                            terminal: GrammarTerminalKind::Nonterminal,
                        },
                        GrammarFieldAlternative {
                            node_kind: "left_only".to_string(),
                            terminal: GrammarTerminalKind::Nonterminal,
                        },
                    ],
                },
                // arity 3: common ordinal 0 under domain-stable sort among
                // {common, right_only_a, right_only_b} — different CHOICE band.
                GrammarField {
                    name: "right".to_string(),
                    required: true,
                    multiple: false,
                    alternatives: vec![
                        GrammarFieldAlternative {
                            node_kind: "common".to_string(),
                            terminal: GrammarTerminalKind::Nonterminal,
                        },
                        GrammarFieldAlternative {
                            node_kind: "right_only_a".to_string(),
                            terminal: GrammarTerminalKind::Nonterminal,
                        },
                        GrammarFieldAlternative {
                            node_kind: "right_only_b".to_string(),
                            terminal: GrammarTerminalKind::Nonterminal,
                        },
                    ],
                },
            ],
        });
        for kind in ["common", "left_only", "right_only_a", "right_only_b"] {
            grammar.rules.push(GrammarRule {
                name: kind.to_string(),
                node_kind: kind.to_string(),
                fields: vec![],
            });
        }

        let model = compile_shared(&grammar);
        let roles = model.token_roles();
        let choice_left_common = shared_alphabet_choice_token_id(0, 2, 0).unwrap(); // CHOICE(2,0)
        let choice_right_common = shared_alphabet_choice_token_id(0, 3, 0).unwrap(); // CHOICE(3,0)
        assert_ne!(
            choice_left_common, choice_right_common,
            "same node_kind on different arities must use different CHOICE tokens"
        );

        // Tree: left=common, right=common. Encode must emit field-local tokens.
        let tree = RawSyntaxDecodedNode {
            rule_name: "wrapper".to_string(),
            node_kind: "wrapper".to_string(),
            fields: vec![
                field_node("left", leaf("common")),
                field_node("right", leaf("common")),
            ],
        };

        let tokens = model
            .encode_complete_tree(&tree)
            .expect("encode common under both fields must succeed");

        let expected = vec![
            roles.node_start,
            roles.field_start,
            choice_left_common, // wrapper.left → CHOICE(2,0)
            roles.node_end,
            roles.field_end,
            roles.field_start,
            choice_right_common, // wrapper.right → CHOICE(3,0), NOT first-wins CHOICE(2,0)
            roles.node_end,
            roles.field_end,
            roles.node_end,
        ];
        assert_eq!(
            tokens, expected,
            "encode must not first-win a global node_kind→token map across fields; \
             wrapper.right.common requires CHOICE(3,0)={choice_right_common}, got stream {tokens:?}"
        );

        // Inverse under field-relative decode (tranche 5); documents the full
        // encode→decode contract once both sides are fixed.
        let decoded = model
            .decode_complete_tree(&tokens)
            .expect("field-relative stream must decode")
            .decoded_node_tree()
            .expect("decoded event stream must rebuild a node tree");
        assert_eq!(decoded, tree);
    }

    /// Spec lock: roles.field_choice_table enumerates every multi-alt / mixed
    /// field independently so (a) token reuse across fields is allowed and
    /// (b) same node_kind may carry different tokens on different fields.
    ///
    /// Encode/decode use this table under SharedAlphabet; the residual global
    /// map remains first-wins and is not a sound reverse identity.
    #[test]
    fn shared_alphabet_field_choice_table_shape_spec() {
        assert_eq!(
            SHARED_ALPHABET_CLAIM_BOUNDARY,
            "compiler_shared_alphabet_layout_only"
        );

        let mut grammar = GrammarIr::new(
            "i5_shared_alphabet_field_table_shape_v0",
            GrammarIrProvenance {
                source_kind: "synthetic_i5_shared_alphabet_field_relative".to_string(),
                source_path: Some("src/syntax_state_machine.rs".to_string()),
                source_sha256: None,
                metadata_path: None,
                metadata_sha256: None,
                parser: None,
            },
        );
        grammar.rules.push(GrammarRule {
            name: "wrapper".to_string(),
            node_kind: "wrapper".to_string(),
            fields: vec![
                GrammarField {
                    name: "left".to_string(),
                    required: true,
                    multiple: false,
                    alternatives: vec![
                        GrammarFieldAlternative {
                            node_kind: "common".to_string(),
                            terminal: GrammarTerminalKind::Nonterminal,
                        },
                        GrammarFieldAlternative {
                            node_kind: "left_only".to_string(),
                            terminal: GrammarTerminalKind::Nonterminal,
                        },
                    ],
                },
                GrammarField {
                    name: "right".to_string(),
                    required: true,
                    multiple: false,
                    alternatives: vec![
                        GrammarFieldAlternative {
                            node_kind: "common".to_string(),
                            terminal: GrammarTerminalKind::Nonterminal,
                        },
                        GrammarFieldAlternative {
                            node_kind: "right_only_a".to_string(),
                            terminal: GrammarTerminalKind::Nonterminal,
                        },
                        GrammarFieldAlternative {
                            node_kind: "right_only_b".to_string(),
                            terminal: GrammarTerminalKind::Nonterminal,
                        },
                    ],
                },
            ],
        });
        for kind in ["common", "left_only", "right_only_a", "right_only_b"] {
            grammar.rules.push(GrammarRule {
                name: kind.to_string(),
                node_kind: kind.to_string(),
                fields: vec![],
            });
        }

        let model = compile_shared(&grammar);
        let roles = model.token_roles();
        let left = roles
            .field_choice_row("wrapper", "left")
            .expect("left field choice row");
        let right = roles
            .field_choice_row("wrapper", "right")
            .expect("right field choice row");
        assert_eq!(left.len(), 2);
        assert_eq!(right.len(), 3);
        assert_eq!(left[0].node_kind, "common");
        assert_eq!(
            left[0].token_id,
            shared_alphabet_choice_token_id(0, 2, 0).unwrap()
        );
        assert_eq!(right[0].node_kind, "common");
        assert_eq!(
            right[0].token_id,
            shared_alphabet_choice_token_id(0, 3, 0).unwrap()
        );
        // Same node_kind, different field-local tokens (load-bearing for (b)).
        assert_ne!(left[0].token_id, right[0].token_id);
        // Encode helper must resolve field-locally (not global first-wins).
        assert_eq!(
            roles.alternative_token_for_field("wrapper", "left", "common"),
            Some(left[0].token_id)
        );
        assert_eq!(
            roles.alternative_token_for_field("wrapper", "right", "common"),
            Some(right[0].token_id)
        );
        // Residual global map still first-wins; encode/decode ignore it.
        let global = &roles.node_start_alternative_tokens;
        assert_eq!(
            global.get("common"),
            Some(&left[0].token_id),
            "residual global map first-wins left's CHOICE(2,0) for common"
        );
        assert_ne!(
            global.get("common"),
            Some(&right[0].token_id),
            "residual global map does not retain right's CHOICE(3,0) — field table owns encode"
        );
    }
}

/// I5 checklist tranche 7 — mask admissibility under the shared CHOICE band.
///
/// Claim boundary: `compiler_shared_alphabet_layout_only`.
///
/// Decision 3: at a choice state for field F of arity n, the admissible mask
/// admits exactly the n tokens `CHOICE(n, 0) .. CHOICE(n, n-1)` (not the entire
/// CHOICE band, and not payload bytes). SharedAlphabet path only; default
/// PerGrammar pins are untouched.
#[cfg(test)]
mod shared_alphabet_mask_admissibility_tests {
    use super::CompiledSyntaxModel;
    use super::model_facts::mixed_payload_witness::mixed_payload_sidecar;
    use crate::grammar_ir::{
        GrammarField, GrammarFieldAlternative, GrammarIr, GrammarIrProvenance, GrammarRule,
        GrammarTerminalKind,
    };
    use crate::grammar_ir_structural_model::{
        GrammarStructuralTokenLayout, lower_grammar_ir_to_structural_model,
    };
    use crate::shared_alphabet_layout::{
        SHARED_ALPHABET_CLAIM_BOUNDARY, SHARED_ALPHABET_MAX_BRANCH_ARITY, SyntaxTokenLayoutMode,
        shared_alphabet_choice_token_id,
    };
    use crate::toy_grammar_frontend::ToyGrammarSpec;
    use std::collections::BTreeSet;

    fn compile_shared(grammar: &GrammarIr) -> CompiledSyntaxModel {
        let layout = ToyGrammarSpec::assignment_language_v1().layout;
        let sidecar = lower_grammar_ir_to_structural_model(
            grammar,
            GrammarStructuralTokenLayout::from_toy_assignment(&layout),
        )
        .expect("lower grammar for SharedAlphabet mask tests");
        CompiledSyntaxModel::from_sidecar_with_mode(&sidecar, SyntaxTokenLayoutMode::SharedAlphabet)
            .expect("SharedAlphabet compile")
    }

    fn multi_alt_required(grammar_id: &str, field: &str, alts: &[&str]) -> GrammarIr {
        let mut grammar = GrammarIr::new(
            grammar_id,
            GrammarIrProvenance {
                source_kind: "synthetic_i5_shared_alphabet_mask".to_string(),
                source_path: Some("src/syntax_state_machine.rs".to_string()),
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
                name: field.to_string(),
                required: true,
                multiple: false,
                alternatives: alts
                    .iter()
                    .map(|kind| GrammarFieldAlternative {
                        node_kind: (*kind).to_string(),
                        terminal: GrammarTerminalKind::Nonterminal,
                    })
                    .collect(),
            }],
        });
        for kind in alts {
            grammar.rules.push(GrammarRule {
                name: (*kind).to_string(),
                node_kind: (*kind).to_string(),
                fields: vec![],
            });
        }
        grammar
    }

    fn choice_set(arity: u32) -> BTreeSet<u32> {
        (0..arity)
            .map(|k| shared_alphabet_choice_token_id(0, arity, k).unwrap())
            .collect()
    }

    /// At open multi-alt field of arity 3, finite mask is exactly CHOICE(3,*).
    #[test]
    fn shared_alphabet_mask_at_arity3_field_is_exactly_choice3_band() {
        assert_eq!(
            SHARED_ALPHABET_CLAIM_BOUNDARY,
            "compiler_shared_alphabet_layout_only"
        );

        let grammar = multi_alt_required(
            "i5_shared_alphabet_mask_arity3_v0",
            "body",
            &["charlie", "alpha", "bravo"],
        );
        let model = compile_shared(&grammar);
        let roles = model.token_roles();
        let prefix = vec![roles.node_start, roles.field_start];
        let admissible = model
            .admissible_next_raw_tokens(&prefix)
            .expect("open multi-alt field under SharedAlphabet");
        let observed: BTreeSet<u32> = admissible.finite_raw_token_ids.iter().copied().collect();
        let expected = choice_set(3);
        assert_eq!(
            observed, expected,
            "arity-3 field must admit exactly CHOICE(3,0..2); got {observed:?}"
        );
        // Domain-stable absolute ids (alpha, bravo, charlie).
        assert_eq!(
            expected,
            BTreeSet::from([
                shared_alphabet_choice_token_id(0, 3, 0).unwrap(),
                shared_alphabet_choice_token_id(0, 3, 1).unwrap(),
                shared_alphabet_choice_token_id(0, 3, 2).unwrap(),
            ])
        );
        assert!(
            !observed.contains(&roles.terminal),
            "multi-alt NT field must not admit generic terminal"
        );
        assert!(
            !observed.contains(&roles.node_start),
            "multi-alt NT field must not admit generic node_start"
        );
        assert!(
            !observed.contains(&roles.field_end),
            "required field with zero values must not admit field_end"
        );
        assert!(
            admissible.raw_token_ranges.is_empty(),
            "choice state must not open payload ranges: {:?}",
            admissible.raw_token_ranges
        );
        assert!(
            !admissible.payload_end_admissible,
            "choice state must not admit payload_end"
        );
    }

    /// Mask must not admit CHOICE tokens from a different arity band.
    #[test]
    fn shared_alphabet_mask_excludes_other_arity_choice_tokens() {
        let grammar = multi_alt_required(
            "i5_shared_alphabet_mask_arity_exclude_v0",
            "body",
            &["alpha", "beta", "gamma"],
        );
        let model = compile_shared(&grammar);
        let roles = model.token_roles();
        let prefix = vec![roles.node_start, roles.field_start];
        let admissible = model
            .admissible_next_raw_tokens(&prefix)
            .expect("open field");
        let observed: BTreeSet<u32> = admissible.finite_raw_token_ids.iter().copied().collect();

        // Wrong-arity bands that must stay closed.
        for arity in [2u32, 4, 5, 10, SHARED_ALPHABET_MAX_BRANCH_ARITY] {
            for k in 0..arity.min(3) {
                let foreign = shared_alphabet_choice_token_id(0, arity, k).unwrap();
                if arity == 3 {
                    continue;
                }
                assert!(
                    !observed.contains(&foreign),
                    "arity-3 field must not admit CHOICE({arity},{k})={foreign}; mask={observed:?}"
                );
            }
        }
        // Entire CHOICE band is huge; mask must stay size n, not pool width.
        assert_eq!(observed.len(), 3);
    }

    /// Two multi-alt fields with different arities: each open field's mask is
    /// exactly its own CHOICE(n,*) set (Decision 3 field gate).
    #[test]
    fn shared_alphabet_mask_is_field_local_across_arity2_and_arity3() {
        let mut grammar = GrammarIr::new(
            "i5_shared_alphabet_mask_two_fields_v0",
            GrammarIrProvenance {
                source_kind: "synthetic_i5_shared_alphabet_mask".to_string(),
                source_path: Some("src/syntax_state_machine.rs".to_string()),
                source_sha256: None,
                metadata_path: None,
                metadata_sha256: None,
                parser: None,
            },
        );
        grammar.rules.push(GrammarRule {
            name: "wrapper".to_string(),
            node_kind: "wrapper".to_string(),
            fields: vec![
                GrammarField {
                    name: "left".to_string(),
                    required: true,
                    multiple: false,
                    alternatives: vec![
                        GrammarFieldAlternative {
                            node_kind: "la".to_string(),
                            terminal: GrammarTerminalKind::Nonterminal,
                        },
                        GrammarFieldAlternative {
                            node_kind: "lb".to_string(),
                            terminal: GrammarTerminalKind::Nonterminal,
                        },
                    ],
                },
                GrammarField {
                    name: "right".to_string(),
                    required: true,
                    multiple: false,
                    alternatives: vec![
                        GrammarFieldAlternative {
                            node_kind: "ra".to_string(),
                            terminal: GrammarTerminalKind::Nonterminal,
                        },
                        GrammarFieldAlternative {
                            node_kind: "rb".to_string(),
                            terminal: GrammarTerminalKind::Nonterminal,
                        },
                        GrammarFieldAlternative {
                            node_kind: "rc".to_string(),
                            terminal: GrammarTerminalKind::Nonterminal,
                        },
                    ],
                },
            ],
        });
        for kind in ["la", "lb", "ra", "rb", "rc"] {
            grammar.rules.push(GrammarRule {
                name: kind.to_string(),
                node_kind: kind.to_string(),
                fields: vec![],
            });
        }

        let model = compile_shared(&grammar);
        let roles = model.token_roles();
        let choice20 = shared_alphabet_choice_token_id(0, 2, 0).unwrap();
        let choice21 = shared_alphabet_choice_token_id(0, 2, 1).unwrap();
        let choice30 = shared_alphabet_choice_token_id(0, 3, 0).unwrap();
        let choice31 = shared_alphabet_choice_token_id(0, 3, 1).unwrap();
        let choice32 = shared_alphabet_choice_token_id(0, 3, 2).unwrap();

        // Open left (arity 2).
        let left_prefix = vec![roles.node_start, roles.field_start];
        let left_mask: BTreeSet<u32> = model
            .admissible_next_raw_tokens(&left_prefix)
            .expect("open left")
            .finite_raw_token_ids
            .iter()
            .copied()
            .collect();
        assert_eq!(
            left_mask,
            BTreeSet::from([choice20, choice21]),
            "open left must admit exactly CHOICE(2,*); got {left_mask:?}"
        );
        assert!(
            !left_mask.contains(&choice30)
                && !left_mask.contains(&choice31)
                && !left_mask.contains(&choice32),
            "open left must not admit CHOICE(3,*)"
        );

        // Complete left with la, then open right (arity 3).
        let right_prefix = vec![
            roles.node_start,
            roles.field_start,
            choice20, // la
            roles.node_end,
            roles.field_end,
            roles.field_start, // right
        ];
        let right_mask: BTreeSet<u32> = model
            .admissible_next_raw_tokens(&right_prefix)
            .expect("open right")
            .finite_raw_token_ids
            .iter()
            .copied()
            .collect();
        assert_eq!(
            right_mask,
            BTreeSet::from([choice30, choice31, choice32]),
            "open right must admit exactly CHOICE(3,*); got {right_mask:?}"
        );
        assert!(
            !right_mask.contains(&choice20) && !right_mask.contains(&choice21),
            "open right must not admit CHOICE(2,*)"
        );
    }

    /// Wrong-arity CHOICE token is not admitted and must fail on advance.
    #[test]
    fn shared_alphabet_rejects_wrong_arity_choice_on_advance() {
        let grammar = multi_alt_required(
            "i5_shared_alphabet_mask_reject_wrong_arity_v0",
            "body",
            &["alpha", "beta", "gamma"],
        );
        let model = compile_shared(&grammar);
        let roles = model.token_roles();
        let prefix = vec![roles.node_start, roles.field_start];
        let wrong = shared_alphabet_choice_token_id(0, 2, 0).unwrap(); // CHOICE(2,0)
        let admissible = model
            .admissible_next_raw_tokens(&prefix)
            .expect("open field");
        assert!(
            !admissible.finite_raw_token_ids.contains(&wrong),
            "CHOICE(2,0) must not be in arity-3 field mask"
        );

        let mut bad = prefix;
        bad.push(wrong);
        let err = model
            .state_from_prefix(&bad)
            .expect_err("wrong-arity CHOICE must not advance");
        let text = format!("{}: {}", err.kind, err.message);
        assert!(
            text.contains("not admissible")
                || text.contains("CHOICE")
                || text.contains("arity")
                || text.contains("shared"),
            "error should identify non-admission / arity: {text}"
        );
    }

    /// Mixed-payload witness under SharedAlphabet: open field admits exactly
    /// CHOICE(5,*) for the five alternatives — no generic terminal, no payload
    /// ranges (shared-mode analogue of mixed_field_mask_admits_only_*).
    #[test]
    fn shared_alphabet_mixed_field_mask_is_exactly_choice5_band() {
        let model = CompiledSyntaxModel::from_sidecar_with_mode(
            &mixed_payload_sidecar(),
            SyntaxTokenLayoutMode::SharedAlphabet,
        )
        .expect("mixed witness SharedAlphabet compile");
        assert_eq!(model.layout_mode(), SyntaxTokenLayoutMode::SharedAlphabet);
        let roles = model.token_roles();

        // Domain-stable sort of mixed alternatives: child, false, number, text, true.
        let expected = choice_set(5);
        assert_eq!(expected.len(), 5);
        for kind in ["child", "false", "number", "text", "true"] {
            let tok = roles
                .alternative_token_for_field("wrapper", "value", kind)
                .unwrap_or_else(|| panic!("missing field choice for {kind}"));
            assert!(
                expected.contains(&tok),
                "{kind} token {tok} must be in CHOICE(5,*)={expected:?}"
            );
        }

        let prefix = vec![roles.node_start, roles.field_start];
        let admissible = model
            .admissible_next_raw_tokens(&prefix)
            .expect("open mixed field under SharedAlphabet");
        let observed: BTreeSet<u32> = admissible.finite_raw_token_ids.iter().copied().collect();
        assert_eq!(
            observed, expected,
            "mixed field under SharedAlphabet must admit exactly CHOICE(5,*); got {observed:?}"
        );
        assert!(
            !observed.contains(&roles.terminal),
            "mixed field must not admit generic terminal under SharedAlphabet"
        );
        assert!(
            !observed.contains(&roles.node_start),
            "mixed field must not admit generic node_start"
        );
        assert!(
            admissible.raw_token_ranges.is_empty(),
            "mixed choice state must not open payload ranges before a choice"
        );
        assert!(!admissible.payload_end_admissible);
    }

    /// Optional multi-alt field: mask is field_missing ∪ CHOICE(n,*), still
    /// not the entire CHOICE band.
    #[test]
    fn shared_alphabet_optional_multi_alt_mask_includes_missing_and_exact_choices() {
        let mut grammar = GrammarIr::new(
            "i5_shared_alphabet_mask_optional_v0",
            GrammarIrProvenance {
                source_kind: "synthetic_i5_shared_alphabet_mask".to_string(),
                source_path: Some("src/syntax_state_machine.rs".to_string()),
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
                name: "opt".to_string(),
                required: false,
                multiple: false,
                alternatives: vec![
                    GrammarFieldAlternative {
                        node_kind: "alpha".to_string(),
                        terminal: GrammarTerminalKind::Nonterminal,
                    },
                    GrammarFieldAlternative {
                        node_kind: "beta".to_string(),
                        terminal: GrammarTerminalKind::Nonterminal,
                    },
                ],
            }],
        });
        for kind in ["alpha", "beta"] {
            grammar.rules.push(GrammarRule {
                name: kind.to_string(),
                node_kind: kind.to_string(),
                fields: vec![],
            });
        }

        let model = compile_shared(&grammar);
        let roles = model.token_roles();
        let prefix = vec![roles.node_start, roles.field_start];
        let observed: BTreeSet<u32> = model
            .admissible_next_raw_tokens(&prefix)
            .expect("open optional multi-alt")
            .finite_raw_token_ids
            .iter()
            .copied()
            .collect();
        let mut expected = choice_set(2);
        expected.insert(roles.field_missing);
        assert_eq!(
            observed, expected,
            "optional multi-alt must admit field_missing ∪ CHOICE(2,*); got {observed:?}"
        );
        // Still exclude wrong-arity CHOICE(3,*).
        for k in 0..3 {
            let foreign = shared_alphabet_choice_token_id(0, 3, k).unwrap();
            assert!(
                !observed.contains(&foreign),
                "optional arity-2 field must not admit CHOICE(3,{k})"
            );
        }
    }
}
