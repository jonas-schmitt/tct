//! Structural schema-evolution comparison records.

use super::{
    BitPackedPresenceMetadata, BpeArtifactCompatibility, BpeArtifactDiff,
    StructuralCompatibilityAssessment, StructuralCompatibilityDecision, StructuralEmissionMetadata,
    StructuralRootKind, StructuralSelectorEmission, StructuralSelectorKind,
    StructuralStrictStreamShape, StructuralTokenEntry, StructuralTokenMap, StructuralTokenRole,
    StructuralValueEmission, StructuralValueWidth, TokenIdLayout, sha256_json,
};
use crate::compiler::CompilerPassArtifacts;
use crate::error::{TctError, TctResult};
use crate::generator::encoding_decision::EncodingDecision;
use crate::generator::schema_discovery::{MemberInfo, MemberType, TypeDiscovery, TypeKind};
use serde_json::Value;

pub(super) fn summarize_structural_token_map(
    artifacts: &CompilerPassArtifacts,
) -> TctResult<StructuralTokenMap> {
    let root_type = root_type_discovery(artifacts)?;
    let root_schema = Value::Object(root_type.schema_object.clone());
    let schema_sha256 = sha256_json(&root_schema, "structural_schema_sha256")?;

    if let Some(value) = root_type.schema_object.get("const") {
        let value_sha256 = sha256_json(value, "structural_const_value_sha256")?;
        return Ok(StructuralTokenMap {
            root_kind: StructuralRootKind::Const,
            schema_sha256,
            entries: vec![StructuralTokenEntry {
                path: "#/const".to_string(),
                generated_order: Some(0),
                role: StructuralTokenRole::ZeroTokenConst,
                required: None,
                member_type_path: Some(root_type.type_name.clone()),
                encoding_decision: Some(encoding_decision_label(artifacts, &root_type.type_name)),
                emission: zero_token_emission(Some(encoding_decision_label(
                    artifacts,
                    &root_type.type_name,
                ))),
                schema_sha256: value_sha256.clone(),
                value_sha256: Some(value_sha256),
            }],
        });
    }

    if root_type.schema_object.get("enum").is_some() && root_type.type_kind == TypeKind::Enum {
        let entries = root_type
            .members
            .iter()
            .enumerate()
            .map(|(index, member)| enum_structural_entry(index, root_type, member, artifacts))
            .collect::<TctResult<Vec<_>>>()?;
        return Ok(StructuralTokenMap {
            root_kind: StructuralRootKind::Enum,
            schema_sha256,
            entries,
        });
    }

    if matches!(
        root_type.type_kind,
        TypeKind::Struct | TypeKind::NestedStruct
    ) {
        let entries = root_type
            .members
            .iter()
            .enumerate()
            .map(|(index, member)| object_structural_entry(index, root_type, member, artifacts))
            .collect::<TctResult<Vec<_>>>()?;
        return Ok(StructuralTokenMap {
            root_kind: StructuralRootKind::Object,
            schema_sha256,
            entries,
        });
    }

    Ok(StructuralTokenMap {
        root_kind: StructuralRootKind::Opaque,
        schema_sha256,
        entries: Vec::new(),
    })
}

fn root_type_discovery(artifacts: &CompilerPassArtifacts) -> TctResult<&TypeDiscovery> {
    let root_type_name = artifacts
        .schema_discovery
        .root_info
        .as_ref()
        .map(|root| root.type_name.as_str())
        .ok_or_else(|| TctError::CodeGeneration {
            message: "DiscoveryPass did not record root schema information".to_string(),
            schema_path: "#/".to_string(),
            suggestion: "Ensure schema discovery completed before schema-evolution reporting"
                .to_string(),
            phase: crate::error::GenerationPhase::SchemaAnalysis,
        })?;

    artifacts
        .schema_discovery
        .discovered_types
        .get(root_type_name)
        .ok_or_else(|| TctError::CodeGeneration {
            message: format!(
                "DiscoveryPass did not record root type '{root_type_name}' for schema-evolution reporting"
            ),
            schema_path: "#/".to_string(),
            suggestion: "Ensure the root type is discovered before token-ID layout reporting"
                .to_string(),
            phase: crate::error::GenerationPhase::SchemaAnalysis,
        })
}

fn enum_structural_entry(
    index: usize,
    root_type: &TypeDiscovery,
    member: &MemberInfo,
    artifacts: &CompilerPassArtifacts,
) -> TctResult<StructuralTokenEntry> {
    let value = member
        .schema
        .get("__original_enum_value")
        .cloned()
        .unwrap_or_else(|| Value::String(member.name.clone()));
    let value_sha256 = sha256_json(&value, "structural_enum_value_sha256")?;
    Ok(StructuralTokenEntry {
        path: format!("#/enum/{index}"),
        generated_order: Some(index),
        role: StructuralTokenRole::EnumVariant,
        required: None,
        member_type_path: Some(root_type.type_name.clone()),
        encoding_decision: Some(encoding_decision_label(artifacts, &root_type.type_name)),
        emission: enum_member_emission(root_type, artifacts),
        schema_sha256: value_sha256.clone(),
        value_sha256: Some(value_sha256),
    })
}

fn object_structural_entry(
    index: usize,
    parent_type: &TypeDiscovery,
    member: &MemberInfo,
    artifacts: &CompilerPassArtifacts,
) -> TctResult<StructuralTokenEntry> {
    let property_schema = Value::Object(member.schema.clone());
    let member_type_path = member_type_path(member);
    let encoding_decision = member_type_path
        .as_deref()
        .map(|type_path| encoding_decision_label(artifacts, type_path));
    let role = object_member_role(member, artifacts);
    Ok(StructuralTokenEntry {
        path: format!("#/properties/{}", json_pointer_escape(&member.name)),
        generated_order: Some(index),
        role,
        required: Some(member.required),
        member_type_path,
        encoding_decision,
        emission: object_member_emission(parent_type, member, role, artifacts),
        schema_sha256: sha256_json(&property_schema, "structural_property_schema_sha256")?,
        value_sha256: member
            .schema
            .get("const")
            .map(|value| sha256_json(value, "structural_property_const_sha256"))
            .transpose()?,
    })
}

fn enum_member_emission(
    root_type: &TypeDiscovery,
    artifacts: &CompilerPassArtifacts,
) -> StructuralEmissionMetadata {
    let decision = type_encoding_decision(artifacts, &root_type.type_name);
    let fixed_token_count = match decision {
        Some(EncodingDecision::SingleVariantEnum { .. }) => Some(0),
        Some(EncodingDecision::EnumCodec { ref codec_spec, .. }) => Some(codec_spec.token_width),
        Some(EncodingDecision::EnumIndex { .. }) | Some(EncodingDecision::Standard) | None => {
            Some(1)
        }
        _ => None,
    };
    StructuralEmissionMetadata {
        parent_encoding_decision: Some(encoding_decision_label(artifacts, &root_type.type_name)),
        selector: StructuralSelectorEmission {
            kind: StructuralSelectorKind::EnumVariantToken,
            fixed_token_count,
            bit_packed_presence: None,
        },
        value: StructuralValueEmission {
            emits_when_present: false,
            width: StructuralValueWidth::Zero,
            fixed_token_count: Some(0),
        },
        strict_stream_shape: StructuralStrictStreamShape::EnumSelector,
        catch_all_map: false,
        nested_object: false,
    }
}

fn zero_token_emission(parent_encoding_decision: Option<String>) -> StructuralEmissionMetadata {
    StructuralEmissionMetadata {
        parent_encoding_decision,
        selector: StructuralSelectorEmission {
            kind: StructuralSelectorKind::None,
            fixed_token_count: Some(0),
            bit_packed_presence: None,
        },
        value: StructuralValueEmission {
            emits_when_present: false,
            width: StructuralValueWidth::Zero,
            fixed_token_count: Some(0),
        },
        strict_stream_shape: StructuralStrictStreamShape::ZeroTokens,
        catch_all_map: false,
        nested_object: false,
    }
}

fn object_member_emission(
    parent_type: &TypeDiscovery,
    member: &MemberInfo,
    role: StructuralTokenRole,
    artifacts: &CompilerPassArtifacts,
) -> StructuralEmissionMetadata {
    if role == StructuralTokenRole::ZeroTokenConst {
        let mut emission = zero_token_emission(Some(encoding_decision_label(
            artifacts,
            &parent_type.type_name,
        )));
        emission.catch_all_map = member_is_catch_all_map(member);
        emission.nested_object = member_is_nested_object(member, artifacts);
        return emission;
    }

    let parent_decision = type_encoding_decision(artifacts, &parent_type.type_name);
    let bit_packed_presence =
        bit_packed_presence_metadata(parent_type, member, parent_decision.as_ref());
    let selector = object_member_selector(role, bit_packed_presence);
    let strict_stream_shape = match selector.kind {
        StructuralSelectorKind::None => StructuralStrictStreamShape::RequiredValueTokens,
        StructuralSelectorKind::OptionPresenceFlag => {
            StructuralStrictStreamShape::PresenceFlagThenValueIfPresent
        }
        StructuralSelectorKind::OptionalFieldStateToken => {
            StructuralStrictStreamShape::OptionalStateThenValueIfPresent
        }
        StructuralSelectorKind::BitPackedPresenceBit => {
            StructuralStrictStreamShape::SharedPresenceMaskThenValueIfPresent
        }
        StructuralSelectorKind::EnumVariantToken => StructuralStrictStreamShape::EnumSelector,
        StructuralSelectorKind::Opaque => StructuralStrictStreamShape::Opaque,
    };

    StructuralEmissionMetadata {
        parent_encoding_decision: Some(encoding_decision_label(artifacts, &parent_type.type_name)),
        selector,
        value: member_value_emission(member, artifacts),
        strict_stream_shape,
        catch_all_map: member_is_catch_all_map(member),
        nested_object: member_is_nested_object(member, artifacts),
    }
}

fn object_member_selector(
    role: StructuralTokenRole,
    bit_packed_presence: Option<BitPackedPresenceMetadata>,
) -> StructuralSelectorEmission {
    match role {
        StructuralTokenRole::RequiredValue | StructuralTokenRole::ZeroTokenConst => {
            StructuralSelectorEmission {
                kind: StructuralSelectorKind::None,
                fixed_token_count: Some(0),
                bit_packed_presence: None,
            }
        }
        StructuralTokenRole::OptionPresenceAndValue => {
            if let Some(bit_packed_presence) = bit_packed_presence {
                StructuralSelectorEmission {
                    kind: StructuralSelectorKind::BitPackedPresenceBit,
                    fixed_token_count: None,
                    bit_packed_presence: Some(bit_packed_presence),
                }
            } else {
                StructuralSelectorEmission {
                    kind: StructuralSelectorKind::OptionPresenceFlag,
                    fixed_token_count: Some(1),
                    bit_packed_presence: None,
                }
            }
        }
        StructuralTokenRole::OptionalFieldStateAndValue => StructuralSelectorEmission {
            kind: StructuralSelectorKind::OptionalFieldStateToken,
            fixed_token_count: Some(1),
            bit_packed_presence: None,
        },
        StructuralTokenRole::EnumVariant => StructuralSelectorEmission {
            kind: StructuralSelectorKind::EnumVariantToken,
            fixed_token_count: Some(1),
            bit_packed_presence: None,
        },
        StructuralTokenRole::Opaque => StructuralSelectorEmission {
            kind: StructuralSelectorKind::Opaque,
            fixed_token_count: None,
            bit_packed_presence: None,
        },
    }
}

fn bit_packed_presence_metadata(
    parent_type: &TypeDiscovery,
    member: &MemberInfo,
    parent_decision: Option<&EncodingDecision>,
) -> Option<BitPackedPresenceMetadata> {
    let EncodingDecision::BitPackedStruct {
        optional_field_count,
        presence_tokens_needed,
    } = parent_decision?
    else {
        return None;
    };
    if !member.uses_option_presence_flag() {
        return None;
    }

    let optional_field_index = parent_type
        .members
        .iter()
        .filter(|candidate| candidate.uses_option_presence_flag())
        .position(|candidate| candidate.name == member.name)?;
    if optional_field_index >= *optional_field_count {
        return None;
    }

    Some(BitPackedPresenceMetadata {
        optional_field_index,
        presence_mask_index: optional_field_index / 32,
        bit_index: optional_field_index % 32,
        presence_mask_count: *presence_tokens_needed,
    })
}

fn member_value_emission(
    member: &MemberInfo,
    artifacts: &CompilerPassArtifacts,
) -> StructuralValueEmission {
    let Some(type_path) = member_type_path(member) else {
        return StructuralValueEmission {
            emits_when_present: true,
            width: StructuralValueWidth::Opaque,
            fixed_token_count: None,
        };
    };

    match type_encoding_decision(artifacts, &type_path) {
        Some(EncodingDecision::SingleVariantEnum { .. })
        | Some(EncodingDecision::ConstantInteger { .. }) => StructuralValueEmission {
            emits_when_present: false,
            width: StructuralValueWidth::Zero,
            fixed_token_count: Some(0),
        },
        Some(EncodingDecision::EnumIndex { .. })
        | Some(EncodingDecision::BoundedInteger { .. })
        | Some(EncodingDecision::BoundedIntegerOffset { .. })
        | Some(EncodingDecision::PatternOptimized { .. }) => StructuralValueEmission {
            emits_when_present: true,
            width: StructuralValueWidth::Fixed,
            fixed_token_count: Some(1),
        },
        Some(EncodingDecision::EnumCodec { codec_spec, .. })
        | Some(EncodingDecision::BoundedIntegerCodec { codec_spec, .. }) => {
            StructuralValueEmission {
                emits_when_present: true,
                width: StructuralValueWidth::Fixed,
                fixed_token_count: Some(codec_spec.token_width),
            }
        }
        Some(EncodingDecision::FixedSizeArray { .. })
        | Some(EncodingDecision::FormatOptimized { .. })
        | Some(EncodingDecision::BitPackedStruct { .. }) => StructuralValueEmission {
            emits_when_present: true,
            width: StructuralValueWidth::SchemaDependent,
            fixed_token_count: None,
        },
        Some(EncodingDecision::Standard) | None => standard_member_value_emission(member),
    }
}

fn standard_member_value_emission(member: &MemberInfo) -> StructuralValueEmission {
    match &member.member_type {
        MemberType::Primitive(type_name) if type_name == "bool" => StructuralValueEmission {
            emits_when_present: true,
            width: StructuralValueWidth::Fixed,
            fixed_token_count: Some(1),
        },
        MemberType::Primitive(type_name) if type_name == "()" => StructuralValueEmission {
            emits_when_present: false,
            width: StructuralValueWidth::Zero,
            fixed_token_count: Some(0),
        },
        MemberType::Primitive(type_name) if primitive_is_variable_width(type_name) => {
            StructuralValueEmission {
                emits_when_present: true,
                width: StructuralValueWidth::Variable,
                fixed_token_count: None,
            }
        }
        MemberType::TypeReference(_)
        | MemberType::ComplexContainer { .. }
        | MemberType::PatternProperties { .. } => StructuralValueEmission {
            emits_when_present: true,
            width: StructuralValueWidth::SchemaDependent,
            fixed_token_count: None,
        },
        MemberType::Container { .. } | MemberType::HashMap { .. } => StructuralValueEmission {
            emits_when_present: true,
            width: StructuralValueWidth::Variable,
            fixed_token_count: None,
        },
        MemberType::Primitive(_) => StructuralValueEmission {
            emits_when_present: true,
            width: StructuralValueWidth::Opaque,
            fixed_token_count: None,
        },
    }
}

fn primitive_is_variable_width(type_name: &str) -> bool {
    matches!(
        type_name,
        "String" | "i64" | "u64" | "f64" | "serde_json::Value"
    ) || type_name.contains("String")
        || type_name.contains("serde_json::Value")
}

fn member_is_catch_all_map(member: &MemberInfo) -> bool {
    matches!(
        member.member_type,
        MemberType::HashMap { .. } | MemberType::PatternProperties { .. }
    )
}

fn member_is_nested_object(member: &MemberInfo, artifacts: &CompilerPassArtifacts) -> bool {
    match &member.member_type {
        MemberType::TypeReference(type_name) => discovered_type_is_object(type_name, artifacts),
        MemberType::ComplexContainer { inner, .. } => discovered_type_is_object(inner, artifacts),
        MemberType::Primitive(_)
        | MemberType::Container { .. }
        | MemberType::HashMap { .. }
        | MemberType::PatternProperties { .. } => false,
    }
}

fn discovered_type_is_object(type_name: &str, artifacts: &CompilerPassArtifacts) -> bool {
    artifacts
        .schema_discovery
        .discovered_types
        .get(type_name)
        .is_some_and(|discovery| {
            matches!(
                discovery.type_kind,
                TypeKind::Struct | TypeKind::NestedStruct | TypeKind::MinimalObjectWrapper
            )
        })
}

fn member_type_path(member: &MemberInfo) -> Option<String> {
    match &member.member_type {
        MemberType::Primitive(type_name) | MemberType::TypeReference(type_name) => {
            Some(type_name.clone())
        }
        MemberType::Container { container, inner } => Some(format!("{container}<{inner}>")),
        MemberType::ComplexContainer { container, inner } => Some(format!("{container}<{inner}>")),
        MemberType::HashMap { key, value } => Some(format!("HashMap<{key},{value}>")),
        MemberType::PatternProperties { validators } => {
            (!validators.is_empty()).then(|| validators.join("|"))
        }
    }
}

fn encoding_decision_label(artifacts: &CompilerPassArtifacts, type_path: &str) -> String {
    artifacts
        .encoding_strategy
        .as_ref()
        .map(|strategy| strategy.get_decision(type_path).strategy_name().to_string())
        .unwrap_or_else(|| "standard".to_string())
}

fn object_member_role(
    member: &MemberInfo,
    artifacts: &CompilerPassArtifacts,
) -> StructuralTokenRole {
    if member.required && member_is_generated_zero_token_const(member, artifacts) {
        StructuralTokenRole::ZeroTokenConst
    } else if member.required {
        StructuralTokenRole::RequiredValue
    } else if member.uses_optional_field_encoding() {
        StructuralTokenRole::OptionalFieldStateAndValue
    } else {
        StructuralTokenRole::OptionPresenceAndValue
    }
}

fn member_is_generated_zero_token_const(
    member: &MemberInfo,
    artifacts: &CompilerPassArtifacts,
) -> bool {
    let Some(const_value) = member.schema.get("const") else {
        return false;
    };
    let Some(type_path) = member_type_path(member) else {
        return false;
    };

    if const_value.is_string() {
        return type_encoding_decision(artifacts, &type_path).is_some_and(|decision| {
            matches!(decision, EncodingDecision::SingleVariantEnum { .. })
        });
    }

    artifacts
        .schema_discovery
        .discovered_types
        .get(&type_path)
        .is_some_and(|type_discovery| {
            matches!(
                type_discovery.type_kind,
                TypeKind::PrimitiveWrapper { .. } | TypeKind::Struct | TypeKind::NestedStruct
            ) && type_discovery.schema_object.get("const").is_some()
        })
}

fn type_encoding_decision(
    artifacts: &CompilerPassArtifacts,
    type_path: &str,
) -> Option<EncodingDecision> {
    artifacts
        .encoding_strategy
        .as_ref()
        .map(|strategy| strategy.get_decision(type_path))
}

pub(super) fn assess_structural_compatibility(
    old: &StructuralTokenMap,
    new: &StructuralTokenMap,
) -> StructuralCompatibilityAssessment {
    if old == new {
        return StructuralCompatibilityAssessment {
            decision: StructuralCompatibilityDecision::Unchanged,
            changed_paths: Vec::new(),
            reasons: vec!["structural token map is unchanged".to_string()],
        };
    }

    if old.root_kind != new.root_kind {
        return StructuralCompatibilityAssessment {
            decision: StructuralCompatibilityDecision::BreakingChange,
            changed_paths: changed_structural_paths(old, new),
            reasons: vec!["root structural token kind changed".to_string()],
        };
    }

    match old.root_kind {
        StructuralRootKind::Enum => assess_enum_structural_compatibility(old, new),
        StructuralRootKind::Object => assess_object_structural_compatibility(old, new),
        StructuralRootKind::Const => StructuralCompatibilityAssessment {
            decision: StructuralCompatibilityDecision::BreakingChange,
            changed_paths: changed_structural_paths(old, new),
            reasons: vec!["root const value changed".to_string()],
        },
        StructuralRootKind::Opaque => StructuralCompatibilityAssessment {
            decision: StructuralCompatibilityDecision::NotProven,
            changed_paths: changed_structural_paths(old, new),
            reasons: vec![
                "structural token compatibility is not proven for this schema shape".to_string(),
            ],
        },
    }
}

fn assess_enum_structural_compatibility(
    old: &StructuralTokenMap,
    new: &StructuralTokenMap,
) -> StructuralCompatibilityAssessment {
    if entries_are_prefix(&old.entries, &new.entries) {
        return StructuralCompatibilityAssessment {
            decision: StructuralCompatibilityDecision::AdditiveCompatible,
            changed_paths: new.entries[old.entries.len()..]
                .iter()
                .map(|entry| entry.path.clone())
                .collect(),
            reasons: vec![
                "new root enum variants are appended after all existing variants; existing variant token IDs are preserved"
                    .to_string(),
            ],
        };
    }

    StructuralCompatibilityAssessment {
        decision: StructuralCompatibilityDecision::BreakingChange,
        changed_paths: changed_structural_paths(old, new),
        reasons: vec![
            "existing root enum variant order or value changed, so schema-local token meanings changed"
                .to_string(),
        ],
    }
}

fn assess_object_structural_compatibility(
    old: &StructuralTokenMap,
    new: &StructuralTokenMap,
) -> StructuralCompatibilityAssessment {
    if !entries_are_prefix(&old.entries, &new.entries) {
        return StructuralCompatibilityAssessment {
            decision: StructuralCompatibilityDecision::BreakingChange,
            changed_paths: changed_structural_paths(old, new),
            reasons: vec![
                "existing object member order, role, requiredness, or schema changed".to_string(),
            ],
        };
    }

    let added_entries = &new.entries[old.entries.len()..];
    if !added_entries.is_empty()
        && added_entries.iter().all(|entry| {
            entry.required == Some(true) && entry.role == StructuralTokenRole::ZeroTokenConst
        })
    {
        return StructuralCompatibilityAssessment {
            decision: StructuralCompatibilityDecision::AdditiveCompatible,
            changed_paths: added_entries
                .iter()
                .map(|entry| entry.path.clone())
                .collect(),
            reasons: vec![
                "new object members are appended required zero-token const fields; existing object member token order and stream shape are preserved"
                    .to_string(),
            ],
        };
    }

    if added_entries.iter().any(|entry| {
        entry.required == Some(true) && entry.role != StructuralTokenRole::ZeroTokenConst
    }) {
        return StructuralCompatibilityAssessment {
            decision: StructuralCompatibilityDecision::BreakingChange,
            changed_paths: added_entries
                .iter()
                .map(|entry| entry.path.clone())
                .collect(),
            reasons: vec![
                "new required object members add nonzero token content that old token streams cannot provide"
                    .to_string(),
            ],
        };
    }

    StructuralCompatibilityAssessment {
        decision: StructuralCompatibilityDecision::NotProven,
        changed_paths: added_entries
            .iter()
            .map(|entry| entry.path.clone())
            .collect(),
        reasons: vec![
            "object member additions that emit optional state or value tokens are not promoted because old strict token streams do not contain those trailing tokens"
                .to_string(),
        ],
    }
}

pub(super) fn structural_promotion_allowed(
    old: &TokenIdLayout,
    new: &TokenIdLayout,
    bpe_artifact_diff: &BpeArtifactDiff,
) -> bool {
    old.root_type_name == new.root_type_name
        && old.optimization_level == new.optimization_level
        && old.optimization_pass_overrides == new.optimization_pass_overrides
        && old.string_encoding == new.string_encoding
        && old.max_base_vocab == new.max_base_vocab
        && matches!(
            bpe_artifact_diff.compatibility,
            BpeArtifactCompatibility::NoArtifact | BpeArtifactCompatibility::Unchanged
        )
}

fn entries_are_prefix(old: &[StructuralTokenEntry], new: &[StructuralTokenEntry]) -> bool {
    old.len() <= new.len() && old.iter().zip(new.iter()).all(|(old, new)| old == new)
}

fn changed_structural_paths(old: &StructuralTokenMap, new: &StructuralTokenMap) -> Vec<String> {
    let mut paths = Vec::new();
    let max_len = old.entries.len().max(new.entries.len());
    for index in 0..max_len {
        match (old.entries.get(index), new.entries.get(index)) {
            (Some(old), Some(new)) if old == new => {}
            (Some(old), Some(new)) => {
                paths.push(old.path.clone());
                if old.path != new.path {
                    paths.push(new.path.clone());
                }
            }
            (Some(old), None) => paths.push(old.path.clone()),
            (None, Some(new)) => paths.push(new.path.clone()),
            (None, None) => {}
        }
    }
    paths.sort();
    paths.dedup();
    paths
}

fn json_pointer_escape(value: &str) -> String {
    value.replace('~', "~0").replace('/', "~1")
}
