//! Checked declarative manifest for the syntax production route.
//!
//! The manifest mirrors the route constructs currently registered in
//! `SYNTAX_PRODUCTION_ROUTE_CONSTRUCTS`. It keeps route setup explicit and
//! checked for the feature-limited stable syntax route: no build script,
//! generated Rust, JSON Schema output, or source-program validity claim is
//! implied by the manifest alone.

use crate::syntax_backend_boundary::SyntaxBackendBoundaryError;
use crate::syntax_production_route::{
    self, SYNTAX_PRODUCTION_ROUTE_CONSTRUCTS, SyntaxProductionRouteImplementationGate,
    SyntaxProductionRouteProjectorMetadataSetup,
};
use crate::syntax_source_corpus_evidence::{self, SyntaxSourceCorpusEvidenceGate};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

pub const TOOL: &str = "syntax_backend_declarative_route_manifest";
pub const VERSION: u32 = 1;
pub const ARTIFACT_KIND: &str = "tct_syntax_backend_declarative_route_manifest";
pub const FIXTURE_PATH: &str = "tests/fixtures/syntax_backend/production-route-manifest-v1.json";
pub const STATUS: &str = "checked_declarative_route_manifest";
pub const COMPLETED_PROMOTION_CLASS: &str = "syntax backend declarative route manifest tranche";
pub const ROUTE_MANIFEST_LOADER: &str =
    "src/syntax_route_manifest.rs::checked_syntax_production_route_manifest_constructs";
pub const LOADER_COMPLETED_PROMOTION_CLASS: &str = "syntax backend route manifest loader tranche";
pub const PROJECTOR_METADATA_SETUP_COMPLETED_PROMOTION_CLASS: &str =
    "syntax backend projector-driven metadata route setup tranche";
pub const GO_MINIMAL_FUNCTION_COMPLETED_PROMOTION_CLASS: &str =
    "syntax backend Go minimal function production route tranche";
pub const NEXT_MISSING_STABLE_PROMOTION_CLASS: &str =
    crate::syntax_roadmap_successors::GO_SOURCE_RECORD_RAW_TOKEN_GENERATOR_CORE_SUCCESSOR_TASK;

const REQUIRED_ROUTE_SETUP_FIELDS: &[&str] = &[
    "target_id",
    "grammar_id",
    "completed_promotion_class",
    "metadata_path",
    "generated_test_markers",
];
const REQUIRED_NO_STABLE_CLAIMS: &[&str] = &[
    "no_generated_json_schema_output_change",
    "no_core_compiler_path_change",
    "no_json_schema_compiler_pass_change",
    "no_generated_source_substring_dependency",
    "no_build_script_route_generation",
];

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SyntaxProductionRouteManifest {
    pub artifact_kind: String,
    pub version: u32,
    pub tool: String,
    pub status: String,
    pub route_setup_source_of_truth: String,
    pub route_entrypoint: String,
    pub route_gate: String,
    pub route_manifest_loader: String,
    pub projector_metadata_setup_source: String,
    pub production_route_fixture_path: String,
    pub source_corpus_evidence_fixture_path: String,
    pub construct_identity_fields: Vec<String>,
    pub projector_metadata_setup_fields: Vec<String>,
    pub constructs: Vec<SyntaxProductionRouteManifestConstruct>,
    pub no_stable_claims: Vec<String>,
    pub completed_promotion_class: String,
    pub loader_completed_promotion_class: String,
    pub projector_metadata_setup_completed_promotion_class: String,
    pub next_missing_stable_promotion_class: String,
}

impl SyntaxProductionRouteManifest {
    pub fn validate_against_route_gate_and_source_corpus(
        &self,
        route_gate: &SyntaxProductionRouteImplementationGate,
        source_corpus_gate: &SyntaxSourceCorpusEvidenceGate,
    ) -> Result<(), SyntaxBackendBoundaryError> {
        require_string("artifact_kind", &self.artifact_kind, ARTIFACT_KIND)?;
        require_u32("version", self.version, VERSION)?;
        require_string("tool", &self.tool, TOOL)?;
        require_string("status", &self.status, STATUS)?;
        require_string(
            "route_setup_source_of_truth",
            &self.route_setup_source_of_truth,
            syntax_production_route::ROUTE_SETUP_SOURCE_OF_TRUTH,
        )?;
        require_string(
            "route_entrypoint",
            &self.route_entrypoint,
            syntax_production_route::ROUTE_ENTRYPOINT,
        )?;
        require_string(
            "route_gate",
            &self.route_gate,
            syntax_production_route::ROUTE_GATE,
        )?;
        require_string(
            "route_manifest_loader",
            &self.route_manifest_loader,
            ROUTE_MANIFEST_LOADER,
        )?;
        require_string(
            "projector_metadata_setup_source",
            &self.projector_metadata_setup_source,
            syntax_production_route::PROJECTOR_METADATA_SETUP_SOURCE_OF_TRUTH,
        )?;
        require_string(
            "production_route_fixture_path",
            &self.production_route_fixture_path,
            syntax_production_route::FIXTURE_PATH,
        )?;
        require_string(
            "source_corpus_evidence_fixture_path",
            &self.source_corpus_evidence_fixture_path,
            syntax_source_corpus_evidence::FIXTURE_PATH,
        )?;
        require_vec_exact(
            "construct_identity_fields",
            &self.construct_identity_fields,
            REQUIRED_ROUTE_SETUP_FIELDS,
        )?;
        require_vec_exact_owned(
            "projector_metadata_setup_fields",
            &self.projector_metadata_setup_fields,
            &syntax_production_route::projector_metadata_setup_fields(),
        )?;
        require_route_constructs("constructs", &self.constructs)?;
        require_all(
            "no_stable_claims",
            &self.no_stable_claims,
            REQUIRED_NO_STABLE_CLAIMS,
        )?;
        require_string(
            "completed_promotion_class",
            &self.completed_promotion_class,
            COMPLETED_PROMOTION_CLASS,
        )?;
        require_string(
            "loader_completed_promotion_class",
            &self.loader_completed_promotion_class,
            LOADER_COMPLETED_PROMOTION_CLASS,
        )?;
        require_string(
            "projector_metadata_setup_completed_promotion_class",
            &self.projector_metadata_setup_completed_promotion_class,
            PROJECTOR_METADATA_SETUP_COMPLETED_PROMOTION_CLASS,
        )?;
        require_string(
            "next_missing_stable_promotion_class",
            &self.next_missing_stable_promotion_class,
            NEXT_MISSING_STABLE_PROMOTION_CLASS,
        )?;

        require_string(
            "route_gate.route_setup.source_of_truth",
            &route_gate.route_setup.source_of_truth,
            &self.route_setup_source_of_truth,
        )?;
        require_vec_exact(
            "route_gate.route_setup.construct_identity_fields",
            &route_gate.route_setup.construct_identity_fields,
            REQUIRED_ROUTE_SETUP_FIELDS,
        )?;
        require_string(
            "route_gate.route_state.route_entrypoint",
            &route_gate.route_state.route_entrypoint,
            &self.route_entrypoint,
        )?;
        require_string(
            "route_gate.route_state.route_gate",
            &route_gate.route_state.route_gate,
            &self.route_gate,
        )?;
        require_string(
            "route_gate.next_missing_stable_promotion_class",
            &route_gate.next_missing_stable_promotion_class,
            NEXT_MISSING_STABLE_PROMOTION_CLASS,
        )?;
        require_string(
            "source_corpus.source_production_route.route_table",
            &source_corpus_gate.source_production_route.route_table,
            &self.route_setup_source_of_truth,
        )?;
        require_string(
            "source_corpus.source_production_route.current_next_missing_stable_promotion_class",
            &source_corpus_gate
                .source_production_route
                .current_next_missing_stable_promotion_class,
            NEXT_MISSING_STABLE_PROMOTION_CLASS,
        )?;
        require_string(
            "source_corpus.source_production_route.projector_metadata_setup_source",
            &source_corpus_gate
                .source_production_route
                .projector_metadata_setup_source,
            &self.projector_metadata_setup_source,
        )?;
        require_string(
            "source_corpus.source_production_route.projector_metadata_setup_completed_promotion_class",
            &source_corpus_gate
                .source_production_route
                .projector_metadata_setup_completed_promotion_class,
            PROJECTOR_METADATA_SETUP_COMPLETED_PROMOTION_CLASS,
        )?;
        require_string(
            "source_corpus.next_missing_stable_promotion_class",
            &source_corpus_gate.next_missing_stable_promotion_class,
            NEXT_MISSING_STABLE_PROMOTION_CLASS,
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SyntaxProductionRouteManifestConstruct {
    pub target_id: String,
    pub grammar_id: String,
    pub completed_promotion_class: String,
    pub metadata_path: String,
    pub projector_metadata_setup: SyntaxProductionRouteProjectorMetadataSetup,
    pub generated_test_markers: Vec<String>,
}

pub fn build_syntax_production_route_manifest() -> SyntaxProductionRouteManifest {
    SyntaxProductionRouteManifest {
        artifact_kind: ARTIFACT_KIND.to_string(),
        version: VERSION,
        tool: TOOL.to_string(),
        status: STATUS.to_string(),
        route_setup_source_of_truth: syntax_production_route::ROUTE_SETUP_SOURCE_OF_TRUTH
            .to_string(),
        route_entrypoint: syntax_production_route::ROUTE_ENTRYPOINT.to_string(),
        route_gate: syntax_production_route::ROUTE_GATE.to_string(),
        route_manifest_loader: ROUTE_MANIFEST_LOADER.to_string(),
        projector_metadata_setup_source:
            syntax_production_route::PROJECTOR_METADATA_SETUP_SOURCE_OF_TRUTH.to_string(),
        production_route_fixture_path: syntax_production_route::FIXTURE_PATH.to_string(),
        source_corpus_evidence_fixture_path: syntax_source_corpus_evidence::FIXTURE_PATH
            .to_string(),
        construct_identity_fields: REQUIRED_ROUTE_SETUP_FIELDS
            .iter()
            .map(|field| (*field).to_string())
            .collect(),
        projector_metadata_setup_fields: syntax_production_route::projector_metadata_setup_fields(),
        constructs: syntax_production_route_manifest_constructs(),
        no_stable_claims: REQUIRED_NO_STABLE_CLAIMS
            .iter()
            .map(|claim| (*claim).to_string())
            .collect(),
        completed_promotion_class: COMPLETED_PROMOTION_CLASS.to_string(),
        loader_completed_promotion_class: LOADER_COMPLETED_PROMOTION_CLASS.to_string(),
        projector_metadata_setup_completed_promotion_class:
            PROJECTOR_METADATA_SETUP_COMPLETED_PROMOTION_CLASS.to_string(),
        next_missing_stable_promotion_class: NEXT_MISSING_STABLE_PROMOTION_CLASS.to_string(),
    }
}

pub fn checked_syntax_production_route_manifest_constructs<'a>(
    manifest: &'a SyntaxProductionRouteManifest,
    route_gate: &SyntaxProductionRouteImplementationGate,
    source_corpus_gate: &SyntaxSourceCorpusEvidenceGate,
) -> Result<&'a [SyntaxProductionRouteManifestConstruct], SyntaxBackendBoundaryError> {
    manifest.validate_against_route_gate_and_source_corpus(route_gate, source_corpus_gate)?;
    Ok(&manifest.constructs)
}

pub fn syntax_production_route_manifest_constructs() -> Vec<SyntaxProductionRouteManifestConstruct>
{
    SYNTAX_PRODUCTION_ROUTE_CONSTRUCTS
        .iter()
        .map(|construct| SyntaxProductionRouteManifestConstruct {
            target_id: construct.target_id.to_string(),
            grammar_id: construct.grammar_id.to_string(),
            completed_promotion_class: construct.completed_promotion_class.to_string(),
            metadata_path: construct.metadata_path.to_string(),
            projector_metadata_setup:
                syntax_production_route::syntax_production_route_projector_metadata_setup(
                    construct,
                ),
            generated_test_markers: construct
                .generated_test_markers
                .iter()
                .map(|marker| (*marker).to_string())
                .collect(),
        })
        .collect()
}

fn require_route_constructs(
    field: &'static str,
    actual: &[SyntaxProductionRouteManifestConstruct],
) -> Result<(), SyntaxBackendBoundaryError> {
    let expected = syntax_production_route_manifest_constructs();
    if actual != expected {
        return Err(SyntaxBackendBoundaryError::new(
            field,
            "manifest constructs must match SYNTAX_PRODUCTION_ROUTE_CONSTRUCTS",
        ));
    }

    let mut grammar_ids = BTreeSet::new();
    for construct in actual {
        if construct.generated_test_markers.is_empty() {
            return Err(SyntaxBackendBoundaryError::new(
                field,
                format!(
                    "construct {} must preserve generated-code test markers",
                    construct.grammar_id
                ),
            ));
        }
        let expected_construct = expected
            .iter()
            .find(|candidate| candidate.grammar_id == construct.grammar_id)
            .ok_or_else(|| {
                SyntaxBackendBoundaryError::new(
                    field,
                    format!(
                        "construct {} must exist in route setup",
                        construct.grammar_id
                    ),
                )
            })?;
        if construct.projector_metadata_setup != expected_construct.projector_metadata_setup {
            return Err(SyntaxBackendBoundaryError::new(
                field,
                format!(
                    "construct {} must match projector metadata setup",
                    construct.grammar_id
                ),
            ));
        }
        require_string(
            "construct.projector_metadata_setup.checked_metadata_path",
            &construct.projector_metadata_setup.checked_metadata_path,
            &construct.metadata_path,
        )?;
        if !construct
            .projector_metadata_setup
            .generated_output_path
            .starts_with("target/")
        {
            return Err(SyntaxBackendBoundaryError::new(
                field,
                format!(
                    "construct {} projector output must stay under target/",
                    construct.grammar_id
                ),
            ));
        }
        if !construct
            .projector_metadata_setup
            .route_manifest_projector_backed
        {
            return Err(SyntaxBackendBoundaryError::new(
                field,
                format!(
                    "construct {} must be route-manifest projector-backed",
                    construct.grammar_id
                ),
            ));
        }
        if !grammar_ids.insert(construct.grammar_id.as_str()) {
            return Err(SyntaxBackendBoundaryError::new(
                field,
                format!("duplicate grammar id {}", construct.grammar_id),
            ));
        }
    }
    Ok(())
}

fn require_string(
    field: &'static str,
    actual: &str,
    expected: &str,
) -> Result<(), SyntaxBackendBoundaryError> {
    if actual == expected {
        Ok(())
    } else {
        Err(SyntaxBackendBoundaryError::new(
            field,
            format!("expected {expected}, got {actual}"),
        ))
    }
}

fn require_u32(
    field: &'static str,
    actual: u32,
    expected: u32,
) -> Result<(), SyntaxBackendBoundaryError> {
    if actual == expected {
        Ok(())
    } else {
        Err(SyntaxBackendBoundaryError::new(
            field,
            format!("expected {expected}, got {actual}"),
        ))
    }
}

fn require_all(
    field: &'static str,
    actual: &[String],
    expected: &[&str],
) -> Result<(), SyntaxBackendBoundaryError> {
    let actual = actual.iter().map(String::as_str).collect::<BTreeSet<_>>();
    for required in expected {
        if !actual.contains(required) {
            return Err(SyntaxBackendBoundaryError::new(
                field,
                format!("missing required value {required}"),
            ));
        }
    }
    Ok(())
}

fn require_vec_exact(
    field: &'static str,
    actual: &[String],
    expected: &[&str],
) -> Result<(), SyntaxBackendBoundaryError> {
    let expected = expected
        .iter()
        .map(|value| (*value).to_string())
        .collect::<Vec<_>>();
    if actual == expected {
        Ok(())
    } else {
        Err(SyntaxBackendBoundaryError::new(
            field,
            format!("expected {expected:?}, got {actual:?}"),
        ))
    }
}

fn require_vec_exact_owned(
    field: &'static str,
    actual: &[String],
    expected: &[String],
) -> Result<(), SyntaxBackendBoundaryError> {
    if actual == expected {
        Ok(())
    } else {
        Err(SyntaxBackendBoundaryError::new(
            field,
            format!("expected {expected:?}, got {actual:?}"),
        ))
    }
}
