//! Optimization Phase - Phase 3 of the 4-phase compilation pipeline
//!
//! This pass makes encoding decisions based on schema analysis and constraints.
//! It does NOT generate code - it only decides HOW each type should be encoded.
//!
//! Pipeline position: Discovery → Analysis → **Optimization** → Generation

use crate::TctResult;
use crate::generator::encoding_decision::{
    CompilationConstraints, EncodingDecision, EncodingStrategy, MemberTypeRefinement,
};
use crate::generator::schema_discovery::{SchemaDiscovery, TypeKind};
use crate::generator::type_analysis::TypeAnalysis;
use crate::optimization_pass::OptimizationConfig;
use serde_json::Value;

/// Optimization phase that makes encoding decisions
pub struct OptimizationPhase {
    /// Configuration for optimization
    config: OptimizationConfig,
}

impl OptimizationPhase {
    /// Create new optimization phase with configuration
    pub fn new(config: OptimizationConfig) -> Self {
        Self { config }
    }

    #[allow(deprecated)]
    fn has_pass(&self, name: &str) -> bool {
        self.config.passes.iter().any(|pass| pass.name() == name)
    }

    /// Run optimization phase to create encoding strategy
    ///
    /// Takes discovered types and analysis results, applies optimization strategies,
    /// and produces encoding decisions for each type.
    ///
    /// Returns the encoding strategy and explicit generation metadata.
    pub fn run(
        &self,
        schema: &Value,
        discovery: &SchemaDiscovery,
        _analysis: &TypeAnalysis,
    ) -> TctResult<EncodingStrategy> {
        crate::log_debug!(
            "⚡ Starting OptimizationPhase with level {:?}, {} discovered types",
            self.config.level,
            discovery.discovered_types.len()
        );

        let mut strategy = EncodingStrategy::new();

        // Record explicit type refinements for additionalProperties at O2+ level.
        // Generation consumes this metadata without replacing DiscoveryPass output.
        if matches!(
            self.config.level,
            crate::optimization_pass::OptimizationLevel::O2
                | crate::optimization_pass::OptimizationLevel::O3
                | crate::optimization_pass::OptimizationLevel::Os
                | crate::optimization_pass::OptimizationLevel::OVocab
        ) {
            crate::log_debug!(
                "🔧 {:?} active: Recording additionalProperties type refinements",
                self.config.level
            );
            self.record_additional_properties_type_refinements(schema, discovery, &mut strategy)?;
        }

        // Extract constraints from config
        let constraints = CompilationConstraints {
            max_vocab: self.config.max_vocab,
            ..Default::default()
        };

        strategy.max_vocabulary_size = constraints.max_vocab;

        // Make encoding decisions for each discovered type.
        for (type_name, type_discovery) in &discovery.discovered_types {
            let decision = self.decide_encoding_for_type(type_name, type_discovery, &constraints);

            crate::log_debug!(
                "  • {}: {:?} → {}",
                type_name,
                type_discovery.type_kind,
                decision.strategy_name()
            );

            strategy.add_decision(type_name, decision);
        }

        // Calculate statistics
        strategy.update_stats();

        crate::log_debug!(
            "✅ OptimizationPhase complete: {} optimized fields, {} enum domains, {} bounded integer domains",
            strategy.stats.optimized_fields,
            strategy.stats.enum_domains,
            strategy.stats.bounded_integer_domains
        );

        Ok(strategy)
    }

    /// Record additionalProperties type refinements using AdditionalPropertiesAnalyzer.
    ///
    /// This optimization analyzes unconstrained additionalProperties (HashMap<String, serde_json::Value>)
    /// and infers more specific types based on contextual heuristics:
    /// - Existing property types (e.g., if all properties are strings → HashMap<String, String>)
    /// - Schema hints (anyOf/oneOf with common patterns)
    /// - Parent type context
    ///
    /// The resulting metadata is consumed by generation without mutating discovery.
    fn record_additional_properties_type_refinements(
        &self,
        schema: &Value,
        discovery: &SchemaDiscovery,
        strategy: &mut EncodingStrategy,
    ) -> TctResult<()> {
        use crate::generator::additional_properties_analyzer::AdditionalPropertiesAnalyzer;
        use crate::generator::schema_discovery::MemberType;

        let mut refinement_count = 0;

        // Iterate through all discovered types
        for (type_name, type_discovery) in &discovery.discovered_types {
            // Only process struct types
            if !matches!(
                type_discovery.type_kind,
                crate::generator::schema_discovery::TypeKind::Struct
            ) {
                continue;
            }

            // Find additional_properties member
            let additional_props_index = type_discovery
                .members
                .iter()
                .position(|m| m.name == "additional_properties");

            if let Some(idx) = additional_props_index {
                let member = &type_discovery.members[idx];

                // Only refine if it's currently HashMap<String, serde_json::Value>
                if let MemberType::HashMap { key, value } = &member.member_type
                    && value == "serde_json::Value"
                {
                    let Some(additional_props_value) =
                        type_discovery.schema_object.get("additionalProperties")
                    else {
                        // Missing additionalProperties defaults to true in JSON Schema.
                        // Do not narrow an unconstrained extension map.
                        continue;
                    };

                    if !additional_props_value.is_object() {
                        // Only explicit object schemas constrain additional property values.
                        // Boolean true is unconstrained and must keep serde_json::Value.
                        continue;
                    }

                    // Build analysis context from schema
                    let context = self.build_analysis_context(
                        schema,
                        type_name,
                        &type_discovery.schema_object,
                    );

                    // Analyze explicitly constrained additionalProperties.
                    let analysis_result = AdditionalPropertiesAnalyzer::analyze(
                        additional_props_value,
                        &context,
                        crate::optimization_pass::OptimizationLevel::O3,
                    );

                    // Record a generation-facing refinement based on analysis.
                    if let Some(refinement) =
                        self.convert_analysis_to_member_refinement(&analysis_result, key)
                    {
                        crate::log_debug!(
                            "  🔧 Recorded {}.additional_properties refinement: {:?} → {:?}",
                            type_name,
                            member.member_type,
                            refinement
                        );
                        strategy.add_member_type_refinement(
                            type_name.clone(),
                            "additional_properties",
                            refinement,
                        );
                        refinement_count += 1;
                    }
                }
            }
        }

        if refinement_count > 0 {
            crate::log_debug!(
                "✨ Type refinement complete: {} additionalProperties fields optimized",
                refinement_count
            );
        }

        Ok(())
    }

    /// Build analysis context from schema for AdditionalPropertiesAnalyzer
    fn build_analysis_context(
        &self,
        _root_schema: &Value,
        _type_name: &str,
        schema_object: &serde_json::Map<String, Value>,
    ) -> crate::generator::additional_properties_analyzer::AnalysisContext {
        use crate::generator::additional_properties_analyzer::{AnalysisContext, PropertyInfo};

        // Extract existing properties to inform analysis
        let existing_properties = if let Some(properties) = schema_object.get("properties") {
            if let Some(props_obj) = properties.as_object() {
                // Convert properties to PropertyInfo
                props_obj
                    .iter()
                    .map(|(name, prop_schema)| {
                        let json_type = if let Some(type_val) = prop_schema.get("type") {
                            type_val.as_str().unwrap_or("unknown").to_string()
                        } else {
                            "unknown".to_string()
                        };

                        let has_pattern = prop_schema.get("pattern").is_some();
                        let is_required = schema_object
                            .get("required")
                            .and_then(|r| r.as_array())
                            .map(|arr| arr.iter().any(|v| v.as_str() == Some(name)))
                            .unwrap_or(false);

                        PropertyInfo {
                            name: name.clone(),
                            json_type,
                            has_pattern,
                            is_required,
                        }
                    })
                    .collect()
            } else {
                Vec::new()
            }
        } else {
            Vec::new()
        };

        // Extract schema title and description
        let schema_title = schema_object
            .get("title")
            .and_then(|v| v.as_str().map(String::from));
        let schema_description = schema_object
            .get("description")
            .and_then(|v| v.as_str().map(String::from));

        // Extract examples
        let examples = schema_object
            .get("examples")
            .and_then(|v| v.as_array())
            .cloned()
            .unwrap_or_default();

        AnalysisContext {
            schema_title,
            schema_description,
            existing_properties,
            examples,
            // Future: Could track parent type context for better inference
            // (e.g., "dependencies" → infer string values, "scripts" → infer string commands)
            // Currently not tracked as it requires schema traversal changes
            parent_context: None,
        }
    }

    /// Convert AdditionalPropertiesType to generation-facing member refinement.
    ///
    /// Maps the analyzer's type inference to concrete Rust HashMap types
    #[allow(clippy::only_used_in_recursion)]
    fn convert_analysis_to_member_refinement(
        &self,
        analysis: &crate::generator::additional_properties_analyzer::AdditionalPropertiesType,
        key_type: &str,
    ) -> Option<MemberTypeRefinement> {
        use crate::generator::additional_properties_analyzer::AdditionalPropertiesType;

        match analysis {
            AdditionalPropertiesType::StringOnlyConstrained
            | AdditionalPropertiesType::StringOnlyUnconstrained => {
                Some(MemberTypeRefinement::HashMap {
                    key: key_type.to_string(),
                    value: "String".to_string(),
                })
            }
            AdditionalPropertiesType::NumberOnlyConstrained
            | AdditionalPropertiesType::NumberOnlyUnconstrained => {
                Some(MemberTypeRefinement::HashMap {
                    key: key_type.to_string(),
                    value: "f64".to_string(),
                })
            }
            AdditionalPropertiesType::IntegerOnlyConstrained => {
                Some(MemberTypeRefinement::HashMap {
                    key: key_type.to_string(),
                    value: "JsonInteger".to_string(),
                })
            }
            AdditionalPropertiesType::BooleanOnlyConstrained
            | AdditionalPropertiesType::BooleanOnlyUnconstrained => {
                Some(MemberTypeRefinement::HashMap {
                    key: key_type.to_string(),
                    value: "bool".to_string(),
                })
            }
            AdditionalPropertiesType::ArrayOf(_inner_type) => {
                // Arrays with unknown inner type → Vec<serde_json::Value>
                // Future: Could recursively refine based on inner_type
                // (e.g., ArrayOf(StringOnly) → Vec<String>)
                // Currently uses generic Value to ensure compatibility
                Some(MemberTypeRefinement::HashMap {
                    key: key_type.to_string(),
                    value: "Vec<serde_json::Value>".to_string(),
                })
            }
            AdditionalPropertiesType::Contextual(boxed_result) => {
                // Recursively convert inner type
                self.convert_analysis_to_member_refinement(boxed_result, key_type)
            }
            // For complex cases that can't be represented as simple HashMap types,
            // keep the generic serde_json::Value (return None to indicate no refinement)
            AdditionalPropertiesType::Arbitrary
            | AdditionalPropertiesType::TaggedUnion
            | AdditionalPropertiesType::Typed(_)
            | AdditionalPropertiesType::Union(_)
            | AdditionalPropertiesType::None => None,
        }
    }

    /// Decide encoding for a single type
    fn decide_encoding_for_type(
        &self,
        type_name: &str,
        type_discovery: &crate::generator::schema_discovery::TypeDiscovery,
        constraints: &CompilationConstraints,
    ) -> EncodingDecision {
        // Enum optimization: Check for single-variant enum (constant)
        // Safe schema-derived optimization - no runtime validation needed
        if self.has_pass("enum_index_optimization")
            && let TypeKind::Enum = &type_discovery.type_kind
            && type_discovery.members.len() == 1
        {
            crate::log_debug!(
                "    → enum optimization: Single-variant enum detected: {} (constant: {:?})",
                type_name,
                type_discovery.members[0].name
            );
            return EncodingDecision::SingleVariantEnum {
                variant: type_discovery.members[0].name.clone(),
            };
        }

        if self.has_pass("bounded_integer_optimization") {
            // Bounded integer optimization: Check for constant integer (min = max)
            // Safe schema-derived optimization - no runtime validation needed
            if let TypeKind::PrimitiveWrapper { primitive_type } = &type_discovery.type_kind
                && primitive_type == "i64"
            {
                let schema = &type_discovery.schema_object;
                if let (Some(min_val), Some(max_val)) = (
                    schema.get("minimum").and_then(|v| v.as_i64()),
                    schema.get("maximum").and_then(|v| v.as_i64()),
                ) && min_val == max_val
                {
                    crate::log_debug!(
                        "    → bounded integer optimization: Constant integer detected: {} (value: {})",
                        type_name,
                        min_val
                    );
                    return EncodingDecision::ConstantInteger { value: min_val };
                }
            }
        }

        // O3 optimization: Check for format-optimized strings
        // Semantic interpretation - requires runtime validation
        if matches!(
            self.config.level,
            crate::optimization_pass::OptimizationLevel::O3
        ) && let TypeKind::PrimitiveWrapper { primitive_type } = &type_discovery.type_kind
            && primitive_type == "String"
            && let Some(format) = type_discovery.schema_object.get("format")
            && let Some(format_str) = format.as_str()
            && let Some(string_format) = self.parse_string_format(format_str)
        {
            crate::log_debug!(
                "    → O3: Format-optimized string detected: {} (format: {:?})",
                type_name,
                string_format
            );
            return EncodingDecision::FormatOptimized {
                format: string_format,
                fallback_to_string: true, // Allow malformed values
            };
        }

        match &type_discovery.type_kind {
            TypeKind::Enum => {
                // Extract variant names from members
                let variants: Vec<String> = type_discovery
                    .members
                    .iter()
                    .map(|m| m.name.clone())
                    .collect();
                self.decide_enum_encoding(type_name, &variants, constraints)
            }
            TypeKind::PrimitiveWrapper { primitive_type } if primitive_type == "i64" => {
                // Handle bounded integers
                self.decide_bounded_integer_encoding(type_name, type_discovery, constraints)
            }
            TypeKind::Struct => {
                // O3/Os optimization: Check for bit-packed optional field presence
                if let Some((optional_count, presence_mask_groups)) =
                    self.should_use_bit_packing(type_discovery)
                {
                    crate::log_debug!(
                        "    → O3/Os: Bit-packed struct detected: {} ({} optional fields → {} presence-mask groups)",
                        type_name,
                        optional_count,
                        presence_mask_groups
                    );
                    return EncodingDecision::BitPackedStruct {
                        optional_field_count: optional_count,
                        presence_tokens_needed: presence_mask_groups,
                    };
                }

                // Standard struct encoding (individual presence flags)
                EncodingDecision::Standard
            }
            _ => {
                // Other types use standard encoding
                EncodingDecision::Standard
            }
        }
    }

    /// Parse JSON Schema format string to StringFormat enum
    fn parse_string_format(
        &self,
        format_str: &str,
    ) -> Option<crate::generator::encoding_decision::StringFormat> {
        use crate::generator::encoding_decision::StringFormat;

        match format_str {
            "int32" => Some(StringFormat::Int32),
            "int64" => Some(StringFormat::Int64),
            "double" => Some(StringFormat::Double),
            "float" => Some(StringFormat::Float),
            "byte" => Some(StringFormat::Byte),
            "date-time" | "datetime" => Some(StringFormat::DateTime),
            "date" => Some(StringFormat::Date),
            "time" => Some(StringFormat::Time),
            "duration" => Some(StringFormat::Duration),
            "uuid" => Some(StringFormat::Uuid),
            "email" | "idn-email" => Some(StringFormat::Email),
            "uri" => Some(StringFormat::Uri),
            "uri-reference" => Some(StringFormat::UriReference),
            "uri-template" | "iri" | "iri-reference" => Some(StringFormat::UriTemplate),
            "hostname" | "idn-hostname" => Some(StringFormat::Hostname),
            "ipv4" => Some(StringFormat::Ipv4),
            "ipv6" => Some(StringFormat::Ipv6),
            "json-pointer" | "relative-json-pointer" => Some(StringFormat::JsonPointer),
            "regex" => Some(StringFormat::Regex),
            _ => None, // Unknown format, use standard string encoding
        }
    }

    /// Check if struct should use bit-packed optional field presence encoding
    ///
    /// Returns Some((optional_count, presence_mask_groups)) if bit-packing should be used.
    ///
    /// A presence-mask group stores up to 32 optional-field bits in one u32
    /// mask. The generated structural codec may emit multiple base tokens for
    /// each u32 mask, depending on the mask value and encoding base.
    fn should_use_bit_packing(
        &self,
        type_discovery: &crate::generator::schema_discovery::TypeDiscovery,
    ) -> Option<(usize, usize)> {
        // Check if bit-packing is enabled in config
        // (Default: true at O3/Os/OVocab, false at O0/O2, configurable via OptimizationConfig::with_bit_packing)
        if !self.config.bit_packing_enabled {
            return None;
        }

        // Count optional fields that use Option<T> (2-state: present/absent).
        // OptionalField<T> (nullable or serde_json::Value-backed) is three-state
        // and cannot be folded into a presence mask.
        let optional_option_fields = type_discovery
            .members
            .iter()
            .filter(|m| m.uses_option_presence_flag())
            .count();

        // Only use bit-packing if we have 4+ optional Option<T> fields
        // Below 4 fields, individual tokens are more efficient (no overhead)
        if optional_option_fields >= 4 {
            // Calculate number of u32 presence-mask groups needed: ceil(count / 32).
            let presence_tokens_needed = optional_option_fields.div_ceil(32);

            Some((optional_option_fields, presence_tokens_needed))
        } else {
            None
        }
    }

    /// Decide encoding for an enum type
    fn decide_enum_encoding(
        &self,
        _type_name: &str,
        variants: &[String],
        constraints: &CompilationConstraints,
    ) -> EncodingDecision {
        // If no optimization passes are enabled, use standard encoding
        if !self.has_pass("enum_index_optimization") {
            return EncodingDecision::Standard;
        }

        if let Some(max_vocab) = constraints.max_vocab
            && variants.len() > max_vocab
        {
            return EncodingDecision::EnumCodec {
                variants: variants.to_vec(),
                codec_spec: crate::generator::encoding_decision::CodecSpec::calculate(
                    variants.len().saturating_sub(1) as u32,
                    Some(max_vocab as u32),
                ),
            };
        }

        // Use standard index-based encoding (single token per variant).
        EncodingDecision::EnumIndex {
            variants: variants.to_vec(),
        }
    }

    /// Decide encoding for a bounded integer type
    fn decide_bounded_integer_encoding(
        &self,
        _type_name: &str,
        type_discovery: &crate::generator::schema_discovery::TypeDiscovery,
        constraints: &CompilationConstraints,
    ) -> EncodingDecision {
        // If no optimization passes are enabled, use standard encoding
        if !self.has_pass("bounded_integer_optimization") {
            return EncodingDecision::Standard;
        }

        // Extract minimum and maximum from schema. Unbounded or partially
        // bounded integers must not be treated as [0, 0].
        let schema = &type_discovery.schema_object;
        let Some(min) = schema.get("minimum").and_then(|v| v.as_i64()) else {
            return EncodingDecision::Standard;
        };
        let Some(max) = schema.get("maximum").and_then(|v| v.as_i64()) else {
            return EncodingDecision::Standard;
        };

        // Calculate range size in a wider type. Offset and codec encodings
        // carry offsets as u32, so wider i64 domains must fall back to the
        // standard lossless i64 wrapper encoding instead of truncating.
        let range = i128::from(max) - i128::from(min) + 1;
        if range <= 0 {
            return EncodingDecision::Standard;
        }
        let Ok(range_u32) = u32::try_from(range) else {
            return EncodingDecision::Standard;
        };

        // Check structural vocabulary constraint (LOSSLESS multi-token encoding)
        // Only use codec encoding when the range EXCEEDS max_vocab
        if let Some(max_structural) = constraints.max_vocab
            && range_u32 > max_structural as u32
        {
            // Range exceeds vocabulary budget: use codec-based multi-token encoding
            let codec_spec = crate::generator::encoding_decision::CodecSpec::calculate(
                range_u32.saturating_sub(1),
                Some(max_structural as u32),
            );

            crate::log_debug!(
                "    → Vocab constraint active: range {} > max_vocab {}, using {}-token codec (base={})",
                range,
                max_structural,
                codec_spec.token_width,
                codec_spec.base
            );

            return EncodingDecision::BoundedIntegerCodec {
                min,
                max,
                codec_spec,
            };
        }

        // No constraints: Use standard offset encoding
        EncodingDecision::BoundedIntegerOffset {
            min,
            max,
            range: range_u32,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::generator::schema_discovery::{
        MemberInfo, MemberType, SchemaLocation, TypeDiscovery,
    };

    fn create_test_config(max_vocab: Option<usize>) -> OptimizationConfig {
        let mut config = crate::optimization_pass::OptimizationConfig::o2();
        if let Some(max) = max_vocab {
            config.max_vocab = Some(max);
        }
        config
    }

    fn create_enum_discovery(name: &str, variant_count: usize) -> (String, TypeDiscovery) {
        use crate::generator::schema_discovery::{MemberInfo, MemberType, SchemaLocation};

        // Create enum members (variants)
        let members: Vec<MemberInfo> = (0..variant_count)
            .map(|i| MemberInfo {
                name: format!("variant_{}", i),
                member_type: MemberType::Primitive("()".to_string()), // Enum variants don't have data
                required: true,
                is_nullable: false,
                schema: serde_json::Map::new(),
            })
            .collect();

        let mut discovery = TypeDiscovery::new(
            name.to_string(),
            TypeKind::Enum,
            SchemaLocation {
                json_path: format!("#/definitions/{}", name),
                description: format!("Test enum {}", name),
                parent_type: None,
                property_name: None,
            },
            serde_json::Map::new(),
        );

        // Add members
        for member in members {
            discovery.add_member(member);
        }

        (name.to_string(), discovery)
    }

    #[test]
    fn test_optimization_pass_no_constraint() {
        let config = create_test_config(None);
        let pass = OptimizationPhase::new(config);

        let mut discovery = SchemaDiscovery::new();
        let (name, type_disc) = create_enum_discovery("Status", 5);
        discovery.discovered_types.insert(name.clone(), type_disc);

        let analysis = TypeAnalysis::default();
        let strategy = pass
            .run(&serde_json::json!({}), &discovery, &analysis)
            .unwrap();

        let decision = strategy.get_decision("Status");
        assert!(matches!(decision, EncodingDecision::EnumIndex { .. }));
    }

    #[test]
    fn test_optimization_pass_with_constraint_fits() {
        let config = create_test_config(Some(100));
        let pass = OptimizationPhase::new(config);

        let mut discovery = SchemaDiscovery::new();
        let (name, type_disc) = create_enum_discovery("Status", 5);
        discovery.discovered_types.insert(name.clone(), type_disc);

        let analysis = TypeAnalysis::default();
        let strategy = pass
            .run(&serde_json::json!({}), &discovery, &analysis)
            .unwrap();

        let decision = strategy.get_decision("Status");
        // 5 variants < 100 max_vocab, so use EnumIndex (not constrained)
        assert!(matches!(decision, EncodingDecision::EnumIndex { .. }));
    }

    #[test]
    fn test_optimization_pass_with_constraint_encodes_wide_enum() {
        let config = create_test_config(Some(257));
        let pass = OptimizationPhase::new(config);

        let mut discovery = SchemaDiscovery::new();
        let (name, type_disc) = create_enum_discovery("ToolBranch", 385);
        discovery.discovered_types.insert(name.clone(), type_disc);

        let strategy = pass
            .run(&serde_json::json!({}), &discovery, &TypeAnalysis::default())
            .unwrap();
        match strategy.get_decision("ToolBranch") {
            EncodingDecision::EnumCodec { codec_spec, .. } => {
                assert_eq!(codec_spec.base, 257);
                assert_eq!(codec_spec.token_width, 2);
                assert_eq!(codec_spec.max_value, 384);
            }
            decision => panic!("expected constrained enum codec, got {decision:?}"),
        }
    }

    #[test]
    fn test_additional_properties_refinement_is_explicit_metadata() {
        let additional_properties_schema = serde_json::json!({"type": "string"});
        let schema = serde_json::json!({
            "type": "object",
            "additionalProperties": additional_properties_schema
        });

        let mut type_discovery = TypeDiscovery::new(
            "Config".to_string(),
            TypeKind::Struct,
            SchemaLocation {
                json_path: "#/".to_string(),
                description: "test config".to_string(),
                parent_type: None,
                property_name: None,
            },
            schema.as_object().unwrap().clone(),
        );
        type_discovery.add_member(MemberInfo {
            name: "additional_properties".to_string(),
            member_type: MemberType::HashMap {
                key: "String".to_string(),
                value: "serde_json::Value".to_string(),
            },
            required: false,
            is_nullable: false,
            schema: additional_properties_schema.as_object().unwrap().clone(),
        });

        let mut discovery = SchemaDiscovery::new();
        discovery
            .discovered_types
            .insert("Config".to_string(), type_discovery);
        let original_member_type = discovery
            .get_type("Config")
            .unwrap()
            .members
            .iter()
            .find(|member| member.name == "additional_properties")
            .unwrap()
            .member_type
            .clone();

        assert_eq!(
            original_member_type,
            MemberType::HashMap {
                key: "String".to_string(),
                value: "serde_json::Value".to_string(),
            }
        );

        let pass =
            OptimizationPhase::new(crate::optimization_pass::OptimizationConfig::with_all_passes());
        let strategy = pass
            .run(&schema, &discovery, &TypeAnalysis::default())
            .unwrap();

        assert_eq!(
            discovery
                .get_type("Config")
                .unwrap()
                .members
                .iter()
                .find(|member| member.name == "additional_properties")
                .unwrap()
                .member_type,
            original_member_type,
            "optimization must not mutate discovery output"
        );

        let path =
            crate::generator::encoding_decision::MemberPath::new("Config", "additional_properties");
        assert_eq!(
            strategy.member_type_refinements.get(&path),
            Some(&MemberTypeRefinement::HashMap {
                key: "String".to_string(),
                value: "String".to_string(),
            })
        );
    }

    #[test]
    fn test_parse_string_format_datetime_variants() {
        use crate::generator::encoding_decision::StringFormat;
        let config = create_test_config(None);
        let pass = OptimizationPhase::new(config);

        // Test datetime formats
        assert_eq!(
            pass.parse_string_format("date-time"),
            Some(StringFormat::DateTime)
        );
        assert_eq!(
            pass.parse_string_format("datetime"),
            Some(StringFormat::DateTime)
        );
        assert_eq!(pass.parse_string_format("date"), Some(StringFormat::Date));
        assert_eq!(pass.parse_string_format("time"), Some(StringFormat::Time));
        assert_eq!(
            pass.parse_string_format("duration"),
            Some(StringFormat::Duration)
        );
    }

    #[test]
    fn test_parse_string_format_numeric_variants() {
        use crate::generator::encoding_decision::StringFormat;
        let config = create_test_config(None);
        let pass = OptimizationPhase::new(config);

        // Test numeric formats
        assert_eq!(pass.parse_string_format("int32"), Some(StringFormat::Int32));
        assert_eq!(pass.parse_string_format("int64"), Some(StringFormat::Int64));
        assert_eq!(
            pass.parse_string_format("double"),
            Some(StringFormat::Double)
        );
        assert_eq!(pass.parse_string_format("float"), Some(StringFormat::Float));
        assert_eq!(pass.parse_string_format("byte"), Some(StringFormat::Byte));
    }

    #[test]
    fn test_parse_string_format_network_variants() {
        use crate::generator::encoding_decision::StringFormat;
        let config = create_test_config(None);
        let pass = OptimizationPhase::new(config);

        // Test network/identifier formats
        assert_eq!(pass.parse_string_format("uuid"), Some(StringFormat::Uuid));
        assert_eq!(pass.parse_string_format("email"), Some(StringFormat::Email));
        assert_eq!(
            pass.parse_string_format("idn-email"),
            Some(StringFormat::Email)
        );
        assert_eq!(pass.parse_string_format("uri"), Some(StringFormat::Uri));
        assert_eq!(
            pass.parse_string_format("uri-reference"),
            Some(StringFormat::UriReference)
        );
        assert_eq!(
            pass.parse_string_format("uri-template"),
            Some(StringFormat::UriTemplate)
        );
        assert_eq!(
            pass.parse_string_format("iri"),
            Some(StringFormat::UriTemplate)
        );
        assert_eq!(
            pass.parse_string_format("iri-reference"),
            Some(StringFormat::UriTemplate)
        );
    }

    #[test]
    fn test_parse_string_format_hostname_and_ip() {
        use crate::generator::encoding_decision::StringFormat;
        let config = create_test_config(None);
        let pass = OptimizationPhase::new(config);

        // Test hostname and IP formats
        assert_eq!(
            pass.parse_string_format("hostname"),
            Some(StringFormat::Hostname)
        );
        assert_eq!(
            pass.parse_string_format("idn-hostname"),
            Some(StringFormat::Hostname)
        );
        assert_eq!(pass.parse_string_format("ipv4"), Some(StringFormat::Ipv4));
        assert_eq!(pass.parse_string_format("ipv6"), Some(StringFormat::Ipv6));
    }

    #[test]
    fn test_parse_string_format_json_pointer_and_regex() {
        use crate::generator::encoding_decision::StringFormat;
        let config = create_test_config(None);
        let pass = OptimizationPhase::new(config);

        // Test JSON pointer and regex formats
        assert_eq!(
            pass.parse_string_format("json-pointer"),
            Some(StringFormat::JsonPointer)
        );
        assert_eq!(
            pass.parse_string_format("relative-json-pointer"),
            Some(StringFormat::JsonPointer)
        );
        assert_eq!(pass.parse_string_format("regex"), Some(StringFormat::Regex));
    }

    #[test]
    fn test_parse_string_format_unknown() {
        let config = create_test_config(None);
        let pass = OptimizationPhase::new(config);

        // Test unknown/unsupported formats return None
        assert_eq!(pass.parse_string_format("unknown"), None);
        assert_eq!(pass.parse_string_format("custom-format"), None);
        assert_eq!(pass.parse_string_format(""), None);
    }

    #[test]
    fn test_should_use_bit_packing_below_threshold() {
        use crate::generator::schema_discovery::{MemberInfo, MemberType, SchemaLocation};

        let config = create_test_config(None);
        let pass = OptimizationPhase::new(config);

        // Create struct with 3 optional fields (below 4-field threshold)
        let mut discovery = TypeDiscovery::new(
            "TestStruct".to_string(),
            TypeKind::Struct,
            SchemaLocation {
                json_path: "#/definitions/TestStruct".to_string(),
                description: "Test struct".to_string(),
                parent_type: None,
                property_name: None,
            },
            serde_json::Map::new(),
        );

        for i in 0..3 {
            discovery.add_member(MemberInfo {
                name: format!("field_{}", i),
                member_type: MemberType::Primitive("String".to_string()),
                required: false,    // Optional field
                is_nullable: false, // Not nullable -> Option<T>
                schema: serde_json::Map::new(),
            });
        }

        // Should return None because 3 < 4 threshold
        assert_eq!(pass.should_use_bit_packing(&discovery), None);
    }

    #[test]
    fn test_should_use_bit_packing_at_threshold() {
        use crate::generator::schema_discovery::{MemberInfo, MemberType, SchemaLocation};

        let config = create_test_config(None).with_bit_packing(true);
        let pass = OptimizationPhase::new(config);

        // Create struct with 4 optional fields (at 4-field threshold)
        let mut discovery = TypeDiscovery::new(
            "TestStruct".to_string(),
            TypeKind::Struct,
            SchemaLocation {
                json_path: "#/definitions/TestStruct".to_string(),
                description: "Test struct".to_string(),
                parent_type: None,
                property_name: None,
            },
            serde_json::Map::new(),
        );

        for i in 0..4 {
            discovery.add_member(MemberInfo {
                name: format!("field_{}", i),
                member_type: MemberType::Primitive("String".to_string()),
                required: false,    // Optional field
                is_nullable: false, // Not nullable -> Option<T>
                schema: serde_json::Map::new(),
            });
        }

        // Should return Some((4, 1)) - 4 fields fit in 1 token
        assert_eq!(pass.should_use_bit_packing(&discovery), Some((4, 1)));
    }

    #[test]
    fn test_should_use_bit_packing_multi_token() {
        use crate::generator::schema_discovery::{MemberInfo, MemberType, SchemaLocation};

        let config = create_test_config(None).with_bit_packing(true);
        let pass = OptimizationPhase::new(config);

        // Create struct with 65 optional fields (requires 3 tokens: ceil(65/32) = 3)
        let mut discovery = TypeDiscovery::new(
            "LargeStruct".to_string(),
            TypeKind::Struct,
            SchemaLocation {
                json_path: "#/definitions/LargeStruct".to_string(),
                description: "Test struct".to_string(),
                parent_type: None,
                property_name: None,
            },
            serde_json::Map::new(),
        );

        for i in 0..65 {
            discovery.add_member(MemberInfo {
                name: format!("field_{}", i),
                member_type: MemberType::Primitive("String".to_string()),
                required: false,    // Optional field
                is_nullable: false, // Not nullable -> Option<T>
                schema: serde_json::Map::new(),
            });
        }

        // Should return Some((65, 3)) - 65 fields require 3 tokens
        assert_eq!(pass.should_use_bit_packing(&discovery), Some((65, 3)));
    }

    #[test]
    fn test_should_use_bit_packing_excludes_nullable_fields() {
        use crate::generator::schema_discovery::{MemberInfo, MemberType, SchemaLocation};

        let config = create_test_config(None);
        let pass = OptimizationPhase::new(config);

        // Create struct with 2 Option<T> fields and 2 OptionalField<T> fields
        let mut discovery = TypeDiscovery::new(
            "MixedStruct".to_string(),
            TypeKind::Struct,
            SchemaLocation {
                json_path: "#/definitions/MixedStruct".to_string(),
                description: "Test struct".to_string(),
                parent_type: None,
                property_name: None,
            },
            serde_json::Map::new(),
        );

        // Add 2 optional non-nullable fields (Option<T>, can be bit-packed)
        for i in 0..2 {
            discovery.add_member(MemberInfo {
                name: format!("optional_field_{}", i),
                member_type: MemberType::Primitive("String".to_string()),
                required: false,
                is_nullable: false, // Option<T>
                schema: serde_json::Map::new(),
            });
        }

        // Add 2 optional nullable fields (OptionalField<T>, can't be bit-packed)
        for i in 0..2 {
            discovery.add_member(MemberInfo {
                name: format!("nullable_field_{}", i),
                member_type: MemberType::Primitive("String".to_string()),
                required: false,
                is_nullable: true, // OptionalField<T> (3-state)
                schema: serde_json::Map::new(),
            });
        }

        // Should return None because only 2 Option<T> fields (< 4 threshold)
        // Nullable fields don't count towards bit-packing
        assert_eq!(pass.should_use_bit_packing(&discovery), None);
    }

    #[test]
    fn test_should_use_bit_packing_excludes_json_value_fields() {
        use crate::generator::schema_discovery::{MemberInfo, MemberType, SchemaLocation};

        let config = create_test_config(None).with_bit_packing(true);
        let pass = OptimizationPhase::new(config);

        let mut discovery = TypeDiscovery::new(
            "ValueMixedStruct".to_string(),
            TypeKind::Struct,
            SchemaLocation {
                json_path: "#/definitions/ValueMixedStruct".to_string(),
                description: "Test struct".to_string(),
                parent_type: None,
                property_name: None,
            },
            serde_json::Map::new(),
        );

        for i in 0..3 {
            discovery.add_member(MemberInfo {
                name: format!("optional_field_{}", i),
                member_type: MemberType::Primitive("String".to_string()),
                required: false,
                is_nullable: false,
                schema: serde_json::Map::new(),
            });
        }

        discovery.add_member(MemberInfo {
            name: "payload".to_string(),
            member_type: MemberType::Primitive("serde_json::Value".to_string()),
            required: false,
            is_nullable: false,
            schema: serde_json::Map::new(),
        });

        assert_eq!(pass.should_use_bit_packing(&discovery), None);
    }

    #[test]
    fn test_should_use_bit_packing_counts_only_option_fields() {
        use crate::generator::schema_discovery::{MemberInfo, MemberType, SchemaLocation};

        let config = create_test_config(None).with_bit_packing(true);
        let pass = OptimizationPhase::new(config);

        let mut discovery = TypeDiscovery::new(
            "ValueMixedStruct".to_string(),
            TypeKind::Struct,
            SchemaLocation {
                json_path: "#/definitions/ValueMixedStruct".to_string(),
                description: "Test struct".to_string(),
                parent_type: None,
                property_name: None,
            },
            serde_json::Map::new(),
        );

        for i in 0..4 {
            discovery.add_member(MemberInfo {
                name: format!("optional_field_{}", i),
                member_type: MemberType::Primitive("String".to_string()),
                required: false,
                is_nullable: false,
                schema: serde_json::Map::new(),
            });
        }

        for i in 0..2 {
            discovery.add_member(MemberInfo {
                name: format!("value_field_{}", i),
                member_type: MemberType::HashMap {
                    key: "String".to_string(),
                    value: "Vec<serde_json::Value>".to_string(),
                },
                required: false,
                is_nullable: false,
                schema: serde_json::Map::new(),
            });
        }

        assert_eq!(pass.should_use_bit_packing(&discovery), Some((4, 1)));
    }

    #[test]
    fn test_should_use_bit_packing_disabled() {
        use crate::generator::schema_discovery::{MemberInfo, MemberType, SchemaLocation};

        let mut config = create_test_config(None);
        config.bit_packing_enabled = false; // Disable bit-packing
        let pass = OptimizationPhase::new(config);

        // Create struct with 10 optional fields (would normally use bit-packing)
        let mut discovery = TypeDiscovery::new(
            "TestStruct".to_string(),
            TypeKind::Struct,
            SchemaLocation {
                json_path: "#/definitions/TestStruct".to_string(),
                description: "Test struct".to_string(),
                parent_type: None,
                property_name: None,
            },
            serde_json::Map::new(),
        );

        for i in 0..10 {
            discovery.add_member(MemberInfo {
                name: format!("field_{}", i),
                member_type: MemberType::Primitive("String".to_string()),
                required: false,
                is_nullable: false,
                schema: serde_json::Map::new(),
            });
        }

        // Should return None because bit-packing is disabled
        assert_eq!(pass.should_use_bit_packing(&discovery), None);
    }

    /// Helper to create integer type discovery with min/max bounds
    fn create_integer_discovery(name: &str, min: i64, max: i64) -> TypeDiscovery {
        use crate::generator::schema_discovery::{SchemaLocation, TypeKind};

        let mut schema = serde_json::Map::new();
        schema.insert("type".to_string(), serde_json::json!("integer"));
        schema.insert("minimum".to_string(), serde_json::json!(min));
        schema.insert("maximum".to_string(), serde_json::json!(max));

        TypeDiscovery::new(
            name.to_string(),
            TypeKind::PrimitiveWrapper {
                primitive_type: "i64".to_string(),
            },
            SchemaLocation {
                json_path: format!("#/definitions/{}", name),
                description: format!("Integer type {} [{}, {}]", name, min, max),
                parent_type: None,
                property_name: None,
            },
            schema,
        )
    }

    #[test]
    fn test_bounded_integer_no_constraint_offset_encoding() {
        let config = create_test_config(None);
        let pass = OptimizationPhase::new(config);

        // Integer range [0, 100) -> 100 values, no vocab constraint
        let discovery = create_integer_discovery("SmallInt", 0, 99);

        let constraints = crate::generator::encoding_decision::CompilationConstraints::default();
        let decision = pass.decide_bounded_integer_encoding("SmallInt", &discovery, &constraints);

        // Should use offset encoding (single token)
        match decision {
            EncodingDecision::BoundedIntegerOffset { min, max, range } => {
                assert_eq!(min, 0);
                assert_eq!(max, 99);
                assert_eq!(range, 100);
            }
            _ => panic!("Expected BoundedIntegerOffset, got {:?}", decision),
        }
    }

    #[test]
    fn test_bounded_integer_requires_bounded_integer_pass() {
        let config =
            OptimizationConfig::from_pass_names(&["enum_optimization".to_string()]).unwrap();
        let pass = OptimizationPhase::new(config);
        let discovery = create_integer_discovery("SmallInt", 0, 99);
        let constraints = crate::generator::encoding_decision::CompilationConstraints::default();

        let decision = pass.decide_bounded_integer_encoding("SmallInt", &discovery, &constraints);

        assert!(matches!(decision, EncodingDecision::Standard));
    }

    #[test]
    fn test_bounded_integer_range_within_vocab_budget() {
        let mut config = create_test_config(None);
        config.max_vocab = Some(1000); // Set vocab budget
        let pass = OptimizationPhase::new(config);

        // Integer range [0, 500) -> 500 values < 1000 max_vocab
        let discovery = create_integer_discovery("MediumInt", 0, 499);

        let constraints = crate::generator::encoding_decision::CompilationConstraints {
            max_vocab: Some(1000),
            ..Default::default()
        };
        let decision = pass.decide_bounded_integer_encoding("MediumInt", &discovery, &constraints);

        // Should use offset encoding (range fits in budget)
        match decision {
            EncodingDecision::BoundedIntegerOffset { min, max, range } => {
                assert_eq!(min, 0);
                assert_eq!(max, 499);
                assert_eq!(range, 500);
            }
            _ => panic!("Expected BoundedIntegerOffset, got {:?}", decision),
        }
    }

    #[test]
    fn test_bounded_integer_range_exceeds_vocab_budget_codec() {
        let mut config = create_test_config(None);
        config.max_vocab = Some(100); // Small vocab budget
        let pass = OptimizationPhase::new(config);

        // Integer range [0, 500) -> 500 values > 100 max_vocab
        let discovery = create_integer_discovery("LargeInt", 0, 499);

        let constraints = crate::generator::encoding_decision::CompilationConstraints {
            max_vocab: Some(100),
            ..Default::default()
        };
        let decision = pass.decide_bounded_integer_encoding("LargeInt", &discovery, &constraints);

        // Should use codec encoding (multi-token)
        match decision {
            EncodingDecision::BoundedIntegerCodec {
                min,
                max,
                codec_spec,
            } => {
                assert_eq!(min, 0);
                assert_eq!(max, 499);
                assert_eq!(codec_spec.base, 100); // Use max_vocab as base
                assert_eq!(codec_spec.max_value, 499);
                // 500 values with base 100: ceil(log_100(500)) = 2 tokens
                assert_eq!(codec_spec.token_width, 2);
            }
            _ => panic!("Expected BoundedIntegerCodec, got {:?}", decision),
        }
    }

    #[test]
    fn test_bounded_integer_negative_range() {
        let config = create_test_config(None);
        let pass = OptimizationPhase::new(config);

        // Negative range: [-100, 100] -> 201 values
        let discovery = create_integer_discovery("SignedInt", -100, 100);

        let constraints = crate::generator::encoding_decision::CompilationConstraints::default();
        let decision = pass.decide_bounded_integer_encoding("SignedInt", &discovery, &constraints);

        // Should use offset encoding with min=-100
        match decision {
            EncodingDecision::BoundedIntegerOffset { min, max, range } => {
                assert_eq!(min, -100);
                assert_eq!(max, 100);
                assert_eq!(range, 201); // 100 - (-100) + 1 = 201
            }
            _ => panic!("Expected BoundedIntegerOffset, got {:?}", decision),
        }
    }

    #[test]
    fn test_bounded_integer_single_value() {
        let config = create_test_config(None);
        let pass = OptimizationPhase::new(config);

        // Single value: [42, 42] -> 1 value
        let discovery = create_integer_discovery("ConstantInt", 42, 42);

        let constraints = crate::generator::encoding_decision::CompilationConstraints::default();
        let decision =
            pass.decide_bounded_integer_encoding("ConstantInt", &discovery, &constraints);

        // Should use offset encoding with range=1
        match decision {
            EncodingDecision::BoundedIntegerOffset { min, max, range } => {
                assert_eq!(min, 42);
                assert_eq!(max, 42);
                assert_eq!(range, 1);
            }
            _ => panic!("Expected BoundedIntegerOffset, got {:?}", decision),
        }
    }

    #[test]
    fn test_bounded_integer_overwide_range_uses_standard_encoding() {
        let config = create_test_config(None);
        let pass = OptimizationPhase::new(config);

        // Offset and codec encodings carry offsets as u32. Wider valid i64
        // ranges must not truncate values through the optimized path.
        let discovery = create_integer_discovery("OverwideInt", 0, 5_000_000_000);

        let constraints = crate::generator::encoding_decision::CompilationConstraints::default();
        let decision =
            pass.decide_bounded_integer_encoding("OverwideInt", &discovery, &constraints);

        assert!(matches!(decision, EncodingDecision::Standard));
    }

    #[test]
    fn test_bounded_integer_large_range_multi_token_codec() {
        let mut config = create_test_config(None);
        config.max_vocab = Some(256); // Base-256 encoding
        let pass = OptimizationPhase::new(config);

        // Very large range: [0, 100000) requires 3 tokens at base-256
        let discovery = create_integer_discovery("VeryLargeInt", 0, 99999);

        let constraints = crate::generator::encoding_decision::CompilationConstraints {
            max_vocab: Some(256),
            ..Default::default()
        };
        let decision =
            pass.decide_bounded_integer_encoding("VeryLargeInt", &discovery, &constraints);

        // Should use codec encoding with 3 tokens
        match decision {
            EncodingDecision::BoundedIntegerCodec {
                min,
                max,
                codec_spec,
            } => {
                assert_eq!(min, 0);
                assert_eq!(max, 99999);
                assert_eq!(codec_spec.base, 256);
                assert_eq!(codec_spec.max_value, 99999);
                // 100000 values with base 256: ceil(log_256(100000)) = 3 tokens
                // 256^2 = 65536 < 100000, 256^3 = 16777216 > 100000
                assert_eq!(codec_spec.token_width, 3);
            }
            _ => panic!("Expected BoundedIntegerCodec, got {:?}", decision),
        }
    }

    #[test]
    fn test_enum_encoding_no_optimization_passes() {
        let mut config = create_test_config(None);
        config.passes.clear(); // Remove all passes
        let pass = OptimizationPhase::new(config);

        let (name, discovery) = create_enum_discovery("Status", 10);
        let constraints = crate::generator::encoding_decision::CompilationConstraints::default();

        let decision =
            pass.decide_enum_encoding(&name, &get_variant_names(&discovery), &constraints);

        // With no optimization passes enabled, should return Standard
        assert!(matches!(decision, EncodingDecision::Standard));
    }

    #[test]
    fn test_enum_encoding_requires_enum_pass() {
        let config =
            OptimizationConfig::from_pass_names(&["integer_optimization".to_string()]).unwrap();
        let pass = OptimizationPhase::new(config);

        let (name, discovery) = create_enum_discovery("Status", 10);
        let constraints = crate::generator::encoding_decision::CompilationConstraints::default();

        let decision =
            pass.decide_enum_encoding(&name, &get_variant_names(&discovery), &constraints);

        assert!(matches!(decision, EncodingDecision::Standard));
    }

    #[test]
    fn test_enum_encoding_single_variant() {
        let config = create_test_config(None);
        let pass = OptimizationPhase::new(config);

        // Single variant enum (unusual but valid)
        let (name, discovery) = create_enum_discovery("SingleVariant", 1);
        let constraints = crate::generator::encoding_decision::CompilationConstraints::default();

        let decision =
            pass.decide_enum_encoding(&name, &get_variant_names(&discovery), &constraints);

        // Should use EnumIndex even for 1 variant
        assert!(matches!(decision, EncodingDecision::EnumIndex { .. }));
    }

    /// Helper to extract variant names from enum TypeDiscovery
    fn get_variant_names(discovery: &TypeDiscovery) -> Vec<String> {
        discovery.members.iter().map(|m| m.name.clone()).collect()
    }

    // === Phase 1: O3 Optimization Tests ===

    #[test]
    fn test_o3_single_variant_enum_optimization() {
        // Test O3 optimization: single-variant enum → constant (0 tokens)
        let config = crate::optimization_pass::OptimizationConfig::with_all_passes();
        let pass = OptimizationPhase::new(config);

        let mut discovery = SchemaDiscovery::new();
        let (name, type_disc) = create_enum_discovery("ConstantEnum", 1);
        discovery.discovered_types.insert(name.clone(), type_disc);

        let analysis = TypeAnalysis::default();
        let strategy = pass
            .run(&serde_json::json!({}), &discovery, &analysis)
            .unwrap();

        let decision = strategy.get_decision("ConstantEnum");
        // O3 should detect single-variant enum
        match decision {
            EncodingDecision::SingleVariantEnum { variant } => {
                assert_eq!(variant, "variant_0");
            }
            _ => panic!("Expected SingleVariantEnum at O3, got {:?}", decision),
        }
    }

    #[test]
    fn test_o3_constant_integer_optimization() {
        // Test O3 optimization: integer with min=max → constant (0 tokens)
        let config = crate::optimization_pass::OptimizationConfig::with_all_passes();
        let pass = OptimizationPhase::new(config);

        let mut discovery = SchemaDiscovery::new();
        // Create integer with minimum=42, maximum=42
        let type_disc = create_integer_discovery("ConstantInt", 42, 42);
        discovery
            .discovered_types
            .insert("ConstantInt".to_string(), type_disc);

        let analysis = TypeAnalysis::default();
        let strategy = pass
            .run(&serde_json::json!({}), &discovery, &analysis)
            .unwrap();

        let decision = strategy.get_decision("ConstantInt");
        // O3 should detect constant integer
        match decision {
            EncodingDecision::ConstantInteger { value } => {
                assert_eq!(value, 42);
            }
            _ => panic!("Expected ConstantInteger at O3, got {:?}", decision),
        }
    }

    #[test]
    fn test_o3_format_optimized_string_int32() {
        // Test O3 optimization: string with format="int32" → specialized encoding
        let config = crate::optimization_pass::OptimizationConfig::with_all_passes();
        let pass = OptimizationPhase::new(config);

        let mut discovery = SchemaDiscovery::new();
        let type_disc = create_string_with_format_discovery("FormattedInt", "int32");
        discovery
            .discovered_types
            .insert("FormattedInt".to_string(), type_disc);

        let analysis = TypeAnalysis::default();
        let strategy = pass
            .run(&serde_json::json!({}), &discovery, &analysis)
            .unwrap();

        let decision = strategy.get_decision("FormattedInt");
        // O3 should detect format-optimized string
        match decision {
            EncodingDecision::FormatOptimized { format, .. } => {
                assert!(matches!(
                    format,
                    crate::generator::encoding_decision::StringFormat::Int32
                ));
            }
            _ => panic!("Expected FormatOptimized at O3, got {:?}", decision),
        }
    }

    #[test]
    fn test_o3_format_optimized_string_datetime() {
        // Test O3 optimization: string with format="date-time" → Unix timestamp encoding
        let config = crate::optimization_pass::OptimizationConfig::with_all_passes();
        let pass = OptimizationPhase::new(config);

        let mut discovery = SchemaDiscovery::new();
        let type_disc = create_string_with_format_discovery("Timestamp", "date-time");
        discovery
            .discovered_types
            .insert("Timestamp".to_string(), type_disc);

        let analysis = TypeAnalysis::default();
        let strategy = pass
            .run(&serde_json::json!({}), &discovery, &analysis)
            .unwrap();

        let decision = strategy.get_decision("Timestamp");
        match decision {
            EncodingDecision::FormatOptimized { format, .. } => {
                assert!(matches!(
                    format,
                    crate::generator::encoding_decision::StringFormat::DateTime
                ));
            }
            _ => panic!("Expected FormatOptimized for date-time, got {:?}", decision),
        }
    }

    #[test]
    fn test_os_profile_preserves_annotation_only_format_strings() {
        let config = crate::optimization_pass::OptimizationConfig::os();
        let pass = OptimizationPhase::new(config);

        let mut discovery = SchemaDiscovery::new();
        let type_disc = create_string_with_format_discovery("Timestamp", "date-time");
        discovery
            .discovered_types
            .insert("Timestamp".to_string(), type_disc);

        let analysis = TypeAnalysis::default();
        let strategy = pass
            .run(&serde_json::json!({}), &discovery, &analysis)
            .unwrap();

        assert!(
            !matches!(
                strategy.get_decision("Timestamp"),
                EncodingDecision::FormatOptimized { .. }
            ),
            "Os must not make JSON Schema format annotations stricter than O2"
        );
    }

    #[test]
    fn test_ovocab_profile_keeps_safe_constant_optimization_without_format_optimization() {
        let config = crate::optimization_pass::OptimizationConfig::ovocab();
        let pass = OptimizationPhase::new(config);

        let mut discovery = SchemaDiscovery::new();
        discovery.discovered_types.insert(
            "ConstantInt".to_string(),
            create_integer_discovery("ConstantInt", 42, 42),
        );
        discovery.discovered_types.insert(
            "Timestamp".to_string(),
            create_string_with_format_discovery("Timestamp", "date-time"),
        );

        let analysis = TypeAnalysis::default();
        let strategy = pass
            .run(&serde_json::json!({}), &discovery, &analysis)
            .unwrap();

        match strategy.get_decision("ConstantInt") {
            EncodingDecision::ConstantInteger { value } => assert_eq!(value, 42),
            decision => panic!("Expected ConstantInteger at OVocab, got {:?}", decision),
        }

        assert!(
            !matches!(
                strategy.get_decision("Timestamp"),
                EncodingDecision::FormatOptimized { .. }
            ),
            "OVocab should not apply token-count-only format optimization"
        );
    }

    #[test]
    fn test_o3_format_optimized_string_uuid() {
        // Test O3 optimization: string with format="uuid"
        let config = crate::optimization_pass::OptimizationConfig::with_all_passes();
        let pass = OptimizationPhase::new(config);

        let mut discovery = SchemaDiscovery::new();
        let type_disc = create_string_with_format_discovery("Identifier", "uuid");
        discovery
            .discovered_types
            .insert("Identifier".to_string(), type_disc);

        let analysis = TypeAnalysis::default();
        let strategy = pass
            .run(&serde_json::json!({}), &discovery, &analysis)
            .unwrap();

        let decision = strategy.get_decision("Identifier");
        match decision {
            EncodingDecision::FormatOptimized { format, .. } => {
                assert!(matches!(
                    format,
                    crate::generator::encoding_decision::StringFormat::Uuid
                ));
            }
            _ => panic!("Expected FormatOptimized for uuid, got {:?}", decision),
        }
    }

    // === Phase 2: String Format Parsing Tests ===

    #[test]
    fn test_parse_string_format_all_variants() {
        // Test parse_string_format with all known format strings
        let config = crate::optimization_pass::OptimizationConfig::with_all_passes();
        let pass = OptimizationPhase::new(config);

        use crate::generator::encoding_decision::StringFormat;

        // Test known formats
        assert!(matches!(
            pass.parse_string_format("int32"),
            Some(StringFormat::Int32)
        ));
        assert!(matches!(
            pass.parse_string_format("int64"),
            Some(StringFormat::Int64)
        ));
        assert!(matches!(
            pass.parse_string_format("double"),
            Some(StringFormat::Double)
        ));
        assert!(matches!(
            pass.parse_string_format("float"),
            Some(StringFormat::Float)
        ));
        assert!(matches!(
            pass.parse_string_format("byte"),
            Some(StringFormat::Byte)
        ));
        assert!(matches!(
            pass.parse_string_format("date-time"),
            Some(StringFormat::DateTime)
        ));
        assert!(matches!(
            pass.parse_string_format("datetime"),
            Some(StringFormat::DateTime)
        ));
        assert!(matches!(
            pass.parse_string_format("date"),
            Some(StringFormat::Date)
        ));
        assert!(matches!(
            pass.parse_string_format("time"),
            Some(StringFormat::Time)
        ));
        assert!(matches!(
            pass.parse_string_format("duration"),
            Some(StringFormat::Duration)
        ));
        assert!(matches!(
            pass.parse_string_format("uuid"),
            Some(StringFormat::Uuid)
        ));
        assert!(matches!(
            pass.parse_string_format("email"),
            Some(StringFormat::Email)
        ));
        assert!(matches!(
            pass.parse_string_format("idn-email"),
            Some(StringFormat::Email)
        ));
        assert!(matches!(
            pass.parse_string_format("uri"),
            Some(StringFormat::Uri)
        ));
        assert!(matches!(
            pass.parse_string_format("ipv4"),
            Some(StringFormat::Ipv4)
        ));
        assert!(matches!(
            pass.parse_string_format("ipv6"),
            Some(StringFormat::Ipv6)
        ));
        assert!(matches!(
            pass.parse_string_format("hostname"),
            Some(StringFormat::Hostname)
        ));

        // Test unknown format
        assert!(pass.parse_string_format("unknown-format").is_none());
    }

    // === Phase 3: Bit-Packing Tests ===

    #[test]
    fn test_bit_packing_disabled() {
        // Test that bit-packing is disabled when config.bit_packing_enabled = false
        let mut config = crate::optimization_pass::OptimizationConfig::o2();
        config.bit_packing_enabled = false;
        let pass = OptimizationPhase::new(config);

        // Create struct with 8 optional fields (enough for bit-packing normally)
        let type_disc = create_struct_with_optional_fields("BitPackable", 8);

        // should_use_bit_packing should return None when disabled
        assert!(pass.should_use_bit_packing(&type_disc).is_none());
    }

    #[test]
    fn test_bit_packing_below_threshold() {
        // Test that bit-packing is NOT used below 4 optional fields threshold
        let config = crate::optimization_pass::OptimizationConfig::o2();
        let pass = OptimizationPhase::new(config);

        // 3 optional fields - below threshold
        let type_disc = create_struct_with_optional_fields("SmallStruct", 3);
        assert!(pass.should_use_bit_packing(&type_disc).is_none());
    }

    #[test]
    fn test_bit_packing_at_threshold() {
        // Test that bit-packing IS used at exactly 4 optional fields
        let config = crate::optimization_pass::OptimizationConfig::o2().with_bit_packing(true);
        let pass = OptimizationPhase::new(config);

        // 4 optional fields - at threshold
        let type_disc = create_struct_with_optional_fields("ThresholdStruct", 4);
        let result = pass.should_use_bit_packing(&type_disc);

        assert!(result.is_some());
        let (count, presence_mask_groups) = result.unwrap();
        assert_eq!(count, 4);
        assert_eq!(presence_mask_groups, 1); // ceil(4 / 32) = 1
    }

    #[test]
    fn test_bit_packing_large_struct() {
        // Test bit-packing with many optional fields
        let config = crate::optimization_pass::OptimizationConfig::o2().with_bit_packing(true);
        let pass = OptimizationPhase::new(config);

        // 64 optional fields - requires 2 u32 presence-mask groups.
        let type_disc = create_struct_with_optional_fields("LargeStruct", 64);
        let result = pass.should_use_bit_packing(&type_disc);

        assert!(result.is_some());
        let (count, presence_mask_groups) = result.unwrap();
        assert_eq!(count, 64);
        assert_eq!(presence_mask_groups, 2); // ceil(64 / 32) = 2
    }

    #[test]
    fn test_bit_packing_with_nullable_fields() {
        // Test that nullable fields (is_nullable=true) are NOT counted for bit-packing
        let config = crate::optimization_pass::OptimizationConfig::o2();
        let pass = OptimizationPhase::new(config);

        // Create struct with nullable fields (use OptionalField, not Option)
        let type_disc = create_struct_with_nullable_fields("NullableStruct", 8);

        // Nullable fields use 3-state OptionalField<T>, can't be bit-packed
        assert!(pass.should_use_bit_packing(&type_disc).is_none());
    }

    // === Helper Functions for New Tests ===

    fn create_string_with_format_discovery(name: &str, format: &str) -> TypeDiscovery {
        use crate::generator::schema_discovery::{SchemaLocation, TypeKind};
        use serde_json::json;

        let mut schema_object = serde_json::Map::new();
        schema_object.insert("type".to_string(), json!("string"));
        schema_object.insert("format".to_string(), json!(format));

        TypeDiscovery::new(
            name.to_string(),
            TypeKind::PrimitiveWrapper {
                primitive_type: "String".to_string(),
            },
            SchemaLocation {
                json_path: format!("#/definitions/{}", name),
                description: format!("Test string with format {}", format),
                parent_type: None,
                property_name: None,
            },
            schema_object,
        )
    }

    fn create_struct_with_optional_fields(name: &str, optional_count: usize) -> TypeDiscovery {
        use crate::generator::schema_discovery::{
            MemberInfo, MemberType, SchemaLocation, TypeKind,
        };

        let mut discovery = TypeDiscovery::new(
            name.to_string(),
            TypeKind::Struct,
            SchemaLocation {
                json_path: format!("#/definitions/{}", name),
                description: format!("Test struct with {} optional fields", optional_count),
                parent_type: None,
                property_name: None,
            },
            serde_json::Map::new(),
        );

        // Add optional fields (!required && !is_nullable → uses Option<T>)
        for i in 0..optional_count {
            discovery.add_member(MemberInfo {
                name: format!("field_{}", i),
                member_type: MemberType::Primitive("String".to_string()),
                required: false,    // Not required
                is_nullable: false, // Not nullable (uses Option<T>, not OptionalField<T>)
                schema: serde_json::Map::new(),
            });
        }

        discovery
    }

    fn create_struct_with_nullable_fields(name: &str, nullable_count: usize) -> TypeDiscovery {
        use crate::generator::schema_discovery::{
            MemberInfo, MemberType, SchemaLocation, TypeKind,
        };

        let mut discovery = TypeDiscovery::new(
            name.to_string(),
            TypeKind::Struct,
            SchemaLocation {
                json_path: format!("#/definitions/{}", name),
                description: format!("Test struct with {} nullable fields", nullable_count),
                parent_type: None,
                property_name: None,
            },
            serde_json::Map::new(),
        );

        // Add nullable fields (is_nullable=true → uses OptionalField<T>)
        for i in 0..nullable_count {
            discovery.add_member(MemberInfo {
                name: format!("nullable_field_{}", i),
                member_type: MemberType::Primitive("String".to_string()),
                required: false,   // Not required
                is_nullable: true, // Nullable (uses OptionalField<T>, can't be bit-packed)
                schema: serde_json::Map::new(),
            });
        }

        discovery
    }
}
