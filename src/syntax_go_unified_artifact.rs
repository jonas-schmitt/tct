//! Unified pinned Go grammar artifact.
//!
//! Bundles the checked go1.24.3 AST schema inventory lowered once into a
//! single `GrammarIr`, its structural-model sidecar over the Go source-file
//! token layout, the compiled raw-syntax state-machine model, and a pinned
//! artifact identity: the `tct-go-grammar-artifact-v1` artifact id plus a
//! content sha256 over the identity constituents (inventory fixture sha, the
//! live canonical stream contract, both Go token-layout descriptors, the
//! fixed role offsets, the pinned alternative node_start table, the
//! structural-model id and digest, and the generated-codec fixture sha). The
//! checked pin fixture
//! [`GO_GRAMMAR_ARTIFACT_PIN_FIXTURE_PATH`] is enforced by the boundary-suite
//! gate `go_grammar_artifact_pin_fixture_matches_live_artifact` and
//! regenerated only via the ignored
//! `regenerate_go_grammar_artifact_pin_fixture` test in
//! `tests/syntax_backend_boundary_test.rs`.
//!
//! Any `content_sha256` change is identity-affecting evidence: treat it like
//! a `tct-token-diff` `breaking_change` for downstream raw-token corpora and
//! BPE artifacts unless the Go token-layout pin gate separately proves the
//! layout itself unchanged.

use crate::go_ast_schema_inventory::{
    GO_AST_SCHEMA_INVENTORY_FIXTURE_PATH, GO_STRUCTURAL_TOKEN_ASSIGNMENT_VERSION,
    GoAstSchemaInventoryAdapter, go_pinned_alternative_node_start_tokens,
};
use crate::grammar_ir::GrammarIr;
use crate::grammar_ir_structural_model::{
    GrammarStructuralModelSidecar, GrammarStructuralTokenLayout,
    lower_grammar_ir_to_structural_model,
};
use crate::provenance::{json_sha256, sha256_hex};
use crate::syntax_backend_boundary::SyntaxBackendBoundaryError;
use crate::syntax_generated_codec::render_go_syntax_codec_fixture;
use crate::syntax_state_machine::{
    CompiledSyntaxModel, RAW_BOOLEAN_FALSE_OFFSET, RAW_BOOLEAN_TRUE_OFFSET, RAW_FIELD_END_OFFSET,
    RAW_FIELD_MISSING_OFFSET, RAW_FIELD_START_OFFSET, RAW_NODE_END_OFFSET, RAW_NODE_START_OFFSET,
    RAW_PAYLOAD_END_OFFSET, RAW_REPEATED_EMPTY_OFFSET, RAW_TERMINAL_OFFSET,
};
use crate::syntax_token_layout::{go_syntax_source_file_token_layout, go_syntax_token_layout};
use crate::syntax_token_layout_diff::{
    LIVE_RAW_SYNTAX_STREAM_CONTRACT, SyntaxTokenLayoutDescriptor, SyntaxTokenLayoutRangeDescriptor,
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::sync::OnceLock;

/// Pinned unified Go grammar artifact id. One artifact id covers every
/// current-plus record once Increment 3 switches emission/validation here.
pub const GO_UNIFIED_GRAMMAR_ARTIFACT_ID: &str = "tct-go-grammar-artifact-v1";

/// Grammar id of the unified inventory grammar as carried by checked
/// source-instance projection reports (`candidate_grammar_id`). Must equal
/// the `grammar_id` stamped by the checked inventory fixture; the pin gate
/// cross-checks this constant against the lowered grammar.
pub const GO_UNIFIED_GRAMMAR_ID: &str = "go_ast_schema_inventory_go1_24_3";

/// `artifact_kind` of the checked inventory fixture when it serves as the
/// projection metadata input (inventory-as-metadata mode of
/// `scripts/analysis/go_syntax_source_instance_project.go`).
pub const GO_AST_SCHEMA_INVENTORY_METADATA_ARTIFACT_KIND: &str = "tct_go_ast_schema_inventory_v1";

/// The generated-codec route bound into the artifact identity. Must stay
/// equal to `syntax_go_source_instance_projection::GENERATED_CODEC_REPORT_ROUTE`
/// (cross-checked by the boundary-suite pin gate so this module never has to
/// import the Go report module).
pub const GO_UNIFIED_GRAMMAR_ARTIFACT_CODEC_ROUTE: &str =
    "go_source_instance_projection_contextual_raw_tokens";

/// Checked pin fixture recording the unified artifact id, content sha256, and
/// the full identity constituents.
pub const GO_GRAMMAR_ARTIFACT_PIN_FIXTURE_PATH: &str =
    "tests/fixtures/syntax_structural_model/go-grammar-artifact-pin-v1.json";

/// `artifact_kind` discriminator of the checked pin fixture.
pub const GO_GRAMMAR_ARTIFACT_PIN_ARTIFACT_KIND: &str = "tct_go_grammar_artifact_pin";

/// Version of the checked pin fixture format.
pub const GO_GRAMMAR_ARTIFACT_PIN_VERSION: u32 = 1;

const GO_AST_SCHEMA_INVENTORY_FIXTURE_CONTENTS: &str =
    include_str!("../tests/fixtures/go_ast_schema_inventory/go1.24.3-ast-schema-inventory-v1.json");

/// Serializable identity constituents of the unified Go grammar artifact.
///
/// `content_sha256` is the sha256 over this struct's stable serde_json
/// serialization (declaration order, `BTreeMap` key order), binding token
/// layout, state-machine input (structural model), and codec identity into a
/// single recomputable hash.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GoUnifiedGrammarArtifactIdentity {
    /// Grammar id stamped by the checked inventory fixture.
    pub grammar_id: String,
    /// Path of the checked go1.24.3 AST schema inventory fixture.
    pub inventory_fixture_path: String,
    /// sha256 of the checked inventory fixture contents.
    pub inventory_fixture_sha256: String,
    /// Pinned structural token assignment version from the inventory fixture.
    pub structural_token_assignment_version: u32,
    /// Live canonical raw-syntax stream contract bound into the artifact
    /// identity (`LIVE_RAW_SYNTAX_STREAM_CONTRACT`). Flipping the contract
    /// changes `content_sha256`, so the deliberate elision break is
    /// identity-affecting through this field even when every token range is
    /// unchanged.
    pub stream_contract: String,
    /// Descriptor of the Go per-construct token layout
    /// (`go-syntax-token-layout-v2`), equal to the layout-pin current leg.
    pub token_layout: SyntaxTokenLayoutDescriptor,
    /// Descriptor of the Go source-file token layout
    /// (`go-syntax-source-file-token-layout-v2`), equal to the layout-pin
    /// current leg.
    pub source_file_token_layout: SyntaxTokenLayoutDescriptor,
    /// The ten fixed structural role offsets, name -> offset within the
    /// structural range.
    pub role_offsets: BTreeMap<String, u32>,
    /// Pinned alternative node_start table from the inventory fixture, node
    /// kind -> offset from the start of the structural range.
    pub pinned_alternative_node_start_offsets: BTreeMap<String, u32>,
    /// Derived structural-model id
    /// (`grammar-ir:{grammar_id}:structural-model-v0`).
    pub structural_model_id: String,
    /// sha256 of the stable serde_json serialization of
    /// `sidecar.structural_model`.
    pub structural_model_sha256: String,
    /// Generated-codec route bound to this artifact.
    pub codec_route: String,
    /// sha256 of `render_go_syntax_codec_fixture` over the unified sidecar.
    pub codec_fixture_sha256: String,
}

impl GoUnifiedGrammarArtifactIdentity {
    /// Recompute the content hash over the identity constituents.
    pub fn content_sha256(&self) -> Result<String, SyntaxBackendBoundaryError> {
        json_sha256(self).map_err(|error| {
            SyntaxBackendBoundaryError::new(
                "go_unified_grammar_artifact.content_sha256",
                format!("failed to serialize the unified artifact identity: {error}"),
            )
        })
    }
}

/// The cached unified Go grammar artifact bundle.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GoUnifiedGrammarArtifact {
    /// Pinned artifact id (`tct-go-grammar-artifact-v1`).
    pub artifact_id: &'static str,
    /// Content sha256 over [`GoUnifiedGrammarArtifactIdentity`].
    pub content_sha256: String,
    /// The identity constituents the content hash covers.
    pub identity: GoUnifiedGrammarArtifactIdentity,
    /// The inventory grammar lowered once from the checked fixture.
    pub grammar: GrammarIr,
    /// Structural-model sidecar over the Go source-file token layout.
    pub sidecar: GrammarStructuralModelSidecar,
    /// Compiled raw-syntax state-machine model for the unified sidecar.
    pub compiled_model: CompiledSyntaxModel,
}

/// The process-wide unified Go grammar artifact, lowered and compiled once.
///
/// Lowering the 53-rule inventory and compiling the state-machine model is
/// reused across all current-plus records, so consumers must not lower the
/// inventory ad hoc when this bundle suffices.
pub fn go_unified_grammar_artifact()
-> Result<&'static GoUnifiedGrammarArtifact, SyntaxBackendBoundaryError> {
    static ARTIFACT: OnceLock<Result<GoUnifiedGrammarArtifact, SyntaxBackendBoundaryError>> =
        OnceLock::new();
    ARTIFACT
        .get_or_init(build_go_unified_grammar_artifact)
        .as_ref()
        .map_err(Clone::clone)
}

fn build_go_unified_grammar_artifact()
-> Result<GoUnifiedGrammarArtifact, SyntaxBackendBoundaryError> {
    let inventory_fixture_sha256 = sha256_hex(GO_AST_SCHEMA_INVENTORY_FIXTURE_CONTENTS.as_bytes());
    let grammar = GoAstSchemaInventoryAdapter::lower_inventory_str(
        GO_AST_SCHEMA_INVENTORY_FIXTURE_CONTENTS,
        Some(GO_AST_SCHEMA_INVENTORY_FIXTURE_PATH.to_string()),
        Some(inventory_fixture_sha256.clone()),
    )
    .map_err(|error| {
        SyntaxBackendBoundaryError::new(
            "go_unified_grammar_artifact.grammar",
            format!("failed to lower the checked Go AST schema inventory: {error}"),
        )
    })?;
    let sidecar =
        lower_grammar_ir_to_structural_model(&grammar, go_syntax_source_file_token_layout())
            .map_err(|error| {
                SyntaxBackendBoundaryError::new(
                    "go_unified_grammar_artifact.structural_model",
                    format!("failed to lower the inventory GrammarIR to StructuralModel: {error}"),
                )
            })?;
    let compiled_model = CompiledSyntaxModel::from_sidecar(&sidecar)?;
    let pinned_offsets = go_pinned_alternative_node_start_tokens().map_err(|error| {
        SyntaxBackendBoundaryError::new(
            "go_unified_grammar_artifact.pinned_alternative_node_start_offsets",
            format!("failed to load the pinned Go structural token assignment: {error}"),
        )
    })?;
    let structural_model_sha256 = json_sha256(&sidecar.structural_model).map_err(|error| {
        SyntaxBackendBoundaryError::new(
            "go_unified_grammar_artifact.structural_model_sha256",
            format!("failed to serialize the unified structural model: {error}"),
        )
    })?;
    let codec_fixture_sha256 = sha256_hex(render_go_syntax_codec_fixture(&sidecar).as_bytes());

    if grammar.grammar_id != GO_UNIFIED_GRAMMAR_ID {
        return Err(SyntaxBackendBoundaryError::new(
            "go_unified_grammar_artifact.grammar_id",
            format!(
                "checked inventory grammar id {} must equal the pinned unified grammar id {GO_UNIFIED_GRAMMAR_ID}",
                grammar.grammar_id
            ),
        ));
    }
    let identity = GoUnifiedGrammarArtifactIdentity {
        grammar_id: grammar.grammar_id.clone(),
        inventory_fixture_path: GO_AST_SCHEMA_INVENTORY_FIXTURE_PATH.to_string(),
        inventory_fixture_sha256,
        structural_token_assignment_version: GO_STRUCTURAL_TOKEN_ASSIGNMENT_VERSION,
        stream_contract: LIVE_RAW_SYNTAX_STREAM_CONTRACT.to_string(),
        token_layout: go_layout_descriptor(
            "go-syntax-token-layout-v2",
            2,
            &go_syntax_token_layout(),
            pinned_offsets,
        )?,
        source_file_token_layout: go_layout_descriptor(
            "go-syntax-source-file-token-layout-v2",
            2,
            &go_syntax_source_file_token_layout(),
            pinned_offsets,
        )?,
        role_offsets: go_fixed_role_offsets(),
        pinned_alternative_node_start_offsets: pinned_offsets.clone(),
        structural_model_id: sidecar.structural_model.model_id.clone(),
        structural_model_sha256,
        codec_route: GO_UNIFIED_GRAMMAR_ARTIFACT_CODEC_ROUTE.to_string(),
        codec_fixture_sha256,
    };
    let content_sha256 = identity.content_sha256()?;

    Ok(GoUnifiedGrammarArtifact {
        artifact_id: GO_UNIFIED_GRAMMAR_ARTIFACT_ID,
        content_sha256,
        identity,
        grammar,
        sidecar,
        compiled_model,
    })
}

fn go_fixed_role_offsets() -> BTreeMap<String, u32> {
    BTreeMap::from([
        ("node_start".to_string(), RAW_NODE_START_OFFSET),
        ("node_end".to_string(), RAW_NODE_END_OFFSET),
        ("field_start".to_string(), RAW_FIELD_START_OFFSET),
        ("field_end".to_string(), RAW_FIELD_END_OFFSET),
        ("field_missing".to_string(), RAW_FIELD_MISSING_OFFSET),
        ("repeated_empty".to_string(), RAW_REPEATED_EMPTY_OFFSET),
        ("terminal".to_string(), RAW_TERMINAL_OFFSET),
        ("payload_end".to_string(), RAW_PAYLOAD_END_OFFSET),
        ("boolean_false".to_string(), RAW_BOOLEAN_FALSE_OFFSET),
        ("boolean_true".to_string(), RAW_BOOLEAN_TRUE_OFFSET),
    ])
}

fn go_layout_descriptor(
    layout_id: &str,
    layout_version: u32,
    layout: &GrammarStructuralTokenLayout,
    pinned_offsets: &BTreeMap<String, u32>,
) -> Result<SyntaxTokenLayoutDescriptor, SyntaxBackendBoundaryError> {
    let structural_start = layout
        .ranges
        .iter()
        .find(|range| range.role == "structural_decisions")
        .map(|range| range.start)
        .ok_or_else(|| {
            SyntaxBackendBoundaryError::new(
                "go_unified_grammar_artifact.token_layout",
                format!("Go layout {layout_id} has no structural_decisions range"),
            )
        })?;
    Ok(SyntaxTokenLayoutDescriptor {
        layout_id: layout_id.to_string(),
        layout_version,
        stream_contract: LIVE_RAW_SYNTAX_STREAM_CONTRACT.to_string(),
        vocab_size: layout.vocab_size,
        pad_token: layout.pad_token,
        ranges: layout
            .ranges
            .iter()
            .map(|range| SyntaxTokenLayoutRangeDescriptor {
                id: range.id.clone(),
                role: range.role.clone(),
                start: range.start,
                end_exclusive: range.end_exclusive,
            })
            .collect(),
        role_offsets: go_fixed_role_offsets(),
        alternative_node_start_tokens: pinned_offsets
            .iter()
            .map(|(node_kind, offset)| (node_kind.clone(), structural_start + *offset))
            .collect(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::syntax_token_layout_diff::{
        RAW_SYNTAX_STREAM_CONTRACT_ELIDED_V1, RAW_SYNTAX_STREAM_CONTRACT_EXPANDED_V1,
    };

    /// The live identity carries the elided canonical contract (the
    /// deliberate elision break landed), and flipping the contract on a copy
    /// of the identity changes `content_sha256`, so any future contract
    /// change stays identity-affecting.
    #[test]
    fn stream_contract_flip_changes_the_identity_content_sha() {
        let artifact =
            go_unified_grammar_artifact().expect("unified Go grammar artifact should build");
        assert_eq!(
            artifact.identity.stream_contract,
            LIVE_RAW_SYNTAX_STREAM_CONTRACT
        );
        assert_eq!(
            artifact.identity.stream_contract, RAW_SYNTAX_STREAM_CONTRACT_ELIDED_V1,
            "post-elision-break guard: the live unified artifact identity carries the elided canonical stream contract"
        );
        for layout in [
            &artifact.identity.token_layout,
            &artifact.identity.source_file_token_layout,
        ] {
            assert_eq!(layout.stream_contract, LIVE_RAW_SYNTAX_STREAM_CONTRACT);
        }

        let mut flipped = artifact.identity.clone();
        flipped.stream_contract = RAW_SYNTAX_STREAM_CONTRACT_EXPANDED_V1.to_string();
        let flipped_sha = flipped
            .content_sha256()
            .expect("flipped identity copy should hash");
        assert_ne!(
            artifact.content_sha256, flipped_sha,
            "a stream-contract change must be identity-affecting through content_sha256"
        );

        // The embedded layout descriptors are identity-affecting on their
        // own as well: a contract flip inside a descriptor leg also moves
        // the content hash.
        let mut descriptor_flipped = artifact.identity.clone();
        descriptor_flipped.source_file_token_layout.stream_contract =
            RAW_SYNTAX_STREAM_CONTRACT_EXPANDED_V1.to_string();
        let descriptor_flipped_sha = descriptor_flipped
            .content_sha256()
            .expect("descriptor-flipped identity copy should hash");
        assert_ne!(artifact.content_sha256, descriptor_flipped_sha);
        assert_ne!(flipped_sha, descriptor_flipped_sha);
    }

    #[test]
    fn unified_artifact_builds_once_and_is_internally_consistent() {
        let artifact =
            go_unified_grammar_artifact().expect("unified Go grammar artifact should build");
        let again = go_unified_grammar_artifact()
            .expect("unified Go grammar artifact should be cached and rebuildable");
        assert!(
            std::ptr::eq(artifact, again),
            "the OnceLock cache must return the same artifact instance"
        );
        assert_eq!(artifact.artifact_id, GO_UNIFIED_GRAMMAR_ARTIFACT_ID);
        assert_eq!(artifact.identity.grammar_id, artifact.grammar.grammar_id);
        assert_eq!(
            artifact.identity.structural_model_id,
            artifact.sidecar.structural_model.model_id
        );
        assert_eq!(
            artifact.content_sha256,
            artifact
                .identity
                .content_sha256()
                .expect("identity hash should recompute"),
        );
    }
}
