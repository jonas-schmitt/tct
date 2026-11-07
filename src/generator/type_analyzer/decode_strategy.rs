//! Decode-strategy analysis for discovered JSON Schema types.
//!
//! This pass records the token-order and structure metadata that later decoder
//! generators consume. It deliberately stays in the analysis layer: decisions
//! made here describe schema-derived decode contracts but do not render Rust
//! source or inspect generated code.

use super::TypeAnalyzer;
use crate::TctResult;
use crate::decoder::TokenStructure;
use crate::generator::schema_discovery::{MemberType, TypeDiscovery, TypeKind};
use crate::generator::type_analysis::{DecodeConstraints, DecodeStrategy};

impl TypeAnalyzer {
    pub(super) fn compute_decode_strategies(&mut self) -> TctResult<()> {
        for (type_name, type_discovery) in &self.discovery.discovered_types {
            let strategy = self.compute_decode_strategy(type_name, type_discovery)?;
            self.analysis
                .decode_strategies
                .insert(type_name.clone(), strategy);
        }
        Ok(())
    }

    /// Compute decode strategy for a specific type
    pub(super) fn compute_decode_strategy(
        &self,
        type_name: &str,
        type_def: &TypeDiscovery,
    ) -> TctResult<DecodeStrategy> {
        match &type_def.type_kind {
            TypeKind::Enum => {
                crate::log_debug!("🎯 Computing enum decode strategy for {}", type_name);

                Ok(DecodeStrategy {
                    field_order: vec![],
                    item_strategy: None,
                    constraints: DecodeConstraints::default(),
                })
            }
            TypeKind::Struct | TypeKind::NestedStruct => {
                crate::log_debug!("🎯 Computing struct decode strategy for {}", type_name);
                let field_order = self.compute_struct_field_order(type_def);

                let total_fields = type_def.members.len();
                let required_fields = type_def.members.iter().filter(|m| m.required).count();

                if total_fields == 0 {
                    // Empty structs encode as zero tokens → Null structure
                    Ok(DecodeStrategy {
                        field_order,
                        item_strategy: None,
                        constraints: DecodeConstraints::default(),
                    })
                } else if required_fields == total_fields {
                    // All fields required - use traditional approach
                    Ok(DecodeStrategy {
                        field_order,
                        item_strategy: None,
                        constraints: DecodeConstraints::default(),
                    })
                } else {
                    // Has optional fields. field_order is the emitted token order
                    // contract, not an independent decode optimization order.
                    crate::log_debug!(
                        "🔀 Optional fields present for {} (required: {}, total: {})",
                        type_name,
                        required_fields,
                        total_fields
                    );

                    Ok(DecodeStrategy {
                        field_order,
                        item_strategy: None,
                        constraints: DecodeConstraints::default(),
                    })
                }
            }
            TypeKind::Reference { target } => {
                crate::log_debug!(
                    "🎯 Computing reference decode strategy for {} -> {}",
                    type_name,
                    target
                );
                // For references, use a simple structure
                Ok(DecodeStrategy {
                    field_order: vec![],
                    item_strategy: None,
                    constraints: DecodeConstraints::default(),
                })
            }
            TypeKind::ArrayOfComplex => {
                crate::log_debug!("🎯 Computing array decode strategy for {}", type_name);
                Ok(DecodeStrategy {
                    field_order: vec![],
                    item_strategy: None, // Could be enhanced to analyze array item type
                    constraints: DecodeConstraints::default(),
                })
            }
            TypeKind::PrimitiveWrapper { primitive_type } => {
                crate::log_debug!(
                    "🎯 Computing primitive wrapper decode strategy for {} ({})",
                    type_name,
                    primitive_type
                );
                Ok(DecodeStrategy {
                    field_order: vec![],
                    item_strategy: None,
                    constraints: DecodeConstraints::default(),
                })
            }
            TypeKind::MinimalObjectWrapper => {
                crate::log_debug!(
                    "🎯 Computing minimal object wrapper decode strategy for {}",
                    type_name
                );
                Ok(DecodeStrategy {
                    field_order: vec![],
                    item_strategy: None,
                    constraints: DecodeConstraints::default(),
                })
            }
        }
    }

    /// Compute emitted field order for struct encoding/decoding.
    fn compute_struct_field_order(&self, type_def: &TypeDiscovery) -> Vec<String> {
        // Use the natural discovered order. Reordering is a token-layout
        // compatibility decision and must update encode/decode together.
        type_def.members.iter().map(|m| m.name.clone()).collect()
    }

    /// Determine the expected token structure for a member type
    pub fn determine_member_token_structure(
        &self,
        member_type: &MemberType,
    ) -> TctResult<TokenStructure> {
        match member_type {
            MemberType::Primitive(rust_type) => {
                // Map Rust primitive types to token structures
                Ok(self.map_primitive_to_token_structure(rust_type))
            }
            MemberType::Container { container, inner } => {
                match container.as_str() {
                    "Vec" => Ok(TokenStructure::Array { length: 0 }),
                    "HashMap" | "BTreeMap" => Ok(TokenStructure::Object { field_count: 0 }),
                    "Option" => {
                        // Option wraps another type - check if it's a complex type first
                        if let Some(inner_type_def) = self.discovery.discovered_types.get(inner) {
                            match &inner_type_def.type_kind {
                                TypeKind::Struct | TypeKind::NestedStruct => {
                                    let total_fields = inner_type_def.members.len();
                                    let required_fields = inner_type_def
                                        .members
                                        .iter()
                                        .filter(|m| m.required)
                                        .count();

                                    // For enum variants, use minimum field count as canonical representation
                                    let field_count = if required_fields == 0 && total_fields > 0 {
                                        // All fields optional - use 1 as minimum practical count
                                        1
                                    } else {
                                        required_fields.max(1)
                                    };

                                    if total_fields == 0 {
                                        Ok(TokenStructure::Null)
                                    } else {
                                        Ok(TokenStructure::Object { field_count })
                                    }
                                }
                                TypeKind::Enum => {
                                    if self.should_enum_optimize_to_primitive(inner_type_def) {
                                        Ok(TokenStructure::Primitive(0))
                                    } else {
                                        Ok(TokenStructure::Array { length: 0 })
                                    }
                                }
                                TypeKind::Reference { .. } => Ok(TokenStructure::Primitive(0)),
                                TypeKind::ArrayOfComplex => Ok(TokenStructure::Array { length: 0 }),
                                TypeKind::PrimitiveWrapper { primitive_type } => {
                                    Ok(self.map_primitive_to_token_structure(primitive_type))
                                }
                                TypeKind::MinimalObjectWrapper => {
                                    Ok(TokenStructure::Object { field_count: 0 })
                                }
                            }
                        } else {
                            // Fall back to primitive mapping
                            Ok(self.map_primitive_to_token_structure(inner))
                        }
                    }
                    "Box" => {
                        // Box wraps another type - check if it's a complex type first
                        if let Some(inner_type_def) = self.discovery.discovered_types.get(inner) {
                            match &inner_type_def.type_kind {
                                TypeKind::Struct | TypeKind::NestedStruct => {
                                    let total_fields = inner_type_def.members.len();
                                    let required_fields = inner_type_def
                                        .members
                                        .iter()
                                        .filter(|m| m.required)
                                        .count();

                                    // For enum variants, use minimum field count as canonical representation
                                    let field_count = if required_fields == 0 && total_fields > 0 {
                                        // All fields optional - use 1 as minimum practical count
                                        1
                                    } else {
                                        required_fields.max(1)
                                    };

                                    if total_fields == 0 {
                                        Ok(TokenStructure::Null)
                                    } else {
                                        Ok(TokenStructure::Object { field_count })
                                    }
                                }
                                TypeKind::Enum => {
                                    if self.should_enum_optimize_to_primitive(inner_type_def) {
                                        Ok(TokenStructure::Primitive(0))
                                    } else {
                                        Ok(TokenStructure::Array { length: 0 })
                                    }
                                }
                                TypeKind::Reference { .. } => Ok(TokenStructure::Primitive(0)),
                                TypeKind::ArrayOfComplex => Ok(TokenStructure::Array { length: 0 }),
                                TypeKind::PrimitiveWrapper { primitive_type } => {
                                    Ok(self.map_primitive_to_token_structure(primitive_type))
                                }
                                TypeKind::MinimalObjectWrapper => {
                                    Ok(TokenStructure::Object { field_count: 0 })
                                }
                            }
                        } else {
                            // Fall back to primitive mapping
                            Ok(self.map_primitive_to_token_structure(inner))
                        }
                    }
                    _ => {
                        crate::log_debug!("⚠️ Unknown container type: {}", container);
                        Ok(TokenStructure::Primitive(0)) // Safe fallback
                    }
                }
            }
            MemberType::TypeReference(type_name) => {
                // Look up the referenced type to determine its structure
                if let Some(type_def) = self.discovery.discovered_types.get(type_name) {
                    match &type_def.type_kind {
                        TypeKind::Struct | TypeKind::NestedStruct => {
                            let total_fields = type_def.members.len();
                            let required_fields =
                                type_def.members.iter().filter(|m| m.required).count();

                            // For enum variants, use minimum field count as canonical representation
                            let field_count = if required_fields == 0 && total_fields > 0 {
                                // All fields optional - use 1 as minimum practical count
                                1
                            } else {
                                required_fields.max(1)
                            };

                            if total_fields == 0 {
                                // Empty structs encode as zero tokens → Null structure
                                Ok(TokenStructure::Null)
                            } else {
                                Ok(TokenStructure::Object { field_count })
                            }
                        }
                        TypeKind::Enum => {
                            if self.should_enum_optimize_to_primitive(type_def) {
                                Ok(TokenStructure::Primitive(0))
                            } else {
                                // Data variant enums use discriminator-prefixed encoding
                                Ok(TokenStructure::Array { length: 0 })
                            }
                        }
                        TypeKind::Reference { .. } => {
                            // Reference to reference - treat as primitive for safety
                            Ok(TokenStructure::Primitive(0))
                        }
                        TypeKind::ArrayOfComplex => Ok(TokenStructure::Array { length: 0 }),
                        TypeKind::PrimitiveWrapper { primitive_type } => {
                            Ok(self.map_primitive_to_token_structure(primitive_type))
                        }
                        TypeKind::MinimalObjectWrapper => {
                            Ok(TokenStructure::Object { field_count: 0 })
                        }
                    }
                } else {
                    crate::log_debug!("⚠️ Type reference not found: {}", type_name);
                    Ok(TokenStructure::Primitive(0)) // Safe fallback
                }
            }
            MemberType::ComplexContainer { container, inner } => {
                // Complex containers like Vec<MyType>, Option<MyStruct>
                match container.as_str() {
                    "Vec" => Ok(TokenStructure::Array { length: 0 }),
                    "HashMap" | "BTreeMap" => Ok(TokenStructure::Object { field_count: 0 }),
                    "Option" | "Box" => {
                        // For Option/Box of complex types, analyze the inner type
                        if let Some(inner_type_def) = self.discovery.discovered_types.get(inner) {
                            match &inner_type_def.type_kind {
                                TypeKind::Struct | TypeKind::NestedStruct => {
                                    let field_count = inner_type_def.members.len();
                                    if field_count == 0 {
                                        // Empty structs encode as zero tokens → Null structure
                                        Ok(TokenStructure::Null)
                                    } else {
                                        Ok(TokenStructure::Object { field_count })
                                    }
                                }
                                TypeKind::Enum => {
                                    if self.should_enum_optimize_to_primitive(inner_type_def) {
                                        Ok(TokenStructure::Primitive(0))
                                    } else {
                                        // Data variant enums use discriminator-prefixed encoding
                                        Ok(TokenStructure::Array { length: 0 })
                                    }
                                }
                                TypeKind::Reference { .. } => {
                                    // Reference to reference - treat as primitive for safety
                                    Ok(TokenStructure::Primitive(0))
                                }
                                TypeKind::ArrayOfComplex => Ok(TokenStructure::Array { length: 0 }),
                                TypeKind::PrimitiveWrapper { primitive_type } => {
                                    Ok(self.map_primitive_to_token_structure(primitive_type))
                                }
                                TypeKind::MinimalObjectWrapper => {
                                    Ok(TokenStructure::Object { field_count: 0 })
                                }
                            }
                        } else {
                            crate::log_debug!(
                                "⚠️ Complex container inner type not found: {}",
                                inner
                            );
                            Ok(TokenStructure::Primitive(0)) // Safe fallback
                        }
                    }
                    _ => {
                        crate::log_debug!("⚠️ Unknown complex container type: {}", container);
                        Ok(TokenStructure::Primitive(0))
                    }
                }
            }
            MemberType::HashMap { key: _, value: _ } => {
                // HashMap types always map to object structures
                Ok(TokenStructure::Object { field_count: 0 })
            }
            MemberType::PatternProperties { validators: _ } => {
                // Pattern properties are typically object-like structures
                Ok(TokenStructure::Object { field_count: 0 })
            }
        }
    }

    /// Check if an enum should be optimized to a primitive token
    /// This applies to simple enums (unit variants) when enum optimization is enabled
    #[allow(deprecated)]
    pub(super) fn should_enum_optimize_to_primitive(&self, type_def: &TypeDiscovery) -> bool {
        // Check if enum optimization is enabled
        let enum_opt_enabled = self
            .optimization_config
            .passes
            .iter()
            .any(|pass| pass.name() == "enum_index_optimization");

        if !enum_opt_enabled {
            return false;
        }

        // Check if this is a simple enum (all variants are unit types)
        let is_simple_enum = type_def.members.iter().all(|member| {
            matches!(member.member_type, crate::generator::schema_discovery::MemberType::Primitive(ref t) if t == "()")
        });

        crate::log_debug!(
            "🔍 Enum optimization check: {} (simple: {}, opt_enabled: {})",
            type_def
                .members
                .first()
                .map(|m| m.name.as_str())
                .unwrap_or("unknown"),
            is_simple_enum,
            enum_opt_enabled
        );

        is_simple_enum
    }

    /// Map Rust primitive type strings to TokenStructure
    pub(super) fn map_primitive_to_token_structure(&self, rust_type: &str) -> TokenStructure {
        match rust_type {
            "String" | "&str" | "str" => TokenStructure::String { length: 0 },
            "bool" | "u8" | "u16" | "u32" | "u64" | "u128" | "usize" | "i8" | "i16" | "i32"
            | "i64" | "i128" | "isize" | "f32" | "f64" | "JsonInteger" => {
                TokenStructure::Primitive(0)
            }
            "()" => TokenStructure::Null,
            _ => {
                crate::log_debug!("⚠️ Unknown primitive type: {}", rust_type);
                TokenStructure::Primitive(0) // Safe fallback
            }
        }
    }
}
