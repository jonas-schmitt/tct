//! Per-type generation dispatch for discovered schema types.
//!
//! This module keeps the high-level `CodeGenerator` routing decisions in one
//! place: apply optimization-owned member refinements, skip already-generated
//! roots, and delegate concrete structs, enums, aliases, bounded integers, and
//! catch-all maps to their specialized generators.

use super::CodeGenerator;
use crate::TctResult;
use crate::generator::bounded_integer_generator::BoundedIntegerGenerator;
use crate::generator::encoding_decision::{EncodingDecision, EncodingStrategy};
use crate::generator::enum_generator::EnumGenerator;
use crate::generator::generation_output::TypeCodeGeneration;
use crate::generator::schema_discovery::{MemberType, SchemaDiscovery, TypeDiscovery, TypeKind};
use crate::generator::struct_generator::StructGenerator;
use crate::generator::type_alias_generator::TypeAliasGenerator;
use crate::generator::type_analysis::TraitDerivability;

impl CodeGenerator {
    pub(super) fn apply_member_type_refinements(
        mut discovery: SchemaDiscovery,
        encoding_strategy: Option<&EncodingStrategy>,
    ) -> SchemaDiscovery {
        let Some(encoding_strategy) = encoding_strategy else {
            return discovery;
        };

        for (member_path, refinement) in &encoding_strategy.member_type_refinements {
            let Some(type_discovery) = discovery.discovered_types.get_mut(&member_path.type_name)
            else {
                crate::log_debug!(
                    "⚠️  Ignoring member refinement for unknown type '{}'",
                    member_path.type_name
                );
                continue;
            };

            let Some(member) = type_discovery
                .members
                .iter_mut()
                .find(|member| member.name == member_path.member_name)
            else {
                crate::log_debug!(
                    "⚠️  Ignoring member refinement for unknown member '{}.{}'",
                    member_path.type_name,
                    member_path.member_name
                );
                continue;
            };

            let refined_member_type = match refinement {
                crate::generator::encoding_decision::MemberTypeRefinement::HashMap {
                    key,
                    value,
                } => MemberType::HashMap {
                    key: key.clone(),
                    value: value.clone(),
                },
            };

            if member.member_type != refined_member_type {
                crate::log_debug!(
                    "🔧 Applying member type refinement for '{}.{}': {:?} → {:?}",
                    member_path.type_name,
                    member_path.member_name,
                    member.member_type,
                    refined_member_type
                );
                member.member_type = refined_member_type;
            }
        }

        discovery
    }

    /// Generate code for a single type
    pub(super) fn generate_type(&mut self, type_name: &str) -> TctResult<()> {
        crate::log_debug!("🔍 Generating type '{}' details:", type_name);

        // Check if this type was already generated to prevent duplicates
        if self.generation.has_type(type_name) {
            crate::log_debug!("   • Type '{}' already generated, skipping", type_name);
            return Ok(());
        }

        // Check if this is the root type and will be generated as root composition
        if let Some(ref root_info) = self.discovery.root_info
            && root_info.type_name == type_name
            && root_info.is_composition
        {
            crate::log_debug!(
                "   • Type '{}' is root composition, will be generated separately",
                type_name
            );
            return Ok(());
        }

        // Get discovery and analysis data for this type - clone to avoid borrow conflicts
        let type_discovery = self
            .discovery
            .get_type(type_name)
            .ok_or_else(|| crate::error::TctError::CodeGeneration {
                message: format!("Type {} not found in discovery results", type_name),
                schema_path: format!("#/types/{}", type_name),
                suggestion: "Ensure DiscoveryPass discovered all types correctly".to_string(),
                phase: crate::error::GenerationPhase::TypeGeneration,
            })?
            .clone();

        let trait_derivability = self.analysis.trait_derivability.get(type_name).cloned();

        crate::log_debug!("   • Type kind: {:?}", type_discovery.type_kind);
        crate::log_debug!("   • Member count: {}", type_discovery.members.len());
        if let Some(ref traits) = trait_derivability {
            crate::log_debug!(
                "   • Can derive Eq: {}, Ord: {}, Hash: {}",
                traits.can_derive_eq,
                traits.can_derive_ord,
                traits.can_derive_hash
            );
        }

        crate::log_debug!(
            "DISPATCH: Type '{}' has TypeKind::{:?}",
            type_name,
            type_discovery.type_kind
        );

        // Generate based on type kind
        match &type_discovery.type_kind {
            TypeKind::Struct | TypeKind::NestedStruct => {
                crate::log_debug!(
                    "   • Generating struct with {} fields",
                    type_discovery.members.len()
                );
                self.generate_struct(&type_discovery, trait_derivability.as_ref())?;
            }
            TypeKind::Enum => {
                crate::log_debug!(
                    "   • Generating enum with {} variants",
                    type_discovery.members.len()
                );
                self.generate_enum(&type_discovery, trait_derivability.as_ref())?;
            }
            TypeKind::ArrayOfComplex => {
                crate::log_debug!("   • Generating array type alias");
                let type_alias_generator =
                    TypeAliasGenerator::with_streaming_encode_encoding_base_and_max_collection_length(
                        self.streaming_encode,
                        self.encoding_base,
                        self.max_collection_length,
                    );
                let type_gen = type_alias_generator
                    .generate_array_type_alias(&type_discovery, trait_derivability.as_ref())?;
                self.generation.add_type_generation(type_gen);
            }
            TypeKind::PrimitiveWrapper { primitive_type } => {
                // Check if this is a bounded integer with special encoding
                let encoding_decision = self
                    .encoding_strategy
                    .as_ref()
                    .map(|s| s.get_decision(type_name))
                    .unwrap_or(EncodingDecision::Standard);
                let is_json_const = type_discovery.schema_object.contains_key("const");

                // Check if this is a bounded integer (i64) with optimization
                let is_bounded_integer = !is_json_const
                    && primitive_type == "i64"
                    && matches!(
                        encoding_decision,
                        EncodingDecision::BoundedIntegerCodec { .. }
                            | EncodingDecision::BoundedIntegerOffset { .. }
                    );

                // Check if this is a constant integer (O3 optimization)
                let is_constant_integer = !is_json_const
                    && matches!(encoding_decision, EncodingDecision::ConstantInteger { .. });

                // Check if this is a format-optimized string (O3 optimization)
                let is_format_optimized =
                    matches!(encoding_decision, EncodingDecision::FormatOptimized { .. });

                if is_bounded_integer {
                    crate::log_debug!(
                        "   • Generating bounded integer wrapper with encoding: {}",
                        encoding_decision.strategy_name()
                    );

                    // Generate newtype struct with codec-based encoding
                    let bounded_int_generator =
                        BoundedIntegerGenerator::with_streaming_encode_and_encoding_base(
                            self.streaming_encode,
                            self.encoding_base,
                        );
                    let (definition_tokens, implementation_tokens) = bounded_int_generator
                        .generate_bounded_integer_type(
                            &type_discovery,
                            &encoding_decision,
                            trait_derivability.as_ref(),
                        )?;

                    self.generation.add_type_generation(
                        TypeCodeGeneration::new_struct(
                            type_discovery.type_name.clone(),
                            definition_tokens,
                            implementation_tokens,
                        )
                        .with_token_encoder_impl(true),
                    );
                } else if is_constant_integer || is_format_optimized {
                    // O3 optimizations: constant integer or format-optimized string
                    crate::log_debug!(
                        "   • Generating primitive wrapper with O3 optimization: {}",
                        encoding_decision.strategy_name()
                    );
                    let type_alias_generator =
                        TypeAliasGenerator::with_streaming_encode_encoding_base_and_max_collection_length(
                            self.streaming_encode,
                            self.encoding_base,
                            self.max_collection_length,
                        );
                    let type_gen = type_alias_generator.generate_primitive_wrapper_with_decision(
                        &type_discovery,
                        primitive_type,
                        trait_derivability.as_ref(),
                        Some(&encoding_decision),
                    )?;
                    self.generation.add_type_generation(type_gen);
                } else {
                    // Standard primitive wrapper
                    crate::log_debug!("   • Generating primitive wrapper for '{}'", primitive_type);
                    let type_alias_generator =
                        TypeAliasGenerator::with_streaming_encode_encoding_base_and_max_collection_length(
                            self.streaming_encode,
                            self.encoding_base,
                            self.max_collection_length,
                        );
                    let type_gen = type_alias_generator.generate_primitive_wrapper(
                        &type_discovery,
                        primitive_type,
                        trait_derivability.as_ref(),
                    )?;
                    self.generation.add_type_generation(type_gen);
                }
            }
            TypeKind::MinimalObjectWrapper => {
                crate::log_debug!(
                    "   • Generating minimal object wrapper for '{}'",
                    type_discovery.type_name
                );
                let type_alias_generator =
                    TypeAliasGenerator::with_streaming_encode_encoding_base_and_max_collection_length(
                        self.streaming_encode,
                        self.encoding_base,
                        self.max_collection_length,
                    );
                let type_gen = type_alias_generator.generate_minimal_object_wrapper(
                    &type_discovery,
                    trait_derivability.as_ref(),
                )?;
                self.generation.add_type_generation(type_gen);
            }
            TypeKind::Reference { target } => {
                crate::log_debug!(
                    "   • Generating type alias for '{}' → '{}'",
                    type_discovery.type_name,
                    target
                );
                // Type aliases don't need Default (they inherit from target type)
                let type_alias_generator = TypeAliasGenerator::new();
                let type_gen =
                    type_alias_generator.generate_type_alias(&type_discovery.type_name, target)?;
                self.generation.add_type_generation(type_gen);
            }
        }

        Ok(())
    }

    /// Generate a struct type
    fn generate_struct(
        &mut self,
        type_discovery: &TypeDiscovery,
        trait_derivability: Option<&TraitDerivability>,
    ) -> TctResult<()> {
        // Delegate to StructGenerator with dependency injection
        let struct_generator = StructGenerator::with_discovery_and_encoding_base(
            &self.analysis,
            &self.discovery,
            self.streaming_encode,
            self.encoding_base,
        );

        // Get encoding decision for this type (if optimization was performed)
        let encoding_decision = self
            .encoding_strategy
            .as_ref()
            .and_then(|strategy| strategy.decisions.get(&type_discovery.type_name));

        // Create callback to identify zero-token types (SingleVariantEnum)
        // These types require 0 tokens to encode because they have only one possible value
        let is_zero_token_fn = |type_name: &str| -> bool {
            self.encoding_strategy
                .as_ref()
                .and_then(|strategy| strategy.decisions.get(type_name))
                .map(|decision| matches!(decision, EncodingDecision::SingleVariantEnum { .. }))
                .unwrap_or(false)
        };

        // Only pass the callback when streaming mode is enabled and we have encoding decisions
        let is_zero_token_type: Option<&dyn Fn(&str) -> bool> =
            if self.streaming_encode && self.encoding_strategy.is_some() {
                Some(&is_zero_token_fn)
            } else {
                None
            };

        let type_generation = struct_generator.generate_with_zero_token_info(
            type_discovery,
            trait_derivability,
            encoding_decision,
            &|name| self.sanitize_field_name(name),
            &|member_type| self.generate_member_type(member_type),
            &|trait_derivability, has_pattern_validators| {
                self.generate_derive_traits(trait_derivability, has_pattern_validators)
            },
            is_zero_token_type,
        )?;

        // Add to generation results
        self.generation.add_type_generation(type_generation);

        Ok(())
    }

    /// Generate an enum type
    fn generate_enum(
        &mut self,
        type_discovery: &TypeDiscovery,
        trait_derivability: Option<&TraitDerivability>,
    ) -> TctResult<()> {
        // Look up encoding decision for this type
        let encoding_decision = self
            .encoding_strategy
            .as_ref()
            .map(|strategy| strategy.get_decision(&type_discovery.type_name));

        // Delegate to EnumGenerator with dependency injection
        let enum_generator = EnumGenerator::new(&self.discovery, self.streaming_encode);

        let type_generation = enum_generator.generate(
            type_discovery,
            trait_derivability,
            encoding_decision.as_ref(),
            &|member_type| self.generate_member_type(member_type),
            &|trait_derivability, has_pattern_validators| {
                self.generate_derive_traits(trait_derivability, has_pattern_validators)
            },
            &|trait_derivability, has_pattern_validators| {
                self.generate_derive_traits_without_deserialize(
                    trait_derivability,
                    has_pattern_validators,
                )
            },
            &|trait_derivability, has_pattern_validators| {
                self.generate_derive_traits_without_default(
                    trait_derivability,
                    has_pattern_validators,
                )
            },
            &|trait_derivability, has_pattern_validators| {
                self.generate_derive_traits_without_deserialize_and_default(
                    trait_derivability,
                    has_pattern_validators,
                )
            },
        )?;

        // Add to generation results
        self.generation.add_type_generation(type_generation);

        Ok(())
    }
}
