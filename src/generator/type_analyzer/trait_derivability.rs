//! Trait-derivability analysis for generated Rust types.
//!
//! This module computes whether discovered schema types can safely derive
//! `Eq`, `Ord`, and `Hash` based on their members and dependencies. The result
//! is stored as analysis metadata so generation can choose derives
//! deterministically without re-walking schema structures.

use super::TypeAnalyzer;
use crate::TctResult;
use crate::generator::schema_discovery::{MemberType, TypeDiscovery, TypeKind};
use crate::generator::type_analysis::TraitDerivability;
use std::collections::HashSet;

impl TypeAnalyzer {
    /// Analyze trait derivability using mathematical rules
    pub(super) fn analyze_trait_derivability(&mut self) -> TctResult<()> {
        crate::log_debug!(
            "🧮 Analyzing trait derivability for {} types",
            self.discovery.discovered_types.len()
        );

        // Collect all type names to analyze
        let type_names: Vec<String> = self.discovery.discovered_types.keys().cloned().collect();

        // Analyze each type recursively (with cycle detection)
        let mut analyzing_stack = std::collections::HashSet::new();
        for type_name in type_names {
            self.analyze_type_recursive(&type_name, &mut analyzing_stack)?;
        }

        Ok(())
    }

    /// Recursively analyze a type's trait derivability, analyzing dependencies first
    pub(super) fn analyze_type_recursive(
        &mut self,
        type_name: &str,
        analyzing_stack: &mut std::collections::HashSet<String>,
    ) -> TctResult<()> {
        // If already analyzed, skip
        if self.analysis.trait_derivability.contains_key(type_name) {
            return Ok(());
        }

        // Check for cycles
        if analyzing_stack.contains(type_name) {
            crate::log_debug!(
                "⚠️  Cycle detected for type '{}', using conservative defaults",
                type_name
            );
            // For cycles, use conservative derivability (can't prove traits work)
            let derivability =
                TraitDerivability::algorithmic(false, false, false, "cycle detected".to_string());
            self.analysis
                .set_trait_derivability(type_name.to_string(), derivability);
            return Ok(());
        }

        // Mark as currently analyzing
        analyzing_stack.insert(type_name.to_string());

        // Get type discovery info
        let type_discovery = self.discovery.discovered_types.get(type_name).cloned();
        if let Some(type_discovery) = type_discovery {
            crate::log_debug!(
                "🔍 Analyzing traits for type '{}' (kind: {:?})",
                type_name,
                type_discovery.type_kind
            );

            let derivability = self.compute_trait_derivability(&type_discovery, analyzing_stack)?;

            crate::log_debug!("📊 Trait derivability for '{}':", type_name);
            crate::log_debug!(
                "   • Can derive Eq: {} (confidence: {:.1}%)",
                derivability.can_derive_eq,
                derivability.confidence * 100.0
            );
            crate::log_debug!(
                "   • Can derive Ord: {} (confidence: {:.1}%)",
                derivability.can_derive_ord,
                derivability.confidence * 100.0
            );
            crate::log_debug!(
                "   • Can derive Hash: {} (confidence: {:.1}%)",
                derivability.can_derive_hash,
                derivability.confidence * 100.0
            );
            crate::log_debug!("   • Reasoning: {}", derivability.reasoning);

            if !derivability.blocking_types.is_empty() {
                crate::log_debug!("   • Blocking types: {:?}", derivability.blocking_types);
            }

            self.analysis
                .set_trait_derivability(type_name.to_string(), derivability);
        }

        // Remove from analyzing stack
        analyzing_stack.remove(type_name);

        Ok(())
    }

    /// Compute trait derivability for a single type using mathematical rules
    pub(super) fn compute_trait_derivability(
        &mut self,
        type_discovery: &TypeDiscovery,
        analyzing_stack: &mut std::collections::HashSet<String>,
    ) -> TctResult<TraitDerivability> {
        let mut can_derive_eq = true;
        let mut can_derive_ord = true;
        let mut can_derive_hash = true;
        let _blocking_types: HashSet<String> = HashSet::new();
        let mut reasoning_parts = Vec::new();

        // Special case: Pattern validators contain HashMap and Regex - cannot derive Eq/Ord/Hash
        if type_discovery.type_name.contains("PatternValidator") {
            crate::log_debug!(
                "🔍 Detected pattern validator '{}' - applying special trait derivability rules",
                type_discovery.type_name
            );
            return Ok(TraitDerivability::algorithmic(
                false, // NOT Eq (HashMap is unordered collection)
                false, // NOT Ord (HashMap is unordered collection)
                false, // NOT Hash (HashMap and Regex don't implement Hash)
                "Pattern validator contains FxHashMap<String, T> and regex::Regex which cannot derive Eq/Ord/Hash".to_string()
            ));
        }

        // Apply mathematical rules based on type kind
        match &type_discovery.type_kind {
            TypeKind::Struct | TypeKind::NestedStruct => {
                reasoning_parts.push("struct type".to_string());

                // For structs: can derive trait if ALL members can derive trait
                for member in &type_discovery.members {
                    let member_derivability = self.analyze_member_derivability_recursive(
                        &member.member_type,
                        analyzing_stack,
                    )?;

                    if !member_derivability.can_derive_eq {
                        can_derive_eq = false;
                        reasoning_parts.push(format!("member {} cannot derive Eq", member.name));
                    }
                    if !member_derivability.can_derive_ord {
                        can_derive_ord = false;
                        reasoning_parts.push(format!("member {} cannot derive Ord", member.name));
                    }
                    if !member_derivability.can_derive_hash {
                        can_derive_hash = false;
                        reasoning_parts.push(format!("member {} cannot derive Hash", member.name));
                    }
                }
            }

            TypeKind::Enum => {
                reasoning_parts.push("enum type".to_string());

                // For enums: can derive trait if ALL variants can derive trait
                for member in &type_discovery.members {
                    let variant_derivability = self.analyze_member_derivability_recursive(
                        &member.member_type,
                        analyzing_stack,
                    )?;

                    if !variant_derivability.can_derive_eq {
                        can_derive_eq = false;
                        reasoning_parts.push(format!("variant {} cannot derive Eq", member.name));
                    }
                    if !variant_derivability.can_derive_ord {
                        can_derive_ord = false;
                        reasoning_parts.push(format!("variant {} cannot derive Ord", member.name));
                    }
                    if !variant_derivability.can_derive_hash {
                        can_derive_hash = false;
                        reasoning_parts.push(format!("variant {} cannot derive Hash", member.name));
                    }
                }
            }

            TypeKind::ArrayOfComplex => {
                reasoning_parts.push("Vec<T> type".to_string());
                // Vec<T> can derive trait if T can derive trait
                // Extract element type from schema.items and analyze it
                if let Some(items) = type_discovery.schema_object.get("items") {
                    if let Some(ref_path) = items.get("$ref").and_then(|v| v.as_str()) {
                        // Reference to another type - recursively analyze it first
                        if let Some(type_name) =
                            crate::schema_utils::local_definition_ref_target(ref_path)
                        {
                            let sanitized_name = crate::utils::to_upper_camel_case(&type_name);

                            // Recursively analyze the element type if not analyzed yet
                            self.analyze_type_recursive(&sanitized_name, analyzing_stack)?;

                            // Now look up its trait derivability (should exist after recursive analysis)
                            if let Some(element_analysis) =
                                self.analysis.trait_derivability.get(&sanitized_name)
                            {
                                can_derive_eq = element_analysis.can_derive_eq;
                                can_derive_ord = element_analysis.can_derive_ord;
                                can_derive_hash = element_analysis.can_derive_hash;
                                reasoning_parts
                                    .push(format!("element type {} derivability", sanitized_name));
                            } else {
                                // Fallback: if still not available, use conservative default
                                can_derive_eq = false;
                                can_derive_ord = false;
                                can_derive_hash = false;
                                reasoning_parts.push(format!(
                                    "element type {} not analyzable",
                                    sanitized_name
                                ));
                            }
                        }
                    } else if let Some(item_type) = items.get("type").and_then(|v| v.as_str()) {
                        // Primitive type - check derivability
                        match item_type {
                            "string" | "integer" | "boolean" => {
                                // Primitives can derive all traits
                                reasoning_parts
                                    .push(format!("primitive element type {}", item_type));
                            }
                            "number" => {
                                // f64 cannot derive Eq, Ord
                                can_derive_eq = false;
                                can_derive_ord = false;
                                reasoning_parts.push("f64 element type (no Eq/Ord)".to_string());
                            }
                            _ => {
                                reasoning_parts.push(format!("unknown element type {}", item_type));
                            }
                        }
                    }
                }
            }

            TypeKind::Reference { target } => {
                reasoning_parts.push(format!("reference to {}", target));
                // References inherit derivability from target type
                // Recursively analyze the target if not analyzed yet
                self.analyze_type_recursive(target, analyzing_stack)?;

                // Now look up its trait derivability
                if let Some(target_analysis) = self.analysis.trait_derivability.get(target) {
                    can_derive_eq = target_analysis.can_derive_eq;
                    can_derive_ord = target_analysis.can_derive_ord;
                    can_derive_hash = target_analysis.can_derive_hash;
                } else {
                    // Conservative fallback
                    can_derive_eq = false;
                    can_derive_ord = false;
                    can_derive_hash = false;
                }
            }

            TypeKind::PrimitiveWrapper { primitive_type } => {
                reasoning_parts.push(format!("primitive wrapper for {}", primitive_type));
                // Primitive wrappers inherit derivability from the wrapped primitive type
                match primitive_type.as_str() {
                    "String" | "i64" | "JsonInteger" | "bool" => {
                        // String, i64, bool all derive Eq, Ord, Hash
                        can_derive_eq = true;
                        can_derive_ord = true;
                        can_derive_hash = true;
                        reasoning_parts.push(format!("{} supports all traits", primitive_type));
                    }
                    "f64" => {
                        // f64 only derives PartialEq (no Eq, Ord, Hash)
                        can_derive_eq = false;
                        can_derive_ord = false;
                        can_derive_hash = false;
                        reasoning_parts.push("f64 does not support Eq/Ord/Hash".to_string());
                    }
                    _ => {
                        // Unknown primitive type, be conservative
                        can_derive_eq = false;
                        can_derive_ord = false;
                        can_derive_hash = false;
                        reasoning_parts.push(format!("unknown primitive type {}", primitive_type));
                    }
                }
            }

            TypeKind::MinimalObjectWrapper => {
                reasoning_parts.push("minimal object wrapper for serde_json::Value".to_string());
                // serde_json::Value does not support Eq, Ord, Hash due to potential floating point values
                can_derive_eq = false;
                can_derive_ord = false;
                can_derive_hash = false;
                reasoning_parts.push("serde_json::Value does not support Eq/Ord/Hash".to_string());
            }
        }

        let reasoning = if reasoning_parts.is_empty() {
            "no specific constraints".to_string()
        } else {
            reasoning_parts.join(", ")
        };

        Ok(TraitDerivability::algorithmic(
            can_derive_eq,
            can_derive_ord,
            can_derive_hash,
            reasoning,
        ))
    }

    /// Analyze derivability of a member type with recursive analysis support
    pub(super) fn analyze_member_derivability_recursive(
        &mut self,
        member_type: &MemberType,
        analyzing_stack: &mut std::collections::HashSet<String>,
    ) -> TctResult<TraitDerivability> {
        match member_type {
            MemberType::Primitive(rust_type) => Ok(self.analyze_primitive_derivability(rust_type)),

            MemberType::TypeReference(type_name) => {
                // Recursively analyze the referenced type if not analyzed yet
                self.analyze_type_recursive(type_name, analyzing_stack)?;

                // Look up analysis for referenced type
                if let Some(derivability) = self.analysis.trait_derivability.get(type_name) {
                    Ok(derivability.clone())
                } else {
                    // Conservative fallback
                    Ok(TraitDerivability::heuristic(
                        false,
                        false,
                        false,
                        format!("unanalyzed type reference: {}", type_name),
                        0.5,
                    ))
                }
            }

            MemberType::Container { container, inner } => {
                let inner_derivability = self.analyze_primitive_derivability(inner);

                // Container derivability follows inner type rules
                match container.as_str() {
                    "Vec" | "Option" => Ok(inner_derivability),
                    _ => Ok(TraitDerivability::heuristic(
                        false,
                        false,
                        false,
                        format!("unknown container type: {}", container),
                        0.5,
                    )),
                }
            }

            MemberType::ComplexContainer { container, inner } => {
                // Recursively analyze the inner complex type
                self.analyze_type_recursive(inner, analyzing_stack)?;

                let inner_derivability =
                    if let Some(analysis) = self.analysis.trait_derivability.get(inner) {
                        analysis.clone()
                    } else {
                        TraitDerivability::heuristic(
                            false,
                            false,
                            false,
                            format!("unanalyzed complex inner type: {}", inner),
                            0.5,
                        )
                    };

                // Container derivability follows inner type rules
                match container.as_str() {
                    "Vec" | "Option" => Ok(inner_derivability),
                    _ => Ok(TraitDerivability::heuristic(
                        false,
                        false,
                        false,
                        format!("unknown complex container: {}", container),
                        0.5,
                    )),
                }
            }

            MemberType::HashMap { value, .. } => {
                // FxHashMap<String, T> derivability depends on value type T
                let _value_derivability = self.analyze_primitive_derivability(value);
                // HashMap itself is not Eq/Ord/Hash (due to unordered nature), but can be Debug/Clone/PartialEq
                Ok(TraitDerivability::algorithmic(
                    false, // HashMap is not Eq (values could be f64, also HashMap is unordered)
                    false, // HashMap is not Ord (unordered collection)
                    false, // HashMap is not Hash (HashMap type itself doesn't implement Hash)
                    format!(
                        "FxHashMap<String, {}> - unordered collection, no Hash trait",
                        value
                    ),
                ))
            }

            MemberType::PatternProperties { .. } => {
                // Pattern properties contain validation logic and dynamic values
                // Conservative derivability since validators may contain complex types
                Ok(TraitDerivability::algorithmic(
                    false, // Not Eq (complex validation structures)
                    false, // Not Ord (complex validation structures)
                    false, // Not Hash (contains validation logic)
                    "PatternProperties - complex validation structure".to_string(),
                ))
            }
        }
    }

    /// Analyze derivability of primitive Rust types
    pub(super) fn analyze_primitive_derivability(&self, rust_type: &str) -> TraitDerivability {
        match rust_type {
            // Integer types: derive all traits
            "i8" | "i16" | "i32" | "i64" | "i128" | "isize" | "u8" | "u16" | "u32" | "u64"
            | "u128" | "usize" | "JsonInteger" => TraitDerivability::algorithmic(
                true,
                true,
                true,
                format!("{} is an integer type", rust_type),
            ),

            // Boolean: derives all traits
            "bool" => TraitDerivability::algorithmic(
                true,
                true,
                true,
                "bool derives all traits".to_string(),
            ),

            // String: derives Eq and Hash but not Ord (usually)
            "String" => TraitDerivability::algorithmic(
                true,
                true,
                true,
                "String derives Eq, Ord, Hash".to_string(),
            ),

            // Floating point: cannot derive Eq, Ord (due to NaN)
            "f32" | "f64" => TraitDerivability::algorithmic(
                false,
                false,
                false,
                format!("{} cannot derive Eq/Ord due to NaN", rust_type),
            ),

            // serde_json::Value: cannot reliably derive traits
            "serde_json::Value" => TraitDerivability::algorithmic(
                false,
                false,
                false,
                "serde_json::Value cannot reliably derive traits".to_string(),
            ),

            // Unit type: derives all traits
            "()" => TraitDerivability::algorithmic(
                true,
                true,
                true,
                "unit type () derives all traits".to_string(),
            ),

            // Unknown types: conservative approach
            _ => TraitDerivability::heuristic(
                false,
                false,
                false,
                format!("unknown primitive type: {}", rust_type),
                0.3,
            ),
        }
    }
}
