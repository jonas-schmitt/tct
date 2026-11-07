//! Projector metadata setup for syntax production-route constructs.

use super::SyntaxProductionRouteConstruct;
use serde::{Deserialize, Serialize};

pub const CSHARP_CHECKED_METADATA_PROJECTOR_COMMAND: &str = "python scripts/analysis/csharp_syntax_metadata_smoke.py --source tests/fixtures/csharp_syntax_sources/minimal_class/source.cs --checked-metadata tests/fixtures/csharp_syntax_metadata/minimal_class/syntax-metadata-v0.json --work-dir target/tct-analysis/csharp-syntax-metadata-smoke --self-test";
pub const CSHARP_CHECKED_METADATA_PROJECTOR_OUTPUT_PATH: &str =
    "target/tct-analysis/csharp-syntax-metadata-smoke/minimal-class.syntax-metadata-v0.json";
const CSHARP_CHECKED_METADATA_PROJECTOR_OUTPUT_ROOT: &str =
    "target/tct-analysis/csharp-syntax-metadata-smoke";
pub const GO_CHECKED_METADATA_PROJECTOR_COMMAND_TEMPLATE: &str = "go run scripts/analysis/go_syntax_metadata_smoke.go --fixture <fixture> --write-checked-metadata --generated-output target/tct-analysis/go-syntax-metadata-smoke/<fixture>.syntax-metadata-v0.json --self-test";
pub const GO_CHECKED_METADATA_PROJECTOR_COMMAND: &str =
    GO_CHECKED_METADATA_PROJECTOR_COMMAND_TEMPLATE;
pub const GO_CHECKED_METADATA_PROJECTOR_OUTPUT_ROOT: &str =
    "target/tct-analysis/go-syntax-metadata-smoke";

const REQUIRED_PROJECTOR_METADATA_SETUP_FIELDS: &[&str] = &[
    "projector_command",
    "checked_metadata_path",
    "generated_output_path",
    "route_manifest_projector_backed",
];

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SyntaxProductionRouteProjectorMetadataSetup {
    pub projector_command: String,
    pub checked_metadata_path: String,
    pub generated_output_path: String,
    pub route_manifest_projector_backed: bool,
}

pub fn projector_metadata_setup_fields() -> Vec<String> {
    REQUIRED_PROJECTOR_METADATA_SETUP_FIELDS
        .iter()
        .map(|field| (*field).to_string())
        .collect()
}

pub fn syntax_production_route_projector_metadata_setup(
    construct: &SyntaxProductionRouteConstruct,
) -> SyntaxProductionRouteProjectorMetadataSetup {
    let generated_output_path = projector_generated_output_path(construct);
    let projector_command = match construct.target_id {
        "csharp" if projector_fixture_id(construct).starts_with("compilation-unit-") => {
            format!(
                "python scripts/analysis/csharp_syntax_metadata_smoke.py --fixture {} --write-checked-metadata --checked-metadata {} --work-dir {} --self-test",
                projector_fixture_id(construct),
                construct.metadata_path,
                CSHARP_CHECKED_METADATA_PROJECTOR_OUTPUT_ROOT
            )
        }
        "csharp" => format!(
            "python scripts/analysis/csharp_syntax_metadata_smoke.py --source {} --checked-metadata {} --work-dir {} --self-test",
            csharp_source_path(construct),
            construct.metadata_path,
            CSHARP_CHECKED_METADATA_PROJECTOR_OUTPUT_ROOT
        ),
        "go" => format!(
            "go run scripts/analysis/go_syntax_metadata_smoke.go --fixture {} --write-checked-metadata --generated-output {} --self-test",
            projector_fixture_id(construct),
            generated_output_path
        ),
        _ => format!("unsupported projector target {}", construct.target_id),
    };
    SyntaxProductionRouteProjectorMetadataSetup {
        projector_command,
        checked_metadata_path: construct.metadata_path.to_string(),
        generated_output_path,
        route_manifest_projector_backed: matches!(construct.target_id, "csharp" | "go"),
    }
}

fn projector_generated_output_path(construct: &SyntaxProductionRouteConstruct) -> String {
    match construct.target_id {
        "csharp" => format!(
            "{}/{}.syntax-metadata-v0.json",
            CSHARP_CHECKED_METADATA_PROJECTOR_OUTPUT_ROOT,
            projector_fixture_id(construct)
        ),
        "go" => format!(
            "{}/{}.syntax-metadata-v0.json",
            GO_CHECKED_METADATA_PROJECTOR_OUTPUT_ROOT,
            projector_fixture_id(construct)
        ),
        _ => format!(
            "target/tct-analysis/unsupported/{}.json",
            construct.grammar_id
        ),
    }
}

fn projector_fixture_id(construct: &SyntaxProductionRouteConstruct) -> String {
    let raw = construct
        .metadata_path
        .strip_suffix("/syntax-metadata-v0.json")
        .and_then(|path| path.rsplit('/').next())
        .unwrap_or(construct.grammar_id);
    raw.replace('_', "-")
}

fn csharp_source_path(construct: &SyntaxProductionRouteConstruct) -> String {
    format!(
        "tests/fixtures/csharp_syntax_sources/{}/source.cs",
        projector_fixture_id(construct).replace('-', "_")
    )
}
