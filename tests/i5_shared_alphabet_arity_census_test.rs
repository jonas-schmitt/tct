//! I5 shared decision-token alphabet — checklist tranche 1 arity census.
//!
//! Measures `max(finite_domain.alternatives.len())` for multi-alt fields on
//! retained pins so `MAX_BRANCH_ARITY` can be confirmed or bumped **before**
//! shared layout constants are written (tranche 2).
//!
//! Claim id: `i5_shared_decision_token_alphabet_contract_v0`
//! Boundary: `compiler_shared_alphabet_layout_only`
//!
//! Not a paper claim, production route, transfer experiment, or pin rewrite.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use serde_json::{Value, json};
use tct::csharp_syntax_metadata::CSharpSyntaxMetadataAdapter;
use tct::go_ast_schema_inventory::{
    GO_AST_SCHEMA_INVENTORY_FIXTURE_PATH, GoAstSchemaInventoryAdapter,
};
use tct::grammar_ir_structural_model::{
    GrammarIntegerValueRange, GrammarStructuralModel, GrammarStructuralTokenLayout,
    GrammarStructuralTokenRange, lower_grammar_ir_to_structural_model,
};
use tct::shared_alphabet_layout::SHARED_ALPHABET_MAX_BRANCH_ARITY;
use tct::syntax_token_layout::{csharp_syntax_token_layout, go_syntax_token_layout};
use tct::tree_sitter_c_subset_codec::TreeSitterCSubsetCodec;
use tct::tree_sitter_json_subset_codec::TreeSitterJsonSubsetCodec;
use tct::tree_sitter_metadata::{TreeSitterGrammarPin, TreeSitterMetadataAdapter};

const CLAIM_ID: &str = "i5_shared_decision_token_alphabet_contract_v0";
const CLAIM_BOUNDARY: &str = "compiler_shared_alphabet_layout_only";
const ARTIFACT_KIND: &str = "tct_i5_shared_alphabet_arity_census_v0";
const TOOL: &str = "i5_shared_alphabet_arity_census";
const VERSION: u32 = 0;

/// Design-default proposed in Decision 1 before this census.
const DESIGN_DEFAULT_MAX_BRANCH_ARITY: u32 = 32;
/// Census-settled constant baked by tranche 2 as `SHARED_ALPHABET_MAX_BRANCH_ARITY`.
/// Raising later is a deliberate alphabet revision (layout-diff for shared mode).
const CENSUS_SETTLED_MAX_BRANCH_ARITY: u32 = SHARED_ALPHABET_MAX_BRANCH_ARITY;

const REPORT_PATH: &str = "target/tct-i5-shared-alphabet-arity-census.json";

#[derive(Debug, Clone)]
struct MultiAltSite {
    owner_rule: String,
    owner_field: String,
    arity: usize,
    domain_id: String,
}

#[derive(Debug, Clone)]
struct SurfaceCensus {
    surface_id: &'static str,
    source: String,
    multi_alt_field_count: usize,
    max_arity: usize,
    max_sites: Vec<MultiAltSite>,
    top_sites: Vec<MultiAltSite>,
}

fn multi_alt_sites(model: &GrammarStructuralModel) -> Vec<MultiAltSite> {
    let mut sites: Vec<MultiAltSite> = model
        .finite_domains
        .iter()
        .filter(|domain| domain.alternatives.len() >= 2)
        .map(|domain| MultiAltSite {
            owner_rule: domain.owner_rule.clone(),
            owner_field: domain.owner_field.clone(),
            arity: domain.alternatives.len(),
            domain_id: domain.id.clone(),
        })
        .collect();
    sites.sort_by(|a, b| {
        b.arity
            .cmp(&a.arity)
            .then_with(|| a.owner_rule.cmp(&b.owner_rule))
            .then_with(|| a.owner_field.cmp(&b.owner_field))
    });
    sites
}

fn census_surface(
    surface_id: &'static str,
    source: impl Into<String>,
    model: &GrammarStructuralModel,
) -> SurfaceCensus {
    let sites = multi_alt_sites(model);
    let multi_alt_field_count = sites.len();
    let max_arity = sites.first().map(|site| site.arity).unwrap_or(0);
    let max_sites: Vec<MultiAltSite> = sites
        .iter()
        .filter(|site| site.arity == max_arity && max_arity >= 2)
        .cloned()
        .collect();
    let top_sites: Vec<MultiAltSite> = sites.into_iter().take(8).collect();
    SurfaceCensus {
        surface_id,
        source: source.into(),
        multi_alt_field_count,
        max_arity,
        max_sites,
        top_sites,
    }
}

fn site_json(site: &MultiAltSite) -> Value {
    json!({
        "owner_rule": site.owner_rule,
        "owner_field": site.owner_field,
        "arity": site.arity,
        "domain_id": site.domain_id,
    })
}

fn surface_json(surface: &SurfaceCensus) -> Value {
    json!({
        "surface_id": surface.surface_id,
        "source": surface.source,
        "multi_alt_field_count": surface.multi_alt_field_count,
        "max_arity": surface.max_arity,
        "max_sites": surface.max_sites.iter().map(site_json).collect::<Vec<_>>(),
        "top_sites": surface.top_sites.iter().map(site_json).collect::<Vec<_>>(),
    })
}

fn go_inventory_census() -> SurfaceCensus {
    let grammar = GoAstSchemaInventoryAdapter::lower_inventory_file(Path::new(
        GO_AST_SCHEMA_INVENTORY_FIXTURE_PATH,
    ))
    .expect("checked Go AST schema inventory should lower to GrammarIR");
    let sidecar = lower_grammar_ir_to_structural_model(&grammar, go_syntax_token_layout())
        .expect("Go inventory GrammarIR should lower to StructuralModel");
    census_surface(
        "go_inventory",
        GO_AST_SCHEMA_INVENTORY_FIXTURE_PATH,
        &sidecar.structural_model,
    )
}

fn csharp_layout_census() -> SurfaceCensus {
    // C# has no single full inventory; scan every checked metadata fixture that
    // uses the retained C# token layout so max arity is not fixture-local.
    let root = Path::new("tests/fixtures/csharp_syntax_metadata");
    let mut per_fixture: BTreeMap<String, usize> = BTreeMap::new();
    let mut merged_sites: Vec<MultiAltSite> = Vec::new();
    let mut fixture_count = 0usize;

    for entry in fs::read_dir(root).expect("csharp_syntax_metadata fixtures should exist") {
        let entry = entry.expect("fixture dir entry");
        let meta = entry.path().join("syntax-metadata-v0.json");
        if !meta.is_file() {
            continue;
        }
        fixture_count += 1;
        let fixture_name = entry.file_name().to_string_lossy().into_owned();
        let grammar_id = format!("csharp_arity_census_{fixture_name}");
        let adapter = CSharpSyntaxMetadataAdapter::new(grammar_id);
        let grammar = adapter
            .lower_metadata_file(&meta)
            .unwrap_or_else(|error| panic!("C# metadata {} should lower: {error}", meta.display()));
        let sidecar = lower_grammar_ir_to_structural_model(&grammar, csharp_syntax_token_layout())
            .unwrap_or_else(|error| {
                panic!(
                    "C# {} StructuralModel lower failed: {error}",
                    meta.display()
                )
            });
        let sites = multi_alt_sites(&sidecar.structural_model);
        let max_here = sites.first().map(|s| s.arity).unwrap_or(0);
        per_fixture.insert(fixture_name.clone(), max_here);
        for mut site in sites {
            // Disambiguate fixture in domain_id for merged reporting.
            site.domain_id = format!("{fixture_name}::{}", site.domain_id);
            merged_sites.push(site);
        }
    }

    assert!(
        fixture_count > 0,
        "expected at least one C# syntax-metadata fixture under {root:?}"
    );

    merged_sites.sort_by(|a, b| {
        b.arity
            .cmp(&a.arity)
            .then_with(|| a.owner_rule.cmp(&b.owner_rule))
            .then_with(|| a.owner_field.cmp(&b.owner_field))
    });
    let max_arity = merged_sites.first().map(|s| s.arity).unwrap_or(0);
    let max_sites: Vec<MultiAltSite> = merged_sites
        .iter()
        .filter(|s| s.arity == max_arity && max_arity >= 2)
        .cloned()
        .collect();
    let multi_alt_field_count = merged_sites.iter().filter(|s| s.arity >= 2).count();
    let top_sites: Vec<MultiAltSite> = merged_sites.into_iter().take(8).collect();

    // Encode fixture map in source string for the report.
    let fixture_summary = per_fixture
        .iter()
        .map(|(name, max)| format!("{name}={max}"))
        .collect::<Vec<_>>()
        .join(",");

    let mut surface = SurfaceCensus {
        surface_id: "csharp_layout",
        source: format!(
            "tests/fixtures/csharp_syntax_metadata/* ({fixture_count} fixtures; {fixture_summary})"
        ),
        multi_alt_field_count,
        max_arity,
        max_sites,
        top_sites,
    };
    // Keep source bounded for readability in assertions.
    if surface.source.len() > 400 {
        surface.source = format!(
            "tests/fixtures/csharp_syntax_metadata/* ({fixture_count} fixtures; max_per_fixture tracked in report surfaces)"
        );
    }
    let _ = per_fixture; // retained via source string when short
    surface
}

fn json_subset_census() -> SurfaceCensus {
    let codec = TreeSitterJsonSubsetCodec::from_pinned_fixture()
        .expect("JSON subset codec should load pinned fixture");
    census_surface(
        "tree_sitter_json_subset",
        "TreeSitterJsonSubsetCodec::from_pinned_fixture",
        &codec.sidecar().structural_model,
    )
}

fn c_subset_census() -> SurfaceCensus {
    let codec = TreeSitterCSubsetCodec::from_pinned_fixture()
        .expect("C subset codec should load pinned fixture");
    census_surface(
        "tree_sitter_c_subset",
        "TreeSitterCSubsetCodec::from_pinned_fixture",
        &codec.sidecar().structural_model,
    )
}

/// Oversized structural band for held-out rust SM lower only (same shape as the
/// generic-codec held-out proof). Not a production layout or pin.
fn tree_sitter_rust_held_out_sidecar_token_layout() -> GrammarStructuralTokenLayout {
    GrammarStructuralTokenLayout {
        vocab_size: 4097,
        pad_token: 4096,
        ranges: vec![
            GrammarStructuralTokenRange {
                id: "tree_sitter_rust.structural_tokens".to_string(),
                role: "structural_decisions".to_string(),
                start: 0,
                end_exclusive: 3072,
            },
            GrammarStructuralTokenRange {
                id: "tree_sitter_rust.utf8_byte_payload".to_string(),
                role: "utf8_byte_payload".to_string(),
                start: 3072,
                end_exclusive: 3328,
            },
            GrammarStructuralTokenRange {
                id: "tree_sitter_rust.u8_integer_payload".to_string(),
                role: "u8_integer_payload".to_string(),
                start: 3328,
                end_exclusive: 4096,
            },
        ],
        integer_value_range: Some(GrammarIntegerValueRange {
            id: "tree_sitter_rust.u8_integer_value".to_string(),
            valid_min: 0,
            valid_max: 255,
        }),
    }
}

fn rust_held_out_census() -> SurfaceCensus {
    let adapter = TreeSitterMetadataAdapter::new("tree_sitter_rust_v0_24_0");
    let (grammar, pin) = adapter
        .lower_pinned_grammar_dir(&TreeSitterGrammarPin::rust_fixture_dir())
        .expect("held-out rust pin should lower to GrammarIR");
    let sidecar = lower_grammar_ir_to_structural_model(
        &grammar,
        tree_sitter_rust_held_out_sidecar_token_layout(),
    )
    .expect("held-out rust pin should lower to StructuralModel");
    census_surface(
        "tree_sitter_rust_held_out_pin",
        format!(
            "{} (pin_id={})",
            TreeSitterGrammarPin::rust_fixture_dir().display(),
            pin.pin_id
        ),
        &sidecar.structural_model,
    )
}

fn write_report(surfaces: &[SurfaceCensus], overall_max: usize, design_default_ok: bool) {
    let path = PathBuf::from(REPORT_PATH);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).expect("target/ should be creatable");
    }
    let report = json!({
        "artifact_kind": ARTIFACT_KIND,
        "tool": TOOL,
        "version": VERSION,
        "claim_id": CLAIM_ID,
        "claim_boundary": CLAIM_BOUNDARY,
        "no_stable_claims": [
            "not_paper_claim",
            "not_transfer_or_interference",
            "not_production_route",
            "not_pin_rewrite",
            "not_shared_layout_constant_bake",
        ],
        "method": "max(finite_domain.alternatives.len()) over multi-alt fields (arity >= 2) after GrammarIR -> StructuralModel lower on retained pins",
        "design_default_max_branch_arity": DESIGN_DEFAULT_MAX_BRANCH_ARITY,
        "census_settled_max_branch_arity": CENSUS_SETTLED_MAX_BRANCH_ARITY,
        "overall_max_arity": overall_max,
        "design_default_sufficient": design_default_ok,
        "decision": if design_default_ok {
            "confirm_design_default_32"
        } else {
            "documented_bump_required"
        },
        "bump_note": if design_default_ok {
            Value::Null
        } else {
            json!({
                "from": DESIGN_DEFAULT_MAX_BRANCH_ARITY,
                "to": CENSUS_SETTLED_MAX_BRANCH_ARITY,
                "reason": "held-out tree-sitter-rust pin multi-alt fields exceed 32 (measured overall_max_arity)",
                "layout_diff_note": "No shared layout constant existed yet; tranche 2 must bake MAX_BRANCH_ARITY=40. This is not a default per-grammar pin change (Decision 4).",
            })
        },
        "surfaces": surfaces.iter().map(surface_json).collect::<Vec<_>>(),
        "next_checklist_tranche": 2,
        "next_checklist_tranche_summary": "Shared alphabet constants + layout constructor with MAX_BRANCH_ARITY settled by this census",
    });
    let body = serde_json::to_string_pretty(&report).expect("report should serialize");
    fs::write(&path, format!("{body}\n")).unwrap_or_else(|error| {
        panic!("failed to write {}: {error}", path.display());
    });
}

#[test]
fn i5_shared_alphabet_arity_census_settles_max_branch_arity() {
    assert_eq!(CLAIM_BOUNDARY, "compiler_shared_alphabet_layout_only");
    assert_eq!(CLAIM_ID, "i5_shared_decision_token_alphabet_contract_v0");

    let surfaces = vec![
        go_inventory_census(),
        csharp_layout_census(),
        json_subset_census(),
        c_subset_census(),
        rust_held_out_census(),
    ];

    let overall_max = surfaces
        .iter()
        .map(|surface| surface.max_arity)
        .max()
        .unwrap_or(0);

    let design_default_ok = overall_max <= DESIGN_DEFAULT_MAX_BRANCH_ARITY as usize;
    write_report(&surfaces, overall_max, design_default_ok);

    // Surface-level sanity: each retained surface must load and report.
    for surface in &surfaces {
        assert!(
            !surface.source.is_empty(),
            "{} missing source provenance",
            surface.surface_id
        );
    }

    let by_id: BTreeMap<&str, &SurfaceCensus> =
        surfaces.iter().map(|s| (s.surface_id, s)).collect();

    // Expected order-of-magnitude anchors from fixture inspection (not pins).
    assert!(
        by_id["go_inventory"].max_arity >= 20,
        "Go Expr-family multi-alt fields should be well above 2; got {}",
        by_id["go_inventory"].max_arity
    );
    assert!(
        by_id["tree_sitter_json_subset"].max_arity >= 2,
        "JSON subset _value should be multi-alt"
    );
    assert!(
        by_id["tree_sitter_rust_held_out_pin"].max_arity >= 40,
        "held-out rust _expression subtypes should drive arity >= 40; got {}",
        by_id["tree_sitter_rust_held_out_pin"].max_arity
    );

    // Design default 32 is **not** sufficient once rust is in the census.
    assert!(
        !design_default_ok,
        "expected design default {DESIGN_DEFAULT_MAX_BRANCH_ARITY} to be insufficient; overall_max={overall_max}"
    );
    assert_eq!(
        overall_max, 40,
        "census expected overall max arity 40 from rust held-out pin; got {overall_max}"
    );

    // Settled constant for tranche 2 must cover the measured max.
    assert!(
        overall_max <= CENSUS_SETTLED_MAX_BRANCH_ARITY as usize,
        "census-settled MAX_BRANCH_ARITY={CENSUS_SETTLED_MAX_BRANCH_ARITY} must cover overall_max={overall_max}"
    );

    let written =
        fs::read_to_string(REPORT_PATH).expect("census report should exist under target/");
    let report: Value = serde_json::from_str(&written).expect("census report JSON");
    assert_eq!(report["artifact_kind"], ARTIFACT_KIND);
    assert_eq!(report["claim_boundary"], CLAIM_BOUNDARY);
    assert_eq!(report["overall_max_arity"], overall_max);
    assert_eq!(
        report["census_settled_max_branch_arity"],
        CENSUS_SETTLED_MAX_BRANCH_ARITY
    );
    assert_eq!(report["decision"], "documented_bump_required");
    assert_eq!(report["bump_note"]["to"], CENSUS_SETTLED_MAX_BRANCH_ARITY);
}
