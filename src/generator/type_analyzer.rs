//! Type Analyzer - Pure Analysis Pass
//!
//! This module implements pure type analysis that processes SchemaDiscovery results
//! and produces TypeAnalysis data structures without generating any code.
//!
//! This is the SECOND pass in our multi-pass compilation system:
//! SchemaWalker → **TypeAnalyzer** → CodeGenerator

use crate::TctResult;
use crate::generator::schema_discovery::{MemberType, SchemaDiscovery};
use crate::generator::type_analysis::TypeAnalysis;
use crate::optimization_pass::OptimizationConfig;
use std::collections::HashSet;

mod decode_strategy;
mod tarjan;
mod trait_derivability;
use tarjan::TarjanScc;

/// Pure type analyzer - NO code generation, ONLY analysis
pub struct TypeAnalyzer {
    /// Schema discovery input
    discovery: SchemaDiscovery,
    /// Analysis results being built
    analysis: TypeAnalysis,
    /// Optimization configuration for optimization-aware analysis
    optimization_config: OptimizationConfig,
}

impl TypeAnalyzer {
    /// Create new type analyzer with optimization configuration
    pub fn new(discovery: SchemaDiscovery, optimization_config: OptimizationConfig) -> Self {
        Self {
            discovery,
            analysis: TypeAnalysis::new(),
            optimization_config,
        }
    }

    /// Analyze discovered types and return analysis results
    pub fn analyze(
        discovery: SchemaDiscovery,
        optimization_config: OptimizationConfig,
    ) -> TctResult<TypeAnalysis> {
        crate::log_debug!(
            "🧠 Starting TypeAnalyzer with {} discovered types (optimization level: {:?})",
            discovery.discovered_types.len(),
            optimization_config.level
        );

        let mut analyzer = Self::new(discovery, optimization_config);

        // Step 1: Build dependency graph from discovered types
        crate::log_debug!("🕸️  Step 1: Building dependency graph");
        analyzer.build_dependency_graph()?;
        crate::log_debug!(
            "✅ Dependency graph built: {} dependencies total",
            analyzer.analysis.stats.total_dependencies
        );

        // Step 2: Detect cycles using Tarjan's algorithm
        crate::log_debug!("🔍 Step 2: Detecting cycles using Tarjan's SCC algorithm");
        analyzer.detect_cycles()?;
        crate::log_debug!(
            "✅ Cycle detection completed: {} cycles found, {} types need boxing",
            analyzer.analysis.stats.cycles_detected,
            analyzer.analysis.stats.types_requiring_boxing
        );

        // Log cycles if found
        if !analyzer.analysis.dependency_graph.cycles.is_empty() {
            crate::log_debug!("🔄 Detected cycles:");
            for (i, cycle) in analyzer.analysis.dependency_graph.cycles.iter().enumerate() {
                crate::log_debug!("   Cycle {}: {:?}", i + 1, cycle);
            }
        }

        // Step 3: Run mathematical trait derivability analysis
        crate::log_debug!("🧮 Step 3: Mathematical trait derivability analysis");
        analyzer.analyze_trait_derivability()?;
        crate::log_debug!(
            "✅ Trait analysis completed: {}/{} types can derive Eq, {}/{} can derive Ord, {}/{} can derive Hash",
            analyzer.analysis.stats.eq_derivable,
            analyzer.analysis.stats.total_types,
            analyzer.analysis.stats.ord_derivable,
            analyzer.analysis.stats.total_types,
            analyzer.analysis.stats.hash_derivable,
            analyzer.analysis.stats.total_types
        );

        // Step 4: Compute type generation order (topological sort)
        crate::log_debug!("📊 Step 4: Computing type generation order");
        analyzer.compute_generation_order()?;
        crate::log_debug!(
            "✅ Generation order computed: {} types ordered",
            analyzer.analysis.generation_order.len()
        );

        // Log generation order
        crate::log_debug!("📋 Type generation order:");
        for (i, type_name) in analyzer.analysis.generation_order.iter().enumerate() {
            crate::log_debug!("   {}. {}", i + 1, type_name);
        }

        // Step 5: Compute decode strategies for schema-aware decoding
        crate::log_debug!("🎯 Step 5: Computing decode strategies for schema-aware decoding");
        analyzer.compute_decode_strategies()?;
        crate::log_debug!(
            "✅ Decode strategies computed: {} strategies generated",
            analyzer.analysis.decode_strategies.len()
        );

        // Step 6: Finalize statistics
        analyzer.finalize_statistics();

        crate::log_debug!("✅ TypeAnalyzer completed successfully!");
        crate::log_debug!("📊 Analysis Statistics:");
        crate::log_debug!("   • Total types: {}", analyzer.analysis.stats.total_types);
        crate::log_debug!(
            "   • Eq derivable: {}",
            analyzer.analysis.stats.eq_derivable
        );
        crate::log_debug!(
            "   • Ord derivable: {}",
            analyzer.analysis.stats.ord_derivable
        );
        crate::log_debug!(
            "   • Hash derivable: {}",
            analyzer.analysis.stats.hash_derivable
        );
        crate::log_debug!(
            "   • Cycles detected: {}",
            analyzer.analysis.stats.cycles_detected
        );
        crate::log_debug!(
            "   • Types requiring boxing: {}",
            analyzer.analysis.stats.types_requiring_boxing
        );
        crate::log_debug!(
            "   • Max dependency depth: {}",
            analyzer.analysis.stats.max_dependency_depth
        );
        crate::log_debug!(
            "   • Algorithmic decisions: {}",
            analyzer.analysis.stats.algorithmic_decisions
        );
        crate::log_debug!(
            "   • Heuristic decisions: {}",
            analyzer.analysis.stats.heuristic_decisions
        );
        crate::log_debug!(
            "   • Decode strategies: {}",
            analyzer.analysis.decode_strategies.len()
        );

        Ok(analyzer.analysis)
    }

    /// Build dependency graph from discovered types
    fn build_dependency_graph(&mut self) -> TctResult<()> {
        // Build dependencies based on direct_references in TypeDiscovery
        for (type_name, type_discovery) in &self.discovery.discovered_types {
            // Add dependencies for each type this type references
            for referenced_type in &type_discovery.direct_references {
                self.analysis
                    .dependency_graph
                    .add_dependency(type_name.clone(), referenced_type.clone());
            }

            // Also analyze member types for additional dependencies
            for member in &type_discovery.members {
                if let Some(dep_type) = self.extract_dependency_from_member(&member.member_type) {
                    self.analysis
                        .dependency_graph
                        .add_dependency(type_name.clone(), dep_type);
                }
            }
        }

        // Update statistics
        self.analysis.stats.total_dependencies = self
            .analysis
            .dependency_graph
            .dependencies
            .values()
            .map(|deps| deps.len())
            .sum();

        Ok(())
    }

    /// Extract type dependency from member type
    fn extract_dependency_from_member(&self, member_type: &MemberType) -> Option<String> {
        match member_type {
            MemberType::TypeReference(type_name) => Some(type_name.clone()),
            MemberType::ComplexContainer { inner, .. } => Some(inner.clone()),
            MemberType::Primitive(_) => None,
            MemberType::Container { .. } => None, // Containers of primitives have no deps
            MemberType::HashMap { .. } => None, // HashMap has no type dependencies (keys/values are primitives or serde_json::Value)
            MemberType::PatternProperties { .. } => None, // Pattern validators will be analyzed separately as their own types
        }
    }

    /// Detect cycles using Tarjan's strongly connected components algorithm
    fn detect_cycles(&mut self) -> TctResult<()> {
        let mut tarjan = TarjanScc::new(&self.analysis.dependency_graph.dependencies);
        let sccs = tarjan.find_sccs();

        // Identify cycles (SCCs with more than one node or self-loops)
        for scc in sccs {
            if scc.len() > 1 || self.has_self_loop(&scc[0]) {
                self.analysis.dependency_graph.cycles.push(scc.clone());

                // Mark all types in this cycle as needing boxing
                for type_name in &scc {
                    self.analysis.boxed_types.insert(type_name.clone());
                }
            }
        }

        // Update statistics
        self.analysis.stats.cycles_detected = self.analysis.dependency_graph.cycles.len();
        self.analysis.stats.types_requiring_boxing = self.analysis.boxed_types.len();

        Ok(())
    }

    /// Check if a type has a self-loop (direct recursive reference)
    fn has_self_loop(&self, type_name: &str) -> bool {
        if let Some(deps) = self.analysis.dependency_graph.dependencies.get(type_name) {
            deps.contains(type_name)
        } else {
            false
        }
    }

    /// Compute type generation order using topological sort
    fn compute_generation_order(&mut self) -> TctResult<()> {
        let mut ordered_types = Vec::new();
        let mut visited = HashSet::new();
        let mut temp_visited = HashSet::new();

        // Perform DFS-based topological sort
        // Skip root composition types as they're handled separately by generate_root()
        //
        // Start from the types in name order: the generated file lists types in this
        // order, so it has to be the same on every run, not whatever order a hash map
        // happens to iterate in.
        let mut type_names: Vec<&String> = self.discovery.discovered_types.keys().collect();
        type_names.sort();
        for type_name in type_names {
            // Skip root composition types - they'll be generated by generate_root()
            let should_skip = if let Some(ref root_info) = self.discovery.root_info {
                root_info.type_name == *type_name && root_info.is_composition
            } else {
                false
            };

            if !should_skip && !visited.contains(type_name) {
                self.topological_sort_visit(
                    type_name,
                    &mut visited,
                    &mut temp_visited,
                    &mut ordered_types,
                )?;
            }
        }

        // Reverse to get correct dependency order (dependencies before dependents)
        ordered_types.reverse();

        self.analysis.generation_order = ordered_types;
        self.analysis.dependency_graph.ordered_types = self.analysis.generation_order.clone();

        Ok(())
    }

    /// Recursive helper for topological sort
    fn topological_sort_visit(
        &self,
        type_name: &str,
        visited: &mut HashSet<String>,
        temp_visited: &mut HashSet<String>,
        ordered_types: &mut Vec<String>,
    ) -> TctResult<()> {
        if temp_visited.contains(type_name) {
            // Cycle detected - but we handle this in cycle detection phase
            return Ok(());
        }

        if visited.contains(type_name) {
            return Ok(());
        }

        temp_visited.insert(type_name.to_string());

        // Visit all dependencies first, in name order (the sets are randomly ordered)
        if let Some(dependencies) = self.analysis.dependency_graph.dependencies.get(type_name) {
            let mut dependencies: Vec<&String> = dependencies.iter().collect();
            dependencies.sort();
            for dep_type in dependencies {
                self.topological_sort_visit(dep_type, visited, temp_visited, ordered_types)?;
            }
        }

        temp_visited.remove(type_name);
        visited.insert(type_name.to_string());
        ordered_types.push(type_name.to_string());

        Ok(())
    }

    /// Finalize analysis statistics
    fn finalize_statistics(&mut self) {
        self.analysis.stats.total_types = self.discovery.discovered_types.len();

        // Calculate max dependency depth
        self.analysis.stats.max_dependency_depth = self.calculate_max_dependency_depth();
    }

    /// Calculate maximum dependency depth in the graph
    fn calculate_max_dependency_depth(&self) -> usize {
        let mut max_depth = 0;

        for type_name in self.discovery.discovered_types.keys() {
            let depth = self.calculate_dependency_depth(type_name, &mut HashSet::new());
            max_depth = max_depth.max(depth);
        }

        max_depth
    }

    /// Calculate dependency depth for a single type
    fn calculate_dependency_depth(&self, type_name: &str, visited: &mut HashSet<String>) -> usize {
        if visited.contains(type_name) {
            return 0; // Avoid infinite recursion in cycles
        }

        visited.insert(type_name.to_string());

        let max_dep_depth = if let Some(dependencies) =
            self.analysis.dependency_graph.dependencies.get(type_name)
        {
            dependencies
                .iter()
                .map(|dep| self.calculate_dependency_depth(dep, visited))
                .max()
                .unwrap_or(0)
        } else {
            0
        };

        visited.remove(type_name);
        max_dep_depth + 1
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::decoder::TokenStructure;
    use crate::generator::schema_discovery::{
        MemberType, SchemaDiscovery, SchemaLocation, TypeDiscovery, TypeKind,
    };
    use crate::optimization_pass::OptimizationConfig;
    use serde_json::Map;
    use std::collections::HashSet;

    // Helper function to create a simple type discovery for testing
    fn create_test_type(type_name: &str, type_kind: TypeKind) -> TypeDiscovery {
        TypeDiscovery::new(
            type_name.to_string(),
            type_kind,
            SchemaLocation {
                json_path: format!("#/{}", type_name),
                description: format!("Test type {}", type_name),
                parent_type: None,
                property_name: None,
            },
            Map::new(),
        )
    }

    #[test]
    fn test_analyze_primitive_derivability_integers() {
        let discovery = SchemaDiscovery::new();
        let config = OptimizationConfig::o0();
        let analyzer = TypeAnalyzer::new(discovery, config);

        // Test integer types - should derive all traits
        for int_type in &["i32", "i64", "u32", "u64", "usize"] {
            let derivability = analyzer.analyze_primitive_derivability(int_type);
            assert!(derivability.can_derive_eq, "{} should derive Eq", int_type);
            assert!(
                derivability.can_derive_ord,
                "{} should derive Ord",
                int_type
            );
            assert!(
                derivability.can_derive_hash,
                "{} should derive Hash",
                int_type
            );
        }
    }

    #[test]
    fn test_analyze_primitive_derivability_floats() {
        let discovery = SchemaDiscovery::new();
        let config = OptimizationConfig::o0();
        let analyzer = TypeAnalyzer::new(discovery, config);

        // Test float types - should NOT derive Eq/Ord due to NaN
        for float_type in &["f32", "f64"] {
            let derivability = analyzer.analyze_primitive_derivability(float_type);
            assert!(
                !derivability.can_derive_eq,
                "{} should NOT derive Eq",
                float_type
            );
            assert!(
                !derivability.can_derive_ord,
                "{} should NOT derive Ord",
                float_type
            );
            assert!(
                !derivability.can_derive_hash,
                "{} should NOT derive Hash",
                float_type
            );
        }
    }

    #[test]
    fn test_analyze_primitive_derivability_string() {
        let discovery = SchemaDiscovery::new();
        let config = OptimizationConfig::o0();
        let analyzer = TypeAnalyzer::new(discovery, config);

        let derivability = analyzer.analyze_primitive_derivability("String");
        assert!(derivability.can_derive_eq);
        assert!(derivability.can_derive_ord);
        assert!(derivability.can_derive_hash);
    }

    #[test]
    fn test_analyze_primitive_derivability_bool() {
        let discovery = SchemaDiscovery::new();
        let config = OptimizationConfig::o0();
        let analyzer = TypeAnalyzer::new(discovery, config);

        let derivability = analyzer.analyze_primitive_derivability("bool");
        assert!(derivability.can_derive_eq);
        assert!(derivability.can_derive_ord);
        assert!(derivability.can_derive_hash);
    }

    #[test]
    fn test_analyze_primitive_derivability_unit() {
        let discovery = SchemaDiscovery::new();
        let config = OptimizationConfig::o0();
        let analyzer = TypeAnalyzer::new(discovery, config);

        let derivability = analyzer.analyze_primitive_derivability("()");
        assert!(derivability.can_derive_eq);
        assert!(derivability.can_derive_ord);
        assert!(derivability.can_derive_hash);
    }

    #[test]
    fn test_analyze_primitive_derivability_json_value() {
        let discovery = SchemaDiscovery::new();
        let config = OptimizationConfig::o0();
        let analyzer = TypeAnalyzer::new(discovery, config);

        let derivability = analyzer.analyze_primitive_derivability("serde_json::Value");
        assert!(!derivability.can_derive_eq);
        assert!(!derivability.can_derive_ord);
        assert!(!derivability.can_derive_hash);
    }

    #[test]
    fn test_has_self_loop() {
        let discovery = SchemaDiscovery::new();
        let config = OptimizationConfig::o0();
        let mut analyzer = TypeAnalyzer::new(discovery.clone(), config);

        // Add a self-loop for type "A"
        let mut deps = HashSet::new();
        deps.insert("A".to_string());
        analyzer
            .analysis
            .dependency_graph
            .dependencies
            .insert("A".to_string(), deps);

        assert!(analyzer.has_self_loop("A"));
        assert!(!analyzer.has_self_loop("B")); // Doesn't exist
    }

    #[test]
    fn test_extract_dependency_from_member_type_reference() {
        let discovery = SchemaDiscovery::new();
        let config = OptimizationConfig::o0();
        let analyzer = TypeAnalyzer::new(discovery, config);

        let member_type = MemberType::TypeReference("User".to_string());
        let result = analyzer.extract_dependency_from_member(&member_type);
        assert_eq!(result, Some("User".to_string()));
    }

    #[test]
    fn test_extract_dependency_from_member_complex_container() {
        let discovery = SchemaDiscovery::new();
        let config = OptimizationConfig::o0();
        let analyzer = TypeAnalyzer::new(discovery, config);

        let member_type = MemberType::ComplexContainer {
            container: "Vec".to_string(),
            inner: "Address".to_string(),
        };
        let result = analyzer.extract_dependency_from_member(&member_type);
        assert_eq!(result, Some("Address".to_string()));
    }

    #[test]
    fn test_extract_dependency_from_member_primitive() {
        let discovery = SchemaDiscovery::new();
        let config = OptimizationConfig::o0();
        let analyzer = TypeAnalyzer::new(discovery, config);

        let member_type = MemberType::Primitive("String".to_string());
        let result = analyzer.extract_dependency_from_member(&member_type);
        assert_eq!(result, None); // Primitives have no dependencies
    }

    #[test]
    fn test_build_dependency_graph_simple() {
        let mut discovery = SchemaDiscovery::new();
        let config = OptimizationConfig::o0();

        // Create type A that references type B
        let mut type_a = create_test_type("A", TypeKind::Struct);
        type_a.add_reference("B".to_string());
        discovery.register_type(type_a);

        // Create type B with no dependencies
        let type_b = create_test_type("B", TypeKind::Struct);
        discovery.register_type(type_b);

        let mut analyzer = TypeAnalyzer::new(discovery, config);
        analyzer.build_dependency_graph().unwrap();

        // Check that A depends on B
        let a_deps = analyzer
            .analysis
            .dependency_graph
            .dependencies
            .get("A")
            .unwrap();
        assert!(a_deps.contains("B"));

        // Check statistics
        assert_eq!(analyzer.analysis.stats.total_dependencies, 1);
    }

    #[test]
    fn test_detect_cycles_no_cycle() {
        let mut discovery = SchemaDiscovery::new();
        let config = OptimizationConfig::o0();

        // Create a linear dependency: A -> B -> C
        let mut type_a = create_test_type("A", TypeKind::Struct);
        type_a.add_reference("B".to_string());
        discovery.register_type(type_a);

        let mut type_b = create_test_type("B", TypeKind::Struct);
        type_b.add_reference("C".to_string());
        discovery.register_type(type_b);

        let type_c = create_test_type("C", TypeKind::Struct);
        discovery.register_type(type_c);

        let mut analyzer = TypeAnalyzer::new(discovery, config);
        analyzer.build_dependency_graph().unwrap();
        analyzer.detect_cycles().unwrap();

        // No cycles should be detected
        assert_eq!(analyzer.analysis.stats.cycles_detected, 0);
        assert_eq!(analyzer.analysis.stats.types_requiring_boxing, 0);
    }

    #[test]
    fn test_detect_cycles_simple_cycle() {
        let mut discovery = SchemaDiscovery::new();
        let config = OptimizationConfig::o0();

        // Create a cycle: A -> B -> A
        let mut type_a = create_test_type("A", TypeKind::Struct);
        type_a.add_reference("B".to_string());
        discovery.register_type(type_a);

        let mut type_b = create_test_type("B", TypeKind::Struct);
        type_b.add_reference("A".to_string());
        discovery.register_type(type_b);

        let mut analyzer = TypeAnalyzer::new(discovery, config);
        analyzer.build_dependency_graph().unwrap();
        analyzer.detect_cycles().unwrap();

        // Should detect 1 cycle with 2 types needing boxing
        assert_eq!(analyzer.analysis.stats.cycles_detected, 1);
        assert_eq!(analyzer.analysis.stats.types_requiring_boxing, 2);
        assert!(analyzer.analysis.boxed_types.contains("A"));
        assert!(analyzer.analysis.boxed_types.contains("B"));
    }

    #[test]
    fn test_detect_cycles_self_loop() {
        let mut discovery = SchemaDiscovery::new();
        let config = OptimizationConfig::o0();

        // Create a self-loop: A -> A
        let mut type_a = create_test_type("A", TypeKind::Struct);
        type_a.add_reference("A".to_string());
        discovery.register_type(type_a);

        let mut analyzer = TypeAnalyzer::new(discovery, config);
        analyzer.build_dependency_graph().unwrap();
        analyzer.detect_cycles().unwrap();

        // Should detect 1 cycle
        assert_eq!(analyzer.analysis.stats.cycles_detected, 1);
        assert_eq!(analyzer.analysis.stats.types_requiring_boxing, 1);
        assert!(analyzer.analysis.boxed_types.contains("A"));
    }

    #[test]
    fn test_map_primitive_to_token_structure() {
        let discovery = SchemaDiscovery::new();
        let config = OptimizationConfig::o0();
        let analyzer = TypeAnalyzer::new(discovery, config);

        // String types
        assert!(matches!(
            analyzer.map_primitive_to_token_structure("String"),
            TokenStructure::String { length: 0 }
        ));

        // Integer types
        assert!(matches!(
            analyzer.map_primitive_to_token_structure("i32"),
            TokenStructure::Primitive(0)
        ));
        assert!(matches!(
            analyzer.map_primitive_to_token_structure("u64"),
            TokenStructure::Primitive(0)
        ));

        // Boolean
        assert!(matches!(
            analyzer.map_primitive_to_token_structure("bool"),
            TokenStructure::Primitive(0)
        ));

        // Unit type
        assert!(matches!(
            analyzer.map_primitive_to_token_structure("()"),
            TokenStructure::Null
        ));
    }

    // ========================================================================================
    // New tests to improve coverage from 72% to 85%+
    // ========================================================================================

    #[test]
    fn test_compute_decode_strategy_for_struct_with_optional_fields() {
        let mut discovery = SchemaDiscovery::new();
        let config = OptimizationConfig::o0();

        // Create struct with mix of required and optional fields
        let mut type_user = create_test_type("User", TypeKind::Struct);

        use crate::generator::schema_discovery::MemberInfo;
        use serde_json::json;

        // Add required field
        type_user.add_member(MemberInfo {
            name: "id".to_string(),
            member_type: MemberType::Primitive("i64".to_string()),
            required: true,
            is_nullable: false,
            schema: json!({"type": "integer"}).as_object().unwrap().clone(),
        });

        // Add optional field
        type_user.add_member(MemberInfo {
            name: "email".to_string(),
            member_type: MemberType::Primitive("String".to_string()),
            required: false,
            is_nullable: false,
            schema: json!({"type": "string"}).as_object().unwrap().clone(),
        });

        discovery.register_type(type_user);

        let analyzer = TypeAnalyzer::new(discovery, config);

        // Test decode strategy computation
        let type_def = analyzer.discovery.discovered_types.get("User").unwrap();
        let strategy = analyzer.compute_decode_strategy("User", type_def).unwrap();

        // Should have field order computed
        assert_eq!(strategy.field_order.len(), 2);
        assert_eq!(strategy.field_order[0], "id");
        assert_eq!(strategy.field_order[1], "email");
    }

    #[test]
    fn test_analyze_member_derivability_hashmap() {
        let discovery = SchemaDiscovery::new();
        let config = OptimizationConfig::o0();
        let mut analyzer = TypeAnalyzer::new(discovery, config);

        let mut analyzing_stack = HashSet::new();

        // Test HashMap member derivability
        let hashmap_member = MemberType::HashMap {
            key: "String".to_string(),
            value: "String".to_string(),
        };

        let result = analyzer
            .analyze_member_derivability_recursive(&hashmap_member, &mut analyzing_stack)
            .unwrap();

        // HashMap cannot derive Eq, Ord, or Hash
        assert!(!result.can_derive_eq, "HashMap should not derive Eq");
        assert!(!result.can_derive_ord, "HashMap should not derive Ord");
        assert!(!result.can_derive_hash, "HashMap should not derive Hash");
        assert!(result.reasoning.contains("FxHashMap"));
    }

    #[test]
    fn test_analyze_member_derivability_pattern_properties() {
        let discovery = SchemaDiscovery::new();
        let config = OptimizationConfig::o0();
        let mut analyzer = TypeAnalyzer::new(discovery, config);

        let mut analyzing_stack = HashSet::new();

        // Test PatternProperties member derivability
        let pattern_member = MemberType::PatternProperties {
            validators: vec!["^[a-z]+$".to_string()],
        };

        let result = analyzer
            .analyze_member_derivability_recursive(&pattern_member, &mut analyzing_stack)
            .unwrap();

        // PatternProperties cannot derive Eq, Ord, or Hash
        assert!(
            !result.can_derive_eq,
            "PatternProperties should not derive Eq"
        );
        assert!(
            !result.can_derive_ord,
            "PatternProperties should not derive Ord"
        );
        assert!(
            !result.can_derive_hash,
            "PatternProperties should not derive Hash"
        );
        assert!(result.reasoning.contains("PatternProperties"));
    }

    #[test]
    fn test_determine_member_token_structure_hashmap_and_pattern() {
        let discovery = SchemaDiscovery::new();
        let config = OptimizationConfig::o0();
        let analyzer = TypeAnalyzer::new(discovery, config);

        // Test HashMap token structure
        let hashmap_member = MemberType::HashMap {
            key: "String".to_string(),
            value: "String".to_string(),
        };

        let result = analyzer
            .determine_member_token_structure(&hashmap_member)
            .unwrap();
        assert!(matches!(result, TokenStructure::Object { field_count: 0 }));

        // Test PatternProperties token structure
        let pattern_member = MemberType::PatternProperties {
            validators: vec!["^[a-z]+$".to_string()],
        };

        let result = analyzer
            .determine_member_token_structure(&pattern_member)
            .unwrap();
        assert!(matches!(result, TokenStructure::Object { field_count: 0 }));
    }

    #[test]
    fn test_compute_decode_strategy_for_all_type_kinds() {
        let mut discovery = SchemaDiscovery::new();
        let config = OptimizationConfig::o0();

        // Create enum type
        let type_enum = create_test_type("Status", TypeKind::Enum);
        discovery.register_type(type_enum.clone());

        // Create reference type
        let type_ref = create_test_type(
            "UserRef",
            TypeKind::Reference {
                target: "User".to_string(),
            },
        );
        discovery.register_type(type_ref.clone());

        // Create array type
        let type_array = create_test_type("ItemList", TypeKind::ArrayOfComplex);
        discovery.register_type(type_array.clone());

        // Create primitive wrapper
        let type_primitive = create_test_type(
            "Count",
            TypeKind::PrimitiveWrapper {
                primitive_type: "i64".to_string(),
            },
        );
        discovery.register_type(type_primitive.clone());

        // Create minimal object wrapper
        let type_minimal = create_test_type("Config", TypeKind::MinimalObjectWrapper);
        discovery.register_type(type_minimal.clone());

        let analyzer = TypeAnalyzer::new(discovery, config);

        // Test all decode strategies
        let strategy_enum = analyzer
            .compute_decode_strategy("Status", &type_enum)
            .unwrap();
        assert!(strategy_enum.field_order.is_empty());

        let strategy_ref = analyzer
            .compute_decode_strategy("UserRef", &type_ref)
            .unwrap();
        assert!(strategy_ref.field_order.is_empty());

        let strategy_array = analyzer
            .compute_decode_strategy("ItemList", &type_array)
            .unwrap();
        assert!(strategy_array.field_order.is_empty());

        let strategy_primitive = analyzer
            .compute_decode_strategy("Count", &type_primitive)
            .unwrap();
        assert!(strategy_primitive.field_order.is_empty());

        let strategy_minimal = analyzer
            .compute_decode_strategy("Config", &type_minimal)
            .unwrap();
        assert!(strategy_minimal.field_order.is_empty());
    }

    #[test]
    fn test_analyze_member_derivability_containers() {
        let discovery = SchemaDiscovery::new();
        let config = OptimizationConfig::o0();
        let mut analyzer = TypeAnalyzer::new(discovery, config);

        let mut analyzing_stack = HashSet::new();

        // Test Vec<String> derivability
        let vec_member = MemberType::Container {
            container: "Vec".to_string(),
            inner: "String".to_string(),
        };
        let result = analyzer
            .analyze_member_derivability_recursive(&vec_member, &mut analyzing_stack)
            .unwrap();
        assert!(result.can_derive_eq);
        assert!(result.can_derive_ord);
        assert!(result.can_derive_hash);

        // Test Option<i64> derivability
        let option_member = MemberType::Container {
            container: "Option".to_string(),
            inner: "i64".to_string(),
        };
        let result = analyzer
            .analyze_member_derivability_recursive(&option_member, &mut analyzing_stack)
            .unwrap();
        assert!(result.can_derive_eq);
        assert!(result.can_derive_ord);
        assert!(result.can_derive_hash);

        // Test Vec<f64> derivability (should fail for Eq/Ord due to f64)
        let vec_float_member = MemberType::Container {
            container: "Vec".to_string(),
            inner: "f64".to_string(),
        };
        let result = analyzer
            .analyze_member_derivability_recursive(&vec_float_member, &mut analyzing_stack)
            .unwrap();
        assert!(!result.can_derive_eq, "Vec<f64> should not derive Eq");
        assert!(!result.can_derive_ord, "Vec<f64> should not derive Ord");
        assert!(!result.can_derive_hash, "Vec<f64> should not derive Hash");
    }

    #[test]
    fn test_determine_member_token_structure_containers() {
        let discovery = SchemaDiscovery::new();
        let config = OptimizationConfig::o0();
        let analyzer = TypeAnalyzer::new(discovery, config);

        // Test Vec container
        let vec_member = MemberType::Container {
            container: "Vec".to_string(),
            inner: "String".to_string(),
        };
        let result = analyzer
            .determine_member_token_structure(&vec_member)
            .unwrap();
        assert!(matches!(result, TokenStructure::Array { length: 0 }));

        // Test HashMap container
        let hashmap_member = MemberType::Container {
            container: "HashMap".to_string(),
            inner: "String".to_string(),
        };
        let result = analyzer
            .determine_member_token_structure(&hashmap_member)
            .unwrap();
        assert!(matches!(result, TokenStructure::Object { field_count: 0 }));

        // Test BTreeMap container
        let btreemap_member = MemberType::Container {
            container: "BTreeMap".to_string(),
            inner: "String".to_string(),
        };
        let result = analyzer
            .determine_member_token_structure(&btreemap_member)
            .unwrap();
        assert!(matches!(result, TokenStructure::Object { field_count: 0 }));
    }

    #[test]
    fn test_should_enum_optimize_to_primitive() {
        let mut discovery = SchemaDiscovery::new();

        use crate::generator::schema_discovery::MemberInfo;
        use serde_json::json;

        // Create a simple enum (all unit variants)
        let mut simple_enum = create_test_type("Status", TypeKind::Enum);
        simple_enum.add_member(MemberInfo {
            name: "Active".to_string(),
            member_type: MemberType::Primitive("()".to_string()),
            required: true,
            is_nullable: false,
            schema: json!({"type": "string", "const": "Active"})
                .as_object()
                .unwrap()
                .clone(),
        });
        simple_enum.add_member(MemberInfo {
            name: "Inactive".to_string(),
            member_type: MemberType::Primitive("()".to_string()),
            required: true,
            is_nullable: false,
            schema: json!({"type": "string", "const": "Inactive"})
                .as_object()
                .unwrap()
                .clone(),
        });

        discovery.register_type(simple_enum.clone());

        // Test with optimization disabled (O0)
        let config_no_opt = OptimizationConfig::o0();
        let analyzer_no_opt = TypeAnalyzer::new(discovery.clone(), config_no_opt);
        let should_optimize = analyzer_no_opt.should_enum_optimize_to_primitive(&simple_enum);
        assert!(!should_optimize, "Should not optimize at O0 level");

        // Test with optimization enabled
        let config_with_opt = OptimizationConfig::o2(); // O2 enables enum optimization
        let analyzer_with_opt = TypeAnalyzer::new(discovery, config_with_opt);
        let should_optimize = analyzer_with_opt.should_enum_optimize_to_primitive(&simple_enum);
        assert!(should_optimize, "Should optimize simple enum at O2 level");
    }

    #[test]
    fn test_compute_decode_strategy_for_empty_struct() {
        let mut discovery = SchemaDiscovery::new();
        let config = OptimizationConfig::o0();

        // Create empty struct (no members)
        let empty_struct = create_test_type("EmptyStruct", TypeKind::Struct);
        discovery.register_type(empty_struct.clone());

        let analyzer = TypeAnalyzer::new(discovery, config);

        // Test decode strategy for empty struct
        let strategy = analyzer
            .compute_decode_strategy("EmptyStruct", &empty_struct)
            .unwrap();

        // Empty struct should have empty field_order
        assert_eq!(strategy.field_order.len(), 0);
    }

    #[test]
    fn test_analyze_type_recursive_with_cycle_detection() {
        let mut discovery = SchemaDiscovery::new();
        let config = OptimizationConfig::o0();

        use crate::generator::schema_discovery::MemberInfo;
        use serde_json::json;

        // Create type A with member of type B
        let mut type_a = create_test_type("A", TypeKind::Struct);
        type_a.add_member(MemberInfo {
            name: "b_field".to_string(),
            member_type: MemberType::TypeReference("B".to_string()),
            required: true,
            is_nullable: false,
            schema: json!({"$ref": "#/definitions/B"})
                .as_object()
                .unwrap()
                .clone(),
        });
        type_a.add_reference("B".to_string());
        discovery.register_type(type_a);

        // Create type B with member of type A (mutual recursion)
        let mut type_b = create_test_type("B", TypeKind::Struct);
        type_b.add_member(MemberInfo {
            name: "a_field".to_string(),
            member_type: MemberType::TypeReference("A".to_string()),
            required: true,
            is_nullable: false,
            schema: json!({"$ref": "#/definitions/A"})
                .as_object()
                .unwrap()
                .clone(),
        });
        type_b.add_reference("A".to_string());
        discovery.register_type(type_b);

        let mut analyzer = TypeAnalyzer::new(discovery, config);

        // Manually analyze trait derivability for both types
        let mut analyzing_stack = HashSet::new();
        analyzer
            .analyze_type_recursive("A", &mut analyzing_stack)
            .unwrap();
        analyzer
            .analyze_type_recursive("B", &mut analyzing_stack)
            .unwrap();

        // Both types should have trait derivability analysis completed
        // Cycle detection should have applied conservative defaults
        assert!(analyzer.analysis.trait_derivability.contains_key("A"));
        assert!(analyzer.analysis.trait_derivability.contains_key("B"));
    }

    #[test]
    fn test_compute_trait_derivability_array_with_ref_to_struct() {
        let mut discovery = SchemaDiscovery::new();
        let config = OptimizationConfig::o0();

        use crate::generator::schema_discovery::MemberInfo;
        use serde_json::json;

        // Create a struct "User" with f64 field (cannot derive Eq/Ord)
        let mut user_struct = create_test_type("User", TypeKind::Struct);
        user_struct.add_member(MemberInfo {
            name: "score".to_string(),
            member_type: MemberType::Primitive("f64".to_string()),
            required: true,
            is_nullable: false,
            schema: json!({"type": "number"}).as_object().unwrap().clone(),
        });
        discovery.register_type(user_struct);

        // Create ArrayOfComplex type with items.$ref = "#/definitions/User"
        let user_list = TypeDiscovery::new(
            "UserList".to_string(),
            TypeKind::ArrayOfComplex,
            SchemaLocation {
                json_path: "#/UserList".to_string(),
                description: "Array of User".to_string(),
                parent_type: None,
                property_name: None,
            },
            json!({
                "type": "array",
                "items": {
                    "$ref": "#/definitions/User"
                }
            })
            .as_object()
            .unwrap()
            .clone(),
        );
        discovery.register_type(user_list.clone());

        let mut analyzer = TypeAnalyzer::new(discovery, config);

        // Analyze trait derivability
        let mut analyzing_stack = HashSet::new();
        let derivability = analyzer
            .compute_trait_derivability(&user_list, &mut analyzing_stack)
            .unwrap();

        // Array should inherit non-derivability from element type (f64 cannot derive Eq/Ord)
        assert!(
            !derivability.can_derive_eq,
            "Array of struct with f64 should not derive Eq"
        );
        assert!(
            !derivability.can_derive_ord,
            "Array of struct with f64 should not derive Ord"
        );
    }

    #[test]
    fn test_compute_trait_derivability_array_with_escaped_ref_to_struct() {
        let mut discovery = SchemaDiscovery::new();
        let config = OptimizationConfig::o0();

        use crate::generator::schema_discovery::MemberInfo;
        use serde_json::json;

        let mut user_record_struct = create_test_type("UserRecord", TypeKind::Struct);
        user_record_struct.add_member(MemberInfo {
            name: "score".to_string(),
            member_type: MemberType::Primitive("f64".to_string()),
            required: true,
            is_nullable: false,
            schema: json!({"type": "number"}).as_object().unwrap().clone(),
        });
        discovery.register_type(user_record_struct);

        let user_record_list = TypeDiscovery::new(
            "UserRecordList".to_string(),
            TypeKind::ArrayOfComplex,
            SchemaLocation {
                json_path: "#/UserRecordList".to_string(),
                description: "Array of UserRecord".to_string(),
                parent_type: None,
                property_name: None,
            },
            json!({
                "type": "array",
                "items": {
                    "$ref": "#/definitions/User~1Record"
                }
            })
            .as_object()
            .unwrap()
            .clone(),
        );
        discovery.register_type(user_record_list.clone());

        let mut analyzer = TypeAnalyzer::new(discovery, config);
        let mut analyzing_stack = HashSet::new();
        let derivability = analyzer
            .compute_trait_derivability(&user_record_list, &mut analyzing_stack)
            .unwrap();

        assert!(
            !derivability.can_derive_eq,
            "escaped array item refs should resolve to the decoded element type"
        );
        assert!(!derivability.can_derive_ord);
    }

    #[test]
    fn test_topological_sort_diamond_dependency() {
        let mut discovery = SchemaDiscovery::new();
        let config = OptimizationConfig::o0();

        // Create diamond dependency: A depends on B and C, both B and C depend on D
        // Topology: A -> B -> D
        //           A -> C -> D
        let mut type_a = create_test_type("A", TypeKind::Struct);
        type_a.add_reference("B".to_string());
        type_a.add_reference("C".to_string());
        discovery.register_type(type_a);

        let mut type_b = create_test_type("B", TypeKind::Struct);
        type_b.add_reference("D".to_string());
        discovery.register_type(type_b);

        let mut type_c = create_test_type("C", TypeKind::Struct);
        type_c.add_reference("D".to_string());
        discovery.register_type(type_c);

        let type_d = create_test_type("D", TypeKind::Struct);
        discovery.register_type(type_d);

        let mut analyzer = TypeAnalyzer::new(discovery, config);
        analyzer.build_dependency_graph().unwrap();
        analyzer.compute_generation_order().unwrap();

        let generation_order = &analyzer.analysis.generation_order;

        // Find positions of each type in generation order
        let d_pos = generation_order.iter().position(|t| t == "D").unwrap();
        let b_pos = generation_order.iter().position(|t| t == "B").unwrap();
        let c_pos = generation_order.iter().position(|t| t == "C").unwrap();
        let a_pos = generation_order.iter().position(|t| t == "A").unwrap();

        // Current implementation reverses DFS post-order, putting dependents before dependencies
        // For diamond: A → B,C → D, the order after reverse is [A, C, B, D]
        // Verify the dependency structure is maintained (A appears before B and C; B and C appear before D)
        assert!(a_pos < b_pos, "A should appear before B (A depends on B)");
        assert!(a_pos < c_pos, "A should appear before C (A depends on C)");
        assert!(b_pos < d_pos, "B should appear before D (B depends on D)");
        assert!(c_pos < d_pos, "C should appear before D (C depends on D)");
    }

    #[test]
    fn test_generation_order_does_not_depend_on_hash_iteration_order() {
        // Dependencies are kept in hash sets with a random iteration order per
        // instance, and the generated file lists types in this order, so the same
        // schema used to produce differently ordered source on every run.
        fn generation_order() -> Vec<String> {
            let mut discovery = SchemaDiscovery::new();
            let mut root = create_test_type("Root", TypeKind::Struct);
            let mut other = create_test_type("Other", TypeKind::Struct);
            for index in 0..12 {
                let leaf_name = format!("Leaf{index:02}");
                root.add_reference(leaf_name.clone());
                if index % 2 == 0 {
                    other.add_reference(leaf_name.clone());
                }
                discovery.register_type(create_test_type(&leaf_name, TypeKind::Struct));
            }
            discovery.register_type(root);
            discovery.register_type(other);

            let mut analyzer = TypeAnalyzer::new(discovery, OptimizationConfig::o0());
            analyzer.build_dependency_graph().unwrap();
            analyzer.compute_generation_order().unwrap();
            analyzer.analysis.generation_order.clone()
        }

        let first = generation_order();
        for _ in 0..25 {
            assert_eq!(generation_order(), first);
        }

        // The order is still a valid dependency order: a type precedes what it uses.
        let position = |name: &str| first.iter().position(|t| t == name).unwrap();
        for index in 0..12 {
            assert!(position("Root") < position(&format!("Leaf{index:02}")));
        }
    }

    // ========================================================================================
    // Additional tests to improve regions coverage from 72.57% to 85%+
    // ========================================================================================

    #[test]
    fn test_determine_member_token_structure_option_of_struct() {
        let mut discovery = SchemaDiscovery::new();
        let config = OptimizationConfig::o0();

        use crate::generator::schema_discovery::MemberInfo;
        use serde_json::json;

        // Create a struct type
        let mut user_struct = create_test_type("User", TypeKind::Struct);
        user_struct.add_member(MemberInfo {
            name: "id".to_string(),
            member_type: MemberType::Primitive("i64".to_string()),
            required: true,
            is_nullable: false,
            schema: json!({"type": "integer"}).as_object().unwrap().clone(),
        });
        user_struct.add_member(MemberInfo {
            name: "name".to_string(),
            member_type: MemberType::Primitive("String".to_string()),
            required: true,
            is_nullable: false,
            schema: json!({"type": "string"}).as_object().unwrap().clone(),
        });
        discovery.register_type(user_struct);

        let analyzer = TypeAnalyzer::new(discovery, config);

        // Test Option<User> token structure
        let option_member = MemberType::Container {
            container: "Option".to_string(),
            inner: "User".to_string(),
        };

        let result = analyzer
            .determine_member_token_structure(&option_member)
            .unwrap();

        // Option<Struct> should map to Object with 2 required fields
        assert!(matches!(result, TokenStructure::Object { field_count: 2 }));
    }

    #[test]
    fn test_determine_member_token_structure_box_of_enum() {
        let mut discovery = SchemaDiscovery::new();
        let config = OptimizationConfig::o2(); // Enable enum optimization

        use crate::generator::schema_discovery::MemberInfo;
        use serde_json::json;

        // Create a simple enum (unit variants)
        let mut status_enum = create_test_type("Status", TypeKind::Enum);
        status_enum.add_member(MemberInfo {
            name: "Active".to_string(),
            member_type: MemberType::Primitive("()".to_string()),
            required: true,
            is_nullable: false,
            schema: json!({"const": "Active"}).as_object().unwrap().clone(),
        });
        status_enum.add_member(MemberInfo {
            name: "Inactive".to_string(),
            member_type: MemberType::Primitive("()".to_string()),
            required: true,
            is_nullable: false,
            schema: json!({"const": "Inactive"}).as_object().unwrap().clone(),
        });
        discovery.register_type(status_enum);

        let analyzer = TypeAnalyzer::new(discovery, config);

        // Test Box<Status> token structure
        let box_member = MemberType::Container {
            container: "Box".to_string(),
            inner: "Status".to_string(),
        };

        let result = analyzer
            .determine_member_token_structure(&box_member)
            .unwrap();

        // Box<Enum> with optimization should map to Primitive
        assert!(matches!(result, TokenStructure::Primitive(0)));
    }

    #[test]
    fn test_determine_member_token_structure_option_of_empty_struct() {
        let mut discovery = SchemaDiscovery::new();
        let config = OptimizationConfig::o0();

        // Create empty struct
        let empty_struct = create_test_type("EmptyStruct", TypeKind::Struct);
        discovery.register_type(empty_struct);

        let analyzer = TypeAnalyzer::new(discovery, config);

        // Test Option<EmptyStruct> token structure
        let option_member = MemberType::Container {
            container: "Option".to_string(),
            inner: "EmptyStruct".to_string(),
        };

        let result = analyzer
            .determine_member_token_structure(&option_member)
            .unwrap();

        // Option<EmptyStruct> should map to Null (empty structs have zero tokens)
        assert!(matches!(result, TokenStructure::Null));
    }

    #[test]
    fn test_determine_member_token_structure_option_of_reference() {
        let mut discovery = SchemaDiscovery::new();
        let config = OptimizationConfig::o0();

        // Create reference type
        let ref_type = create_test_type(
            "UserRef",
            TypeKind::Reference {
                target: "User".to_string(),
            },
        );
        discovery.register_type(ref_type);

        let analyzer = TypeAnalyzer::new(discovery, config);

        // Test Option<UserRef> token structure
        let option_member = MemberType::Container {
            container: "Option".to_string(),
            inner: "UserRef".to_string(),
        };

        let result = analyzer
            .determine_member_token_structure(&option_member)
            .unwrap();

        // Option<Reference> should map to Primitive
        assert!(matches!(result, TokenStructure::Primitive(0)));
    }

    #[test]
    fn test_determine_member_token_structure_option_of_array() {
        let mut discovery = SchemaDiscovery::new();
        let config = OptimizationConfig::o0();

        // Create array type
        let array_type = create_test_type("ItemList", TypeKind::ArrayOfComplex);
        discovery.register_type(array_type);

        let analyzer = TypeAnalyzer::new(discovery, config);

        // Test Option<ItemList> token structure
        let option_member = MemberType::Container {
            container: "Option".to_string(),
            inner: "ItemList".to_string(),
        };

        let result = analyzer
            .determine_member_token_structure(&option_member)
            .unwrap();

        // Option<Array> should map to Array
        assert!(matches!(result, TokenStructure::Array { length: 0 }));
    }

    #[test]
    fn test_determine_member_token_structure_option_of_primitive_wrapper() {
        let mut discovery = SchemaDiscovery::new();
        let config = OptimizationConfig::o0();

        // Create primitive wrapper type
        let wrapper_type = create_test_type(
            "Count",
            TypeKind::PrimitiveWrapper {
                primitive_type: "i64".to_string(),
            },
        );
        discovery.register_type(wrapper_type);

        let analyzer = TypeAnalyzer::new(discovery, config);

        // Test Option<Count> token structure
        let option_member = MemberType::Container {
            container: "Option".to_string(),
            inner: "Count".to_string(),
        };

        let result = analyzer
            .determine_member_token_structure(&option_member)
            .unwrap();

        // Option<PrimitiveWrapper> should map to Primitive (i64)
        assert!(matches!(result, TokenStructure::Primitive(0)));
    }

    #[test]
    fn test_determine_member_token_structure_option_of_minimal_object() {
        let mut discovery = SchemaDiscovery::new();
        let config = OptimizationConfig::o0();

        // Create minimal object wrapper type
        let minimal_type = create_test_type("Config", TypeKind::MinimalObjectWrapper);
        discovery.register_type(minimal_type);

        let analyzer = TypeAnalyzer::new(discovery, config);

        // Test Option<Config> token structure
        let option_member = MemberType::Container {
            container: "Option".to_string(),
            inner: "Config".to_string(),
        };

        let result = analyzer
            .determine_member_token_structure(&option_member)
            .unwrap();

        // Option<MinimalObjectWrapper> should map to Object
        assert!(matches!(result, TokenStructure::Object { field_count: 0 }));
    }

    #[test]
    fn test_determine_member_token_structure_option_fallback_to_primitive() {
        let discovery = SchemaDiscovery::new();
        let config = OptimizationConfig::o0();

        let analyzer = TypeAnalyzer::new(discovery, config);

        // Test Option<unknown type> fallback to primitive mapping
        let option_member = MemberType::Container {
            container: "Option".to_string(),
            inner: "UnknownType".to_string(), // Type not in discovery
        };

        let result = analyzer
            .determine_member_token_structure(&option_member)
            .unwrap();

        // Should fall back to primitive mapping for unknown types
        // "UnknownType" doesn't match any known primitive, so it should be Primitive(0)
        assert!(matches!(result, TokenStructure::Primitive(0)));
    }

    #[test]
    fn test_determine_member_token_structure_box_of_struct() {
        let mut discovery = SchemaDiscovery::new();
        let config = OptimizationConfig::o0();

        use crate::generator::schema_discovery::MemberInfo;
        use serde_json::json;

        // Create a struct with optional fields
        let mut product_struct = create_test_type("Product", TypeKind::Struct);
        product_struct.add_member(MemberInfo {
            name: "id".to_string(),
            member_type: MemberType::Primitive("i64".to_string()),
            required: true,
            is_nullable: false,
            schema: json!({"type": "integer"}).as_object().unwrap().clone(),
        });
        product_struct.add_member(MemberInfo {
            name: "description".to_string(),
            member_type: MemberType::Primitive("String".to_string()),
            required: false, // Optional field
            is_nullable: false,
            schema: json!({"type": "string"}).as_object().unwrap().clone(),
        });
        discovery.register_type(product_struct);

        let analyzer = TypeAnalyzer::new(discovery, config);

        // Test Box<Product> token structure (struct with optional fields)
        let box_member = MemberType::Container {
            container: "Box".to_string(),
            inner: "Product".to_string(),
        };

        let result = analyzer
            .determine_member_token_structure(&box_member)
            .unwrap();

        // Box<Struct> with 1 required field should map to Object with field_count 1
        assert!(matches!(result, TokenStructure::Object { field_count: 1 }));
    }

    #[test]
    fn test_determine_member_token_structure_box_fallback_to_primitive() {
        let discovery = SchemaDiscovery::new();
        let config = OptimizationConfig::o0();

        let analyzer = TypeAnalyzer::new(discovery, config);

        // Test Box<unknown type> fallback to primitive mapping
        let box_member = MemberType::Container {
            container: "Box".to_string(),
            inner: "UnknownType".to_string(), // Type not in discovery
        };

        let result = analyzer
            .determine_member_token_structure(&box_member)
            .unwrap();

        // Should fall back to primitive mapping for unknown types
        assert!(matches!(result, TokenStructure::Primitive(0)));
    }

    #[test]
    fn test_determine_member_token_structure_unknown_container() {
        let discovery = SchemaDiscovery::new();
        let config = OptimizationConfig::o0();

        let analyzer = TypeAnalyzer::new(discovery, config);

        // Test unknown container type
        let unknown_container = MemberType::Container {
            container: "UnknownContainer".to_string(),
            inner: "String".to_string(),
        };

        let result = analyzer
            .determine_member_token_structure(&unknown_container)
            .unwrap();

        // Unknown containers should fall back to Primitive(0)
        assert!(matches!(result, TokenStructure::Primitive(0)));
    }

    #[test]
    fn test_determine_member_token_structure_complex_container_option() {
        let mut discovery = SchemaDiscovery::new();
        let config = OptimizationConfig::o0();

        use crate::generator::schema_discovery::MemberInfo;
        use serde_json::json;

        // Create struct type
        let mut address_struct = create_test_type("Address", TypeKind::Struct);
        address_struct.add_member(MemberInfo {
            name: "street".to_string(),
            member_type: MemberType::Primitive("String".to_string()),
            required: true,
            is_nullable: false,
            schema: json!({"type": "string"}).as_object().unwrap().clone(),
        });
        discovery.register_type(address_struct);

        let analyzer = TypeAnalyzer::new(discovery, config);

        // Test ComplexContainer Option<Address>
        let complex_option = MemberType::ComplexContainer {
            container: "Option".to_string(),
            inner: "Address".to_string(),
        };

        let result = analyzer
            .determine_member_token_structure(&complex_option)
            .unwrap();

        // ComplexContainer Option<Struct> should map to Object with field_count
        assert!(matches!(result, TokenStructure::Object { field_count: 1 }));
    }

    #[test]
    fn test_determine_member_token_structure_complex_container_box() {
        let mut discovery = SchemaDiscovery::new();
        let config = OptimizationConfig::o0();

        // Create enum type (no optimization)
        let result_enum = create_test_type("Result", TypeKind::Enum);
        discovery.register_type(result_enum);

        let analyzer = TypeAnalyzer::new(discovery, config);

        // Test ComplexContainer Box<Result>
        let complex_box = MemberType::ComplexContainer {
            container: "Box".to_string(),
            inner: "Result".to_string(),
        };

        let result = analyzer
            .determine_member_token_structure(&complex_box)
            .unwrap();

        // ComplexContainer Box<Enum> without optimization should map to Array
        assert!(matches!(result, TokenStructure::Array { length: 0 }));
    }

    #[test]
    fn test_determine_member_token_structure_complex_container_vec() {
        let discovery = SchemaDiscovery::new();
        let config = OptimizationConfig::o0();

        let analyzer = TypeAnalyzer::new(discovery, config);

        // Test ComplexContainer Vec<T>
        let complex_vec = MemberType::ComplexContainer {
            container: "Vec".to_string(),
            inner: "SomeType".to_string(),
        };

        let result = analyzer
            .determine_member_token_structure(&complex_vec)
            .unwrap();

        // Vec should map to Array
        assert!(matches!(result, TokenStructure::Array { length: 0 }));
    }

    #[test]
    fn test_determine_member_token_structure_complex_container_hashmap() {
        let discovery = SchemaDiscovery::new();
        let config = OptimizationConfig::o0();

        let analyzer = TypeAnalyzer::new(discovery, config);

        // Test ComplexContainer HashMap<K, V>
        let complex_hashmap = MemberType::ComplexContainer {
            container: "HashMap".to_string(),
            inner: "SomeType".to_string(),
        };

        let result = analyzer
            .determine_member_token_structure(&complex_hashmap)
            .unwrap();

        // HashMap should map to Object
        assert!(matches!(result, TokenStructure::Object { field_count: 0 }));
    }

    #[test]
    fn test_determine_member_token_structure_complex_container_unknown() {
        let discovery = SchemaDiscovery::new();
        let config = OptimizationConfig::o0();

        let analyzer = TypeAnalyzer::new(discovery, config);

        // Test ComplexContainer with unknown container type
        let complex_unknown = MemberType::ComplexContainer {
            container: "UnknownComplexContainer".to_string(),
            inner: "SomeType".to_string(),
        };

        let result = analyzer
            .determine_member_token_structure(&complex_unknown)
            .unwrap();

        // Unknown complex containers should fall back to Primitive(0)
        assert!(matches!(result, TokenStructure::Primitive(0)));
    }

    #[test]
    fn test_determine_member_token_structure_type_reference_not_found() {
        let discovery = SchemaDiscovery::new();
        let config = OptimizationConfig::o0();

        let analyzer = TypeAnalyzer::new(discovery, config);

        // Test TypeReference to non-existent type
        let type_ref = MemberType::TypeReference("NonExistentType".to_string());

        let result = analyzer
            .determine_member_token_structure(&type_ref)
            .unwrap();

        // Non-existent type reference should fall back to Primitive(0)
        assert!(matches!(result, TokenStructure::Primitive(0)));
    }

    #[test]
    fn test_analyze_member_derivability_complex_container_vec() {
        let mut discovery = SchemaDiscovery::new();
        let config = OptimizationConfig::o0();

        use crate::generator::schema_discovery::MemberInfo;
        use serde_json::json;

        // Create struct with derivable traits
        let mut item_struct = create_test_type("Item", TypeKind::Struct);
        item_struct.add_member(MemberInfo {
            name: "id".to_string(),
            member_type: MemberType::Primitive("i64".to_string()),
            required: true,
            is_nullable: false,
            schema: json!({"type": "integer"}).as_object().unwrap().clone(),
        });
        discovery.register_type(item_struct);

        let mut analyzer = TypeAnalyzer::new(discovery, config);
        let mut analyzing_stack = HashSet::new();

        // Test Vec<Item> derivability (ComplexContainer)
        let vec_complex = MemberType::ComplexContainer {
            container: "Vec".to_string(),
            inner: "Item".to_string(),
        };

        let result = analyzer
            .analyze_member_derivability_recursive(&vec_complex, &mut analyzing_stack)
            .unwrap();

        // Vec<Struct with integer> should derive all traits
        assert!(result.can_derive_eq);
        assert!(result.can_derive_ord);
        assert!(result.can_derive_hash);
    }

    #[test]
    fn test_analyze_member_derivability_complex_container_unknown() {
        let discovery = SchemaDiscovery::new();
        let config = OptimizationConfig::o0();

        let mut analyzer = TypeAnalyzer::new(discovery, config);
        let mut analyzing_stack = HashSet::new();

        // Test unknown ComplexContainer derivability
        let unknown_complex = MemberType::ComplexContainer {
            container: "UnknownContainer".to_string(),
            inner: "SomeType".to_string(),
        };

        let result = analyzer
            .analyze_member_derivability_recursive(&unknown_complex, &mut analyzing_stack)
            .unwrap();

        // Unknown containers should use conservative derivability (heuristic)
        assert!(!result.can_derive_eq);
        assert!(!result.can_derive_ord);
        assert!(!result.can_derive_hash);
        assert!(result.reasoning.contains("unknown complex container"));
    }

    #[test]
    fn test_analyze_member_derivability_container_unknown() {
        let discovery = SchemaDiscovery::new();
        let config = OptimizationConfig::o0();

        let mut analyzer = TypeAnalyzer::new(discovery, config);
        let mut analyzing_stack = HashSet::new();

        // Test unknown Container derivability
        let unknown_container = MemberType::Container {
            container: "UnknownSimpleContainer".to_string(),
            inner: "String".to_string(),
        };

        let result = analyzer
            .analyze_member_derivability_recursive(&unknown_container, &mut analyzing_stack)
            .unwrap();

        // Unknown containers should use conservative derivability (heuristic)
        assert!(!result.can_derive_eq);
        assert!(!result.can_derive_ord);
        assert!(!result.can_derive_hash);
        assert!(result.reasoning.contains("unknown container type"));
    }

    #[test]
    fn test_compute_trait_derivability_array_with_primitive_string() {
        let mut discovery = SchemaDiscovery::new();
        let config = OptimizationConfig::o0();

        use serde_json::json;

        // Create ArrayOfComplex with primitive string items
        let array_type = TypeDiscovery::new(
            "StringList".to_string(),
            TypeKind::ArrayOfComplex,
            SchemaLocation {
                json_path: "#/StringList".to_string(),
                description: "Array of strings".to_string(),
                parent_type: None,
                property_name: None,
            },
            json!({
                "type": "array",
                "items": {
                    "type": "string"
                }
            })
            .as_object()
            .unwrap()
            .clone(),
        );
        discovery.register_type(array_type.clone());

        let mut analyzer = TypeAnalyzer::new(discovery, config);
        let mut analyzing_stack = HashSet::new();

        let derivability = analyzer
            .compute_trait_derivability(&array_type, &mut analyzing_stack)
            .unwrap();

        // Array of strings should derive all traits
        assert!(
            derivability.can_derive_eq,
            "Array of strings should derive Eq"
        );
        assert!(
            derivability.can_derive_ord,
            "Array of strings should derive Ord"
        );
        assert!(
            derivability.can_derive_hash,
            "Array of strings should derive Hash"
        );
    }

    #[test]
    fn test_compute_trait_derivability_array_with_primitive_number() {
        let mut discovery = SchemaDiscovery::new();
        let config = OptimizationConfig::o0();

        use serde_json::json;

        // Create ArrayOfComplex with primitive number (f64) items
        let array_type = TypeDiscovery::new(
            "NumberList".to_string(),
            TypeKind::ArrayOfComplex,
            SchemaLocation {
                json_path: "#/NumberList".to_string(),
                description: "Array of numbers".to_string(),
                parent_type: None,
                property_name: None,
            },
            json!({
                "type": "array",
                "items": {
                    "type": "number"
                }
            })
            .as_object()
            .unwrap()
            .clone(),
        );
        discovery.register_type(array_type.clone());

        let mut analyzer = TypeAnalyzer::new(discovery, config);
        let mut analyzing_stack = HashSet::new();

        let derivability = analyzer
            .compute_trait_derivability(&array_type, &mut analyzing_stack)
            .unwrap();

        // Array of f64 should NOT derive Eq/Ord (due to NaN)
        assert!(
            !derivability.can_derive_eq,
            "Array of f64 should not derive Eq"
        );
        assert!(
            !derivability.can_derive_ord,
            "Array of f64 should not derive Ord"
        );
    }

    #[test]
    fn test_compute_trait_derivability_array_with_primitive_integer() {
        let mut discovery = SchemaDiscovery::new();
        let config = OptimizationConfig::o0();

        use serde_json::json;

        // Create ArrayOfComplex with primitive integer items
        let array_type = TypeDiscovery::new(
            "IntegerList".to_string(),
            TypeKind::ArrayOfComplex,
            SchemaLocation {
                json_path: "#/IntegerList".to_string(),
                description: "Array of integers".to_string(),
                parent_type: None,
                property_name: None,
            },
            json!({
                "type": "array",
                "items": {
                    "type": "integer"
                }
            })
            .as_object()
            .unwrap()
            .clone(),
        );
        discovery.register_type(array_type.clone());

        let mut analyzer = TypeAnalyzer::new(discovery, config);
        let mut analyzing_stack = HashSet::new();

        let derivability = analyzer
            .compute_trait_derivability(&array_type, &mut analyzing_stack)
            .unwrap();

        // Array of integers should derive all traits
        assert!(
            derivability.can_derive_eq,
            "Array of integers should derive Eq"
        );
        assert!(
            derivability.can_derive_ord,
            "Array of integers should derive Ord"
        );
        assert!(
            derivability.can_derive_hash,
            "Array of integers should derive Hash"
        );
    }

    #[test]
    fn test_compute_trait_derivability_array_with_primitive_boolean() {
        let mut discovery = SchemaDiscovery::new();
        let config = OptimizationConfig::o0();

        use serde_json::json;

        // Create ArrayOfComplex with primitive boolean items
        let array_type = TypeDiscovery::new(
            "BoolList".to_string(),
            TypeKind::ArrayOfComplex,
            SchemaLocation {
                json_path: "#/BoolList".to_string(),
                description: "Array of booleans".to_string(),
                parent_type: None,
                property_name: None,
            },
            json!({
                "type": "array",
                "items": {
                    "type": "boolean"
                }
            })
            .as_object()
            .unwrap()
            .clone(),
        );
        discovery.register_type(array_type.clone());

        let mut analyzer = TypeAnalyzer::new(discovery, config);
        let mut analyzing_stack = HashSet::new();

        let derivability = analyzer
            .compute_trait_derivability(&array_type, &mut analyzing_stack)
            .unwrap();

        // Array of booleans should derive all traits
        assert!(
            derivability.can_derive_eq,
            "Array of booleans should derive Eq"
        );
        assert!(
            derivability.can_derive_ord,
            "Array of booleans should derive Ord"
        );
        assert!(
            derivability.can_derive_hash,
            "Array of booleans should derive Hash"
        );
    }

    #[test]
    fn test_compute_trait_derivability_reference_type() {
        let mut discovery = SchemaDiscovery::new();
        let config = OptimizationConfig::o0();

        use crate::generator::schema_discovery::MemberInfo;
        use serde_json::json;

        // Create target struct with f64 field (cannot derive Eq/Ord)
        let mut target_struct = create_test_type("Target", TypeKind::Struct);
        target_struct.add_member(MemberInfo {
            name: "value".to_string(),
            member_type: MemberType::Primitive("f64".to_string()),
            required: true,
            is_nullable: false,
            schema: json!({"type": "number"}).as_object().unwrap().clone(),
        });
        discovery.register_type(target_struct);

        // Create reference type
        let ref_type = create_test_type(
            "TargetRef",
            TypeKind::Reference {
                target: "Target".to_string(),
            },
        );
        discovery.register_type(ref_type.clone());

        let mut analyzer = TypeAnalyzer::new(discovery, config);
        let mut analyzing_stack = HashSet::new();

        let derivability = analyzer
            .compute_trait_derivability(&ref_type, &mut analyzing_stack)
            .unwrap();

        // Reference should inherit non-derivability from target (f64 cannot derive Eq/Ord)
        assert!(
            !derivability.can_derive_eq,
            "Reference to struct with f64 should not derive Eq"
        );
        assert!(
            !derivability.can_derive_ord,
            "Reference to struct with f64 should not derive Ord"
        );
    }

    #[test]
    fn test_compute_trait_derivability_primitive_wrapper_f64() {
        let mut discovery = SchemaDiscovery::new();
        let config = OptimizationConfig::o0();

        // Create primitive wrapper for f64
        let wrapper_type = create_test_type(
            "FloatWrapper",
            TypeKind::PrimitiveWrapper {
                primitive_type: "f64".to_string(),
            },
        );
        discovery.register_type(wrapper_type.clone());

        let mut analyzer = TypeAnalyzer::new(discovery, config);
        let mut analyzing_stack = HashSet::new();

        let derivability = analyzer
            .compute_trait_derivability(&wrapper_type, &mut analyzing_stack)
            .unwrap();

        // f64 wrapper should NOT derive Eq/Ord/Hash
        assert!(
            !derivability.can_derive_eq,
            "f64 wrapper should not derive Eq"
        );
        assert!(
            !derivability.can_derive_ord,
            "f64 wrapper should not derive Ord"
        );
        assert!(
            !derivability.can_derive_hash,
            "f64 wrapper should not derive Hash"
        );
    }

    #[test]
    fn test_compute_trait_derivability_primitive_wrapper_string() {
        let mut discovery = SchemaDiscovery::new();
        let config = OptimizationConfig::o0();

        // Create primitive wrapper for String
        let wrapper_type = create_test_type(
            "StringWrapper",
            TypeKind::PrimitiveWrapper {
                primitive_type: "String".to_string(),
            },
        );
        discovery.register_type(wrapper_type.clone());

        let mut analyzer = TypeAnalyzer::new(discovery, config);
        let mut analyzing_stack = HashSet::new();

        let derivability = analyzer
            .compute_trait_derivability(&wrapper_type, &mut analyzing_stack)
            .unwrap();

        // String wrapper should derive all traits
        assert!(
            derivability.can_derive_eq,
            "String wrapper should derive Eq"
        );
        assert!(
            derivability.can_derive_ord,
            "String wrapper should derive Ord"
        );
        assert!(
            derivability.can_derive_hash,
            "String wrapper should derive Hash"
        );
    }

    #[test]
    fn test_compute_trait_derivability_primitive_wrapper_unknown() {
        let mut discovery = SchemaDiscovery::new();
        let config = OptimizationConfig::o0();

        // Create primitive wrapper for unknown type
        let wrapper_type = create_test_type(
            "UnknownWrapper",
            TypeKind::PrimitiveWrapper {
                primitive_type: "UnknownPrimitive".to_string(),
            },
        );
        discovery.register_type(wrapper_type.clone());

        let mut analyzer = TypeAnalyzer::new(discovery, config);
        let mut analyzing_stack = HashSet::new();

        let derivability = analyzer
            .compute_trait_derivability(&wrapper_type, &mut analyzing_stack)
            .unwrap();

        // Unknown primitive wrapper should be conservative (no traits)
        assert!(
            !derivability.can_derive_eq,
            "Unknown primitive wrapper should not derive Eq"
        );
        assert!(
            !derivability.can_derive_ord,
            "Unknown primitive wrapper should not derive Ord"
        );
        assert!(
            !derivability.can_derive_hash,
            "Unknown primitive wrapper should not derive Hash"
        );
    }

    #[test]
    fn test_compute_trait_derivability_minimal_object_wrapper() {
        let mut discovery = SchemaDiscovery::new();
        let config = OptimizationConfig::o0();

        // Create minimal object wrapper
        let minimal_type = create_test_type("Minimal", TypeKind::MinimalObjectWrapper);
        discovery.register_type(minimal_type.clone());

        let mut analyzer = TypeAnalyzer::new(discovery, config);
        let mut analyzing_stack = HashSet::new();

        let derivability = analyzer
            .compute_trait_derivability(&minimal_type, &mut analyzing_stack)
            .unwrap();

        // MinimalObjectWrapper (serde_json::Value) should NOT derive Eq/Ord/Hash
        assert!(
            !derivability.can_derive_eq,
            "MinimalObjectWrapper should not derive Eq"
        );
        assert!(
            !derivability.can_derive_ord,
            "MinimalObjectWrapper should not derive Ord"
        );
        assert!(
            !derivability.can_derive_hash,
            "MinimalObjectWrapper should not derive Hash"
        );
    }

    #[test]
    fn test_analyze_member_derivability_type_reference_unanalyzed() {
        let mut discovery = SchemaDiscovery::new();
        let config = OptimizationConfig::o0();

        // Register a type but don't analyze it
        let target_type = create_test_type("Target", TypeKind::Struct);
        discovery.register_type(target_type);

        // Create analyzer but don't run full analysis
        let mut analyzer = TypeAnalyzer::new(discovery, config);
        let mut analyzing_stack = HashSet::new();

        // Test TypeReference to unanalyzed type
        let type_ref = MemberType::TypeReference("Target".to_string());

        // This should trigger recursive analysis of "Target"
        let result = analyzer
            .analyze_member_derivability_recursive(&type_ref, &mut analyzing_stack)
            .unwrap();

        // Should have successfully analyzed and returned derivability
        // Empty struct should be able to derive all traits
        assert!(result.can_derive_eq);
        assert!(result.can_derive_ord);
        assert!(result.can_derive_hash);
    }

    // ========================================================================================
    // Additional tests to push regions coverage from 81.34% to 85%+
    // ========================================================================================

    #[test]
    fn test_determine_member_token_structure_option_of_struct_all_optional() {
        let mut discovery = SchemaDiscovery::new();
        let config = OptimizationConfig::o0();

        use crate::generator::schema_discovery::MemberInfo;
        use serde_json::json;

        // Create struct with all optional fields
        let mut settings_struct = create_test_type("Settings", TypeKind::Struct);
        settings_struct.add_member(MemberInfo {
            name: "theme".to_string(),
            member_type: MemberType::Primitive("String".to_string()),
            required: false,
            is_nullable: false,
            schema: json!({"type": "string"}).as_object().unwrap().clone(),
        });
        settings_struct.add_member(MemberInfo {
            name: "lang".to_string(),
            member_type: MemberType::Primitive("String".to_string()),
            required: false,
            is_nullable: false,
            schema: json!({"type": "string"}).as_object().unwrap().clone(),
        });
        discovery.register_type(settings_struct);

        let analyzer = TypeAnalyzer::new(discovery, config);

        // Test Option<Settings> where Settings has all optional fields
        let option_member = MemberType::Container {
            container: "Option".to_string(),
            inner: "Settings".to_string(),
        };

        let result = analyzer
            .determine_member_token_structure(&option_member)
            .unwrap();

        // All fields optional → field_count should be 1 (minimum practical)
        assert!(matches!(result, TokenStructure::Object { field_count: 1 }));
    }

    #[test]
    fn test_determine_member_token_structure_box_of_struct_all_optional() {
        let mut discovery = SchemaDiscovery::new();
        let config = OptimizationConfig::o0();

        use crate::generator::schema_discovery::MemberInfo;
        use serde_json::json;

        // Create struct with all optional fields
        let mut config_struct = create_test_type("Config", TypeKind::Struct);
        config_struct.add_member(MemberInfo {
            name: "option1".to_string(),
            member_type: MemberType::Primitive("bool".to_string()),
            required: false,
            is_nullable: false,
            schema: json!({"type": "boolean"}).as_object().unwrap().clone(),
        });
        config_struct.add_member(MemberInfo {
            name: "option2".to_string(),
            member_type: MemberType::Primitive("bool".to_string()),
            required: false,
            is_nullable: false,
            schema: json!({"type": "boolean"}).as_object().unwrap().clone(),
        });
        discovery.register_type(config_struct);

        let analyzer = TypeAnalyzer::new(discovery, config);

        // Test Box<Config> where Config has all optional fields
        let box_member = MemberType::Container {
            container: "Box".to_string(),
            inner: "Config".to_string(),
        };

        let result = analyzer
            .determine_member_token_structure(&box_member)
            .unwrap();

        // All fields optional → field_count should be 1 (minimum practical)
        assert!(matches!(result, TokenStructure::Object { field_count: 1 }));
    }

    #[test]
    fn test_determine_member_token_structure_complex_container_option_empty_struct() {
        let mut discovery = SchemaDiscovery::new();
        let config = OptimizationConfig::o0();

        // Create empty struct
        let empty_struct = create_test_type("EmptyStruct", TypeKind::Struct);
        discovery.register_type(empty_struct);

        let analyzer = TypeAnalyzer::new(discovery, config);

        // Test ComplexContainer Option<EmptyStruct>
        let complex_option = MemberType::ComplexContainer {
            container: "Option".to_string(),
            inner: "EmptyStruct".to_string(),
        };

        let result = analyzer
            .determine_member_token_structure(&complex_option)
            .unwrap();

        // Option<EmptyStruct> should map to Null
        assert!(matches!(result, TokenStructure::Null));
    }

    #[test]
    fn test_determine_member_token_structure_complex_container_box_empty_struct() {
        let mut discovery = SchemaDiscovery::new();
        let config = OptimizationConfig::o0();

        // Create empty struct
        let empty_struct = create_test_type("EmptyStruct", TypeKind::Struct);
        discovery.register_type(empty_struct);

        let analyzer = TypeAnalyzer::new(discovery, config);

        // Test ComplexContainer Box<EmptyStruct>
        let complex_box = MemberType::ComplexContainer {
            container: "Box".to_string(),
            inner: "EmptyStruct".to_string(),
        };

        let result = analyzer
            .determine_member_token_structure(&complex_box)
            .unwrap();

        // Box<EmptyStruct> should map to Null
        assert!(matches!(result, TokenStructure::Null));
    }

    #[test]
    fn test_determine_member_token_structure_complex_container_option_enum_optimized() {
        let mut discovery = SchemaDiscovery::new();
        let config = OptimizationConfig::o2(); // Enable enum optimization

        use crate::generator::schema_discovery::MemberInfo;
        use serde_json::json;

        // Create simple enum
        let mut status_enum = create_test_type("Status", TypeKind::Enum);
        status_enum.add_member(MemberInfo {
            name: "On".to_string(),
            member_type: MemberType::Primitive("()".to_string()),
            required: true,
            is_nullable: false,
            schema: json!({"const": "On"}).as_object().unwrap().clone(),
        });
        status_enum.add_member(MemberInfo {
            name: "Off".to_string(),
            member_type: MemberType::Primitive("()".to_string()),
            required: true,
            is_nullable: false,
            schema: json!({"const": "Off"}).as_object().unwrap().clone(),
        });
        discovery.register_type(status_enum);

        let analyzer = TypeAnalyzer::new(discovery, config);

        // Test ComplexContainer Option<Status> with optimization
        let complex_option = MemberType::ComplexContainer {
            container: "Option".to_string(),
            inner: "Status".to_string(),
        };

        let result = analyzer
            .determine_member_token_structure(&complex_option)
            .unwrap();

        // Option<Enum> with optimization should map to Primitive
        assert!(matches!(result, TokenStructure::Primitive(0)));
    }

    #[test]
    fn test_determine_member_token_structure_complex_container_option_reference() {
        let mut discovery = SchemaDiscovery::new();
        let config = OptimizationConfig::o0();

        // Create reference type
        let ref_type = create_test_type(
            "TargetRef",
            TypeKind::Reference {
                target: "Target".to_string(),
            },
        );
        discovery.register_type(ref_type);

        let analyzer = TypeAnalyzer::new(discovery, config);

        // Test ComplexContainer Option<TargetRef>
        let complex_option = MemberType::ComplexContainer {
            container: "Option".to_string(),
            inner: "TargetRef".to_string(),
        };

        let result = analyzer
            .determine_member_token_structure(&complex_option)
            .unwrap();

        // Option<Reference> should map to Primitive
        assert!(matches!(result, TokenStructure::Primitive(0)));
    }

    #[test]
    fn test_determine_member_token_structure_complex_container_option_array() {
        let mut discovery = SchemaDiscovery::new();
        let config = OptimizationConfig::o0();

        // Create array type
        let array_type = create_test_type("ItemList", TypeKind::ArrayOfComplex);
        discovery.register_type(array_type);

        let analyzer = TypeAnalyzer::new(discovery, config);

        // Test ComplexContainer Option<ItemList>
        let complex_option = MemberType::ComplexContainer {
            container: "Option".to_string(),
            inner: "ItemList".to_string(),
        };

        let result = analyzer
            .determine_member_token_structure(&complex_option)
            .unwrap();

        // Option<Array> should map to Array
        assert!(matches!(result, TokenStructure::Array { length: 0 }));
    }

    #[test]
    fn test_determine_member_token_structure_complex_container_option_primitive_wrapper() {
        let mut discovery = SchemaDiscovery::new();
        let config = OptimizationConfig::o0();

        // Create primitive wrapper type
        let wrapper_type = create_test_type(
            "Count",
            TypeKind::PrimitiveWrapper {
                primitive_type: "i64".to_string(),
            },
        );
        discovery.register_type(wrapper_type);

        let analyzer = TypeAnalyzer::new(discovery, config);

        // Test ComplexContainer Option<Count>
        let complex_option = MemberType::ComplexContainer {
            container: "Option".to_string(),
            inner: "Count".to_string(),
        };

        let result = analyzer
            .determine_member_token_structure(&complex_option)
            .unwrap();

        // Option<PrimitiveWrapper> should map to Primitive
        assert!(matches!(result, TokenStructure::Primitive(0)));
    }

    #[test]
    fn test_determine_member_token_structure_complex_container_option_minimal_object() {
        let mut discovery = SchemaDiscovery::new();
        let config = OptimizationConfig::o0();

        // Create minimal object wrapper type
        let minimal_type = create_test_type("Data", TypeKind::MinimalObjectWrapper);
        discovery.register_type(minimal_type);

        let analyzer = TypeAnalyzer::new(discovery, config);

        // Test ComplexContainer Option<Data>
        let complex_option = MemberType::ComplexContainer {
            container: "Option".to_string(),
            inner: "Data".to_string(),
        };

        let result = analyzer
            .determine_member_token_structure(&complex_option)
            .unwrap();

        // Option<MinimalObjectWrapper> should map to Object
        assert!(matches!(result, TokenStructure::Object { field_count: 0 }));
    }

    #[test]
    fn test_determine_member_token_structure_complex_container_option_not_found() {
        let discovery = SchemaDiscovery::new();
        let config = OptimizationConfig::o0();

        let analyzer = TypeAnalyzer::new(discovery, config);

        // Test ComplexContainer Option<UnknownType> (type not found)
        let complex_option = MemberType::ComplexContainer {
            container: "Option".to_string(),
            inner: "UnknownType".to_string(),
        };

        let result = analyzer
            .determine_member_token_structure(&complex_option)
            .unwrap();

        // Unknown type should fall back to Primitive(0)
        assert!(matches!(result, TokenStructure::Primitive(0)));
    }

    #[test]
    fn test_compute_trait_derivability_array_with_primitive_unknown() {
        let mut discovery = SchemaDiscovery::new();
        let config = OptimizationConfig::o0();

        use serde_json::json;

        // Create ArrayOfComplex with unknown primitive type
        let array_type = TypeDiscovery::new(
            "UnknownList".to_string(),
            TypeKind::ArrayOfComplex,
            SchemaLocation {
                json_path: "#/UnknownList".to_string(),
                description: "Array of unknown type".to_string(),
                parent_type: None,
                property_name: None,
            },
            json!({
                "type": "array",
                "items": {
                    "type": "unknownType"
                }
            })
            .as_object()
            .unwrap()
            .clone(),
        );
        discovery.register_type(array_type.clone());

        let mut analyzer = TypeAnalyzer::new(discovery, config);
        let mut analyzing_stack = HashSet::new();

        let derivability = analyzer
            .compute_trait_derivability(&array_type, &mut analyzing_stack)
            .unwrap();

        // Unknown primitive type should have reasoning part added
        // but default behavior continues (no change to derivability)
        // The reasoning should mention the unknown type
        assert!(derivability.reasoning.contains("unknown element type"));
    }

    #[test]
    fn test_compute_trait_derivability_array_missing_ref_target() {
        let mut discovery = SchemaDiscovery::new();
        let config = OptimizationConfig::o0();

        use serde_json::json;

        // Create ArrayOfComplex with $ref to non-existent type
        let array_type = TypeDiscovery::new(
            "MissingRefList".to_string(),
            TypeKind::ArrayOfComplex,
            SchemaLocation {
                json_path: "#/MissingRefList".to_string(),
                description: "Array referencing missing type".to_string(),
                parent_type: None,
                property_name: None,
            },
            json!({
                "type": "array",
                "items": {
                    "$ref": "#/definitions/NonExistent"
                }
            })
            .as_object()
            .unwrap()
            .clone(),
        );
        discovery.register_type(array_type.clone());

        let mut analyzer = TypeAnalyzer::new(discovery, config);
        let mut analyzing_stack = HashSet::new();

        let derivability = analyzer
            .compute_trait_derivability(&array_type, &mut analyzing_stack)
            .unwrap();

        // Missing reference target should use conservative fallback
        assert!(
            !derivability.can_derive_eq,
            "Array with missing ref target should not derive Eq (conservative)"
        );
        assert!(
            !derivability.can_derive_ord,
            "Array with missing ref target should not derive Ord (conservative)"
        );
        assert!(
            !derivability.can_derive_hash,
            "Array with missing ref target should not derive Hash (conservative)"
        );
        assert!(derivability.reasoning.contains("not analyzable"));
    }

    #[test]
    fn test_compute_trait_derivability_reference_to_nonexistent_target() {
        let mut discovery = SchemaDiscovery::new();
        let config = OptimizationConfig::o0();

        // Create reference to non-existent target
        let ref_type = create_test_type(
            "BrokenRef",
            TypeKind::Reference {
                target: "NonExistentTarget".to_string(),
            },
        );
        discovery.register_type(ref_type.clone());

        let mut analyzer = TypeAnalyzer::new(discovery, config);
        let mut analyzing_stack = HashSet::new();

        let derivability = analyzer
            .compute_trait_derivability(&ref_type, &mut analyzing_stack)
            .unwrap();

        // Reference to non-existent target should use conservative fallback
        assert!(
            !derivability.can_derive_eq,
            "Reference to non-existent target should not derive Eq (conservative)"
        );
        assert!(
            !derivability.can_derive_ord,
            "Reference to non-existent target should not derive Ord (conservative)"
        );
        assert!(
            !derivability.can_derive_hash,
            "Reference to non-existent target should not derive Hash (conservative)"
        );
    }

    #[test]
    fn test_analyze_member_derivability_complex_container_option() {
        let mut discovery = SchemaDiscovery::new();
        let config = OptimizationConfig::o0();

        use crate::generator::schema_discovery::MemberInfo;
        use serde_json::json;

        // Create struct with integer field
        let mut data_struct = create_test_type("Data", TypeKind::Struct);
        data_struct.add_member(MemberInfo {
            name: "value".to_string(),
            member_type: MemberType::Primitive("i64".to_string()),
            required: true,
            is_nullable: false,
            schema: json!({"type": "integer"}).as_object().unwrap().clone(),
        });
        discovery.register_type(data_struct);

        let mut analyzer = TypeAnalyzer::new(discovery, config);
        let mut analyzing_stack = HashSet::new();

        // Test Option<Data> derivability (ComplexContainer)
        let option_complex = MemberType::ComplexContainer {
            container: "Option".to_string(),
            inner: "Data".to_string(),
        };

        let result = analyzer
            .analyze_member_derivability_recursive(&option_complex, &mut analyzing_stack)
            .unwrap();

        // Option<Struct with integer> should derive all traits
        assert!(result.can_derive_eq);
        assert!(result.can_derive_ord);
        assert!(result.can_derive_hash);
    }

    #[test]
    fn test_determine_member_token_structure_type_reference_enum() {
        let mut discovery = SchemaDiscovery::new();
        let config = OptimizationConfig::o0();

        // Create enum type (no optimization)
        let enum_type = create_test_type("Status", TypeKind::Enum);
        discovery.register_type(enum_type);

        let analyzer = TypeAnalyzer::new(discovery, config);

        // Test TypeReference to enum
        let type_ref = MemberType::TypeReference("Status".to_string());

        let result = analyzer
            .determine_member_token_structure(&type_ref)
            .unwrap();

        // Enum without optimization should map to Array
        assert!(matches!(result, TokenStructure::Array { length: 0 }));
    }

    #[test]
    fn test_determine_member_token_structure_type_reference_array() {
        let mut discovery = SchemaDiscovery::new();
        let config = OptimizationConfig::o0();

        // Create array type
        let array_type = create_test_type("ItemList", TypeKind::ArrayOfComplex);
        discovery.register_type(array_type);

        let analyzer = TypeAnalyzer::new(discovery, config);

        // Test TypeReference to array
        let type_ref = MemberType::TypeReference("ItemList".to_string());

        let result = analyzer
            .determine_member_token_structure(&type_ref)
            .unwrap();

        // Array should map to Array structure
        assert!(matches!(result, TokenStructure::Array { length: 0 }));
    }

    #[test]
    fn test_determine_member_token_structure_type_reference_reference() {
        let mut discovery = SchemaDiscovery::new();
        let config = OptimizationConfig::o0();

        // Create reference type (reference to reference)
        let ref_type = create_test_type(
            "UserRef",
            TypeKind::Reference {
                target: "User".to_string(),
            },
        );
        discovery.register_type(ref_type);

        let analyzer = TypeAnalyzer::new(discovery, config);

        // Test TypeReference to reference (reference to reference)
        let type_ref = MemberType::TypeReference("UserRef".to_string());

        let result = analyzer
            .determine_member_token_structure(&type_ref)
            .unwrap();

        // Reference to reference should map to Primitive (safe fallback)
        assert!(matches!(result, TokenStructure::Primitive(0)));
    }

    #[test]
    fn test_determine_member_token_structure_type_reference_primitive_wrapper() {
        let mut discovery = SchemaDiscovery::new();
        let config = OptimizationConfig::o0();

        // Create primitive wrapper type
        let wrapper_type = create_test_type(
            "Count",
            TypeKind::PrimitiveWrapper {
                primitive_type: "i64".to_string(),
            },
        );
        discovery.register_type(wrapper_type);

        let analyzer = TypeAnalyzer::new(discovery, config);

        // Test TypeReference to primitive wrapper
        let type_ref = MemberType::TypeReference("Count".to_string());

        let result = analyzer
            .determine_member_token_structure(&type_ref)
            .unwrap();

        // PrimitiveWrapper should map to Primitive
        assert!(matches!(result, TokenStructure::Primitive(0)));
    }

    #[test]
    fn test_determine_member_token_structure_type_reference_minimal_object() {
        let mut discovery = SchemaDiscovery::new();
        let config = OptimizationConfig::o0();

        // Create minimal object wrapper type
        let minimal_type = create_test_type("Data", TypeKind::MinimalObjectWrapper);
        discovery.register_type(minimal_type);

        let analyzer = TypeAnalyzer::new(discovery, config);

        // Test TypeReference to minimal object wrapper
        let type_ref = MemberType::TypeReference("Data".to_string());

        let result = analyzer
            .determine_member_token_structure(&type_ref)
            .unwrap();

        // MinimalObjectWrapper should map to Object
        assert!(matches!(result, TokenStructure::Object { field_count: 0 }));
    }

    // ========================================================================================
    // Final tests to reach 85% regions coverage
    // ========================================================================================

    #[test]
    fn test_compute_trait_derivability_pattern_validator_special_case() {
        let mut discovery = SchemaDiscovery::new();
        let config = OptimizationConfig::o0();

        // Create a type with "PatternValidator" in the name (special case)
        let validator_type = create_test_type("SomePatternValidator", TypeKind::Struct);
        discovery.register_type(validator_type.clone());

        let mut analyzer = TypeAnalyzer::new(discovery, config);
        let mut analyzing_stack = HashSet::new();

        let derivability = analyzer
            .compute_trait_derivability(&validator_type, &mut analyzing_stack)
            .unwrap();

        // PatternValidator types should NOT derive Eq/Ord/Hash (contains HashMap and Regex)
        assert!(
            !derivability.can_derive_eq,
            "PatternValidator should not derive Eq"
        );
        assert!(
            !derivability.can_derive_ord,
            "PatternValidator should not derive Ord"
        );
        assert!(
            !derivability.can_derive_hash,
            "PatternValidator should not derive Hash"
        );
        assert!(derivability.reasoning.contains("HashMap"));
        assert!(
            derivability.reasoning.contains("Regex") || derivability.reasoning.contains("regex")
        );
    }

    #[test]
    fn test_compute_trait_derivability_nested_struct() {
        let mut discovery = SchemaDiscovery::new();
        let config = OptimizationConfig::o0();

        use crate::generator::schema_discovery::MemberInfo;
        use serde_json::json;

        // Create NestedStruct type (different from regular Struct)
        let mut nested_struct = create_test_type("NestedData", TypeKind::NestedStruct);
        nested_struct.add_member(MemberInfo {
            name: "value".to_string(),
            member_type: MemberType::Primitive("i64".to_string()),
            required: true,
            is_nullable: false,
            schema: json!({"type": "integer"}).as_object().unwrap().clone(),
        });
        discovery.register_type(nested_struct.clone());

        let mut analyzer = TypeAnalyzer::new(discovery, config);
        let mut analyzing_stack = HashSet::new();

        let derivability = analyzer
            .compute_trait_derivability(&nested_struct, &mut analyzing_stack)
            .unwrap();

        // NestedStruct with integer field should derive all traits
        assert!(derivability.can_derive_eq);
        assert!(derivability.can_derive_ord);
        assert!(derivability.can_derive_hash);
    }

    #[test]
    fn test_compute_trait_derivability_enum_with_data_variants() {
        let mut discovery = SchemaDiscovery::new();
        let config = OptimizationConfig::o0();

        use crate::generator::schema_discovery::MemberInfo;
        use serde_json::json;

        // Create enum with data variants (not just unit variants)
        let mut data_enum = create_test_type("Result", TypeKind::Enum);

        // Add variant with struct data
        data_enum.add_member(MemberInfo {
            name: "Ok".to_string(),
            member_type: MemberType::TypeReference("Value".to_string()),
            required: true,
            is_nullable: false,
            schema: json!({"type": "object", "properties": {"value": {"type": "string"}}})
                .as_object()
                .unwrap()
                .clone(),
        });

        // Add variant with f64 data (cannot derive Eq/Ord)
        data_enum.add_member(MemberInfo {
            name: "Err".to_string(),
            member_type: MemberType::Primitive("f64".to_string()),
            required: true,
            is_nullable: false,
            schema: json!({"type": "number"}).as_object().unwrap().clone(),
        });

        discovery.register_type(data_enum.clone());

        let mut analyzer = TypeAnalyzer::new(discovery, config);
        let mut analyzing_stack = HashSet::new();

        let derivability = analyzer
            .compute_trait_derivability(&data_enum, &mut analyzing_stack)
            .unwrap();

        // Enum with f64 variant should NOT derive Eq/Ord
        assert!(
            !derivability.can_derive_eq,
            "Enum with f64 variant should not derive Eq"
        );
        assert!(
            !derivability.can_derive_ord,
            "Enum with f64 variant should not derive Ord"
        );
    }

    #[test]
    fn test_determine_member_token_structure_type_reference_enum_with_optimization() {
        let mut discovery = SchemaDiscovery::new();
        let config = OptimizationConfig::o2(); // Enable enum optimization

        use crate::generator::schema_discovery::MemberInfo;
        use serde_json::json;

        // Create simple enum with unit variants
        let mut status_enum = create_test_type("Status", TypeKind::Enum);
        status_enum.add_member(MemberInfo {
            name: "Active".to_string(),
            member_type: MemberType::Primitive("()".to_string()),
            required: true,
            is_nullable: false,
            schema: json!({"const": "Active"}).as_object().unwrap().clone(),
        });
        status_enum.add_member(MemberInfo {
            name: "Inactive".to_string(),
            member_type: MemberType::Primitive("()".to_string()),
            required: true,
            is_nullable: false,
            schema: json!({"const": "Inactive"}).as_object().unwrap().clone(),
        });
        discovery.register_type(status_enum);

        let analyzer = TypeAnalyzer::new(discovery, config);

        // Test TypeReference to simple enum with optimization
        let type_ref = MemberType::TypeReference("Status".to_string());

        let result = analyzer
            .determine_member_token_structure(&type_ref)
            .unwrap();

        // Simple enum with optimization should map to Primitive
        assert!(matches!(result, TokenStructure::Primitive(0)));
    }

    #[test]
    fn test_compute_decode_strategy_for_struct_all_required_fields() {
        let mut discovery = SchemaDiscovery::new();
        let config = OptimizationConfig::o0();

        use crate::generator::schema_discovery::MemberInfo;
        use serde_json::json;

        // Create struct with all required fields (no optional)
        let mut point_struct = create_test_type("Point", TypeKind::Struct);
        point_struct.add_member(MemberInfo {
            name: "x".to_string(),
            member_type: MemberType::Primitive("i64".to_string()),
            required: true,
            is_nullable: false,
            schema: json!({"type": "integer"}).as_object().unwrap().clone(),
        });
        point_struct.add_member(MemberInfo {
            name: "y".to_string(),
            member_type: MemberType::Primitive("i64".to_string()),
            required: true,
            is_nullable: false,
            schema: json!({"type": "integer"}).as_object().unwrap().clone(),
        });
        discovery.register_type(point_struct.clone());

        let analyzer = TypeAnalyzer::new(discovery, config);

        // Test decode strategy for struct with all required fields
        let strategy = analyzer
            .compute_decode_strategy("Point", &point_struct)
            .unwrap();

        // All required fields → traditional approach
        assert_eq!(strategy.field_order.len(), 2);
        assert_eq!(strategy.field_order[0], "x");
        assert_eq!(strategy.field_order[1], "y");
    }

    #[test]
    fn test_analyze_type_recursive_unknown_type() {
        // Test analyze_type_recursive when type is not in discovered_types
        let discovery = SchemaDiscovery::new();
        let config = OptimizationConfig::o0();

        let mut analyzer = TypeAnalyzer::new(discovery, config);
        let mut analyzing_stack = std::collections::HashSet::new();

        // Try to analyze a type that doesn't exist
        let result = analyzer.analyze_type_recursive("UnknownType", &mut analyzing_stack);

        // Should succeed (no-op for unknown types)
        assert!(result.is_ok());
    }

    #[test]
    fn test_topological_sort_with_no_dependencies() {
        // Test topological_sort_visit when type has no dependencies
        let mut discovery = SchemaDiscovery::new();
        let config = OptimizationConfig::o0();

        let standalone_type = create_test_type("Standalone", TypeKind::Struct);
        discovery.register_type(standalone_type);

        let mut analyzer = TypeAnalyzer::new(discovery, config);
        analyzer.analysis.dependency_graph.dependencies.clear(); // Ensure no dependencies

        let mut visited = std::collections::HashSet::new();
        let mut temp_visited = std::collections::HashSet::new();
        let mut ordered_types = Vec::new();

        let result = analyzer.topological_sort_visit(
            "Standalone",
            &mut visited,
            &mut temp_visited,
            &mut ordered_types,
        );

        assert!(result.is_ok());
        assert_eq!(ordered_types, vec!["Standalone"]);
    }

    #[test]
    fn test_calculate_dependency_depth_no_dependencies() {
        // Test calculate_dependency_depth when type has no dependencies
        let mut discovery = SchemaDiscovery::new();
        let config = OptimizationConfig::o0();

        let standalone_type = create_test_type("Standalone", TypeKind::Struct);
        discovery.register_type(standalone_type);

        let mut analyzer = TypeAnalyzer::new(discovery, config);
        analyzer.analysis.dependency_graph.dependencies.clear(); // Ensure no dependencies

        let mut visited = std::collections::HashSet::new();
        let depth = analyzer.calculate_dependency_depth("Standalone", &mut visited);

        // Standalone type should have depth 1 (no dependencies)
        assert_eq!(depth, 1);
    }

    #[test]
    fn test_determine_member_token_structure_container_hashmap() {
        // Test HashMap container type
        let discovery = SchemaDiscovery::new();
        let config = OptimizationConfig::o0();

        let analyzer = TypeAnalyzer::new(discovery, config);
        let hashmap_member = MemberType::Container {
            container: "HashMap".to_string(),
            inner: "String".to_string(),
        };

        let result = analyzer
            .determine_member_token_structure(&hashmap_member)
            .unwrap();

        assert!(matches!(result, TokenStructure::Object { field_count: 0 }));
    }

    #[test]
    fn test_determine_member_token_structure_container_btreemap() {
        // Test BTreeMap container type
        let discovery = SchemaDiscovery::new();
        let config = OptimizationConfig::o0();

        let analyzer = TypeAnalyzer::new(discovery, config);
        let btreemap_member = MemberType::Container {
            container: "BTreeMap".to_string(),
            inner: "String".to_string(),
        };

        let result = analyzer
            .determine_member_token_structure(&btreemap_member)
            .unwrap();

        assert!(matches!(result, TokenStructure::Object { field_count: 0 }));
    }

    #[test]
    fn test_determine_member_token_structure_complex_container_unknown_type() {
        // Test ComplexContainer with unknown container type
        let discovery = SchemaDiscovery::new();
        let config = OptimizationConfig::o0();

        let analyzer = TypeAnalyzer::new(discovery, config);
        let complex_unknown = MemberType::ComplexContainer {
            container: "UnknownContainer".to_string(),
            inner: "String".to_string(),
        };

        let result = analyzer
            .determine_member_token_structure(&complex_unknown)
            .unwrap();

        // Unknown complex container should fallback to Primitive(0)
        assert!(matches!(result, TokenStructure::Primitive(0)));
    }

    #[test]
    fn test_analyze_member_derivability_recursive_hashmap_with_value_type() {
        // Test analyze_member_derivability_recursive for HashMap<String, T>
        let discovery = SchemaDiscovery::new();
        let config = OptimizationConfig::o0();

        let mut analyzer = TypeAnalyzer::new(discovery, config);
        let mut analyzing_stack = std::collections::HashSet::new();

        let hashmap_member = MemberType::HashMap {
            key: "String".to_string(),
            value: "i64".to_string(),
        };

        let result =
            analyzer.analyze_member_derivability_recursive(&hashmap_member, &mut analyzing_stack);

        assert!(result.is_ok());
        let derivability = result.unwrap();

        // HashMap cannot derive Eq/Ord/Hash
        assert!(!derivability.can_derive_eq);
        assert!(!derivability.can_derive_ord);
        assert!(!derivability.can_derive_hash);
    }

    #[test]
    fn test_analyze_member_derivability_recursive_pattern_properties() {
        // Test analyze_member_derivability_recursive for PatternProperties
        let discovery = SchemaDiscovery::new();
        let config = OptimizationConfig::o0();

        let mut analyzer = TypeAnalyzer::new(discovery, config);
        let mut analyzing_stack = std::collections::HashSet::new();

        let pattern_props = MemberType::PatternProperties { validators: vec![] };

        let result =
            analyzer.analyze_member_derivability_recursive(&pattern_props, &mut analyzing_stack);

        assert!(result.is_ok());
        let derivability = result.unwrap();

        // PatternProperties cannot derive Eq/Ord/Hash
        assert!(!derivability.can_derive_eq);
        assert!(!derivability.can_derive_ord);
        assert!(!derivability.can_derive_hash);
    }

    #[test]
    fn test_analyze_primitive_derivability_for_all_integer_types() {
        // Test analyze_primitive_derivability for all integer types
        let discovery = SchemaDiscovery::new();
        let config = OptimizationConfig::o0();

        let analyzer = TypeAnalyzer::new(discovery, config);

        let integer_types = vec![
            "i8", "i16", "i32", "i64", "i128", "isize", "u8", "u16", "u32", "u64", "u128", "usize",
        ];

        for int_type in integer_types {
            let derivability = analyzer.analyze_primitive_derivability(int_type);

            // All integer types should derive all traits
            assert!(derivability.can_derive_eq, "{} should derive Eq", int_type);
            assert!(
                derivability.can_derive_ord,
                "{} should derive Ord",
                int_type
            );
            assert!(
                derivability.can_derive_hash,
                "{} should derive Hash",
                int_type
            );
        }
    }

    #[test]
    fn test_analyze_primitive_derivability_f32_and_f64() {
        // Test analyze_primitive_derivability for floating point types
        let discovery = SchemaDiscovery::new();
        let config = OptimizationConfig::o0();

        let analyzer = TypeAnalyzer::new(discovery, config);

        for float_type in &["f32", "f64"] {
            let derivability = analyzer.analyze_primitive_derivability(float_type);

            // Floating point types cannot derive Eq/Ord/Hash due to NaN
            assert!(
                !derivability.can_derive_eq,
                "{} should not derive Eq",
                float_type
            );
            assert!(
                !derivability.can_derive_ord,
                "{} should not derive Ord",
                float_type
            );
            assert!(
                !derivability.can_derive_hash,
                "{} should not derive Hash",
                float_type
            );
        }
    }

    #[test]
    fn test_analyze_primitive_derivability_string_and_bool() {
        // Test analyze_primitive_derivability for String and bool
        let discovery = SchemaDiscovery::new();
        let config = OptimizationConfig::o0();

        let analyzer = TypeAnalyzer::new(discovery, config);

        // Test String
        let string_derivability = analyzer.analyze_primitive_derivability("String");
        assert!(string_derivability.can_derive_eq);
        assert!(string_derivability.can_derive_ord);
        assert!(string_derivability.can_derive_hash);

        // Test bool
        let bool_derivability = analyzer.analyze_primitive_derivability("bool");
        assert!(bool_derivability.can_derive_eq);
        assert!(bool_derivability.can_derive_ord);
        assert!(bool_derivability.can_derive_hash);
    }

    #[test]
    fn test_analyze_primitive_derivability_unknown_type() {
        // Test analyze_primitive_derivability for unknown primitive type
        let discovery = SchemaDiscovery::new();
        let config = OptimizationConfig::o0();

        let analyzer = TypeAnalyzer::new(discovery, config);

        let derivability = analyzer.analyze_primitive_derivability("UnknownType");

        // Unknown types should be conservative (no traits)
        assert!(!derivability.can_derive_eq);
        assert!(!derivability.can_derive_ord);
        assert!(!derivability.can_derive_hash);
    }

    #[test]
    fn test_compute_decode_strategy_for_mixed_optional_required() {
        use crate::generator::schema_discovery::MemberInfo;
        use serde_json::json;

        let mut discovery = SchemaDiscovery::new();
        let config = OptimizationConfig::o0();

        // Create struct with mixed optional and required fields
        let mut mixed_struct = create_test_type("MixedStruct", TypeKind::Struct);
        mixed_struct.add_member(MemberInfo {
            name: "required_field".to_string(),
            member_type: MemberType::Primitive("String".to_string()),
            required: true,
            is_nullable: false,
            schema: json!({"type": "string"}).as_object().unwrap().clone(),
        });
        mixed_struct.add_member(MemberInfo {
            name: "optional_field".to_string(),
            member_type: MemberType::Primitive("i64".to_string()),
            required: false,
            is_nullable: true,
            schema: json!({"type": "integer"}).as_object().unwrap().clone(),
        });
        discovery.register_type(mixed_struct.clone());

        let analyzer = TypeAnalyzer::new(discovery, config);

        let strategy = analyzer
            .compute_decode_strategy("MixedStruct", &mixed_struct)
            .unwrap();

        // Should have field order for both fields
        assert_eq!(strategy.field_order.len(), 2);
    }

    #[test]
    fn test_compute_decode_strategy_for_enum() {
        let mut discovery = SchemaDiscovery::new();
        let config = OptimizationConfig::o0();

        let enum_type = create_test_type("Status", TypeKind::Enum);
        discovery.register_type(enum_type.clone());

        let analyzer = TypeAnalyzer::new(discovery, config);

        let strategy = analyzer
            .compute_decode_strategy("Status", &enum_type)
            .unwrap();

        // Enum should have empty field order
        assert_eq!(strategy.field_order.len(), 0);
    }

    #[test]
    fn test_compute_decode_strategy_for_array_of_complex() {
        let mut discovery = SchemaDiscovery::new();
        let config = OptimizationConfig::o0();

        let array_type = create_test_type("ItemList", TypeKind::ArrayOfComplex);
        discovery.register_type(array_type.clone());

        let analyzer = TypeAnalyzer::new(discovery, config);

        let strategy = analyzer
            .compute_decode_strategy("ItemList", &array_type)
            .unwrap();

        // Array should have empty field order
        assert_eq!(strategy.field_order.len(), 0);
    }

    #[test]
    fn test_compute_decode_strategy_for_minimal_object_wrapper() {
        let mut discovery = SchemaDiscovery::new();
        let config = OptimizationConfig::o0();

        let wrapper_type = create_test_type("MinimalWrapper", TypeKind::MinimalObjectWrapper);
        discovery.register_type(wrapper_type.clone());

        let analyzer = TypeAnalyzer::new(discovery, config);

        let strategy = analyzer
            .compute_decode_strategy("MinimalWrapper", &wrapper_type)
            .unwrap();

        // Minimal object wrapper should have empty field order
        assert_eq!(strategy.field_order.len(), 0);
    }

    #[test]
    fn test_map_primitive_to_token_structure_for_all_types() {
        // Test map_primitive_to_token_structure for various primitive types
        let discovery = SchemaDiscovery::new();
        let config = OptimizationConfig::o0();

        let analyzer = TypeAnalyzer::new(discovery, config);

        // Integer types
        for int_type in &[
            "i8", "i16", "i32", "i64", "i128", "isize", "u8", "u16", "u32", "u64", "u128", "usize",
        ] {
            let structure = analyzer.map_primitive_to_token_structure(int_type);
            assert!(matches!(structure, TokenStructure::Primitive(0)));
        }

        // Floating point types
        for float_type in &["f32", "f64"] {
            let structure = analyzer.map_primitive_to_token_structure(float_type);
            assert!(matches!(structure, TokenStructure::Primitive(0)));
        }

        // String - special case (mapped to String structure, not Primitive)
        let string_structure = analyzer.map_primitive_to_token_structure("String");
        assert!(matches!(
            string_structure,
            TokenStructure::String { length: 0 }
        ));

        // Bool
        let bool_structure = analyzer.map_primitive_to_token_structure("bool");
        assert!(matches!(bool_structure, TokenStructure::Primitive(0)));

        // Unknown type
        let unknown_structure = analyzer.map_primitive_to_token_structure("UnknownType");
        assert!(matches!(unknown_structure, TokenStructure::Primitive(0)));
    }

    #[test]
    fn test_extract_dependency_from_member_for_all_member_types() {
        // Test extract_dependency_from_member for various MemberType variants
        let discovery = SchemaDiscovery::new();
        let config = OptimizationConfig::o0();

        let analyzer = TypeAnalyzer::new(discovery, config);

        // Primitive - no dependency
        let prim = MemberType::Primitive("String".to_string());
        assert_eq!(analyzer.extract_dependency_from_member(&prim), None);

        // Container - no dependency (inner is primitive)
        let container = MemberType::Container {
            container: "Vec".to_string(),
            inner: "String".to_string(),
        };
        assert_eq!(analyzer.extract_dependency_from_member(&container), None);

        // TypeReference - has dependency
        let type_ref = MemberType::TypeReference("User".to_string());
        assert_eq!(
            analyzer.extract_dependency_from_member(&type_ref),
            Some("User".to_string())
        );

        // ComplexContainer - has dependency
        let complex_container = MemberType::ComplexContainer {
            container: "Option".to_string(),
            inner: "User".to_string(),
        };
        assert_eq!(
            analyzer.extract_dependency_from_member(&complex_container),
            Some("User".to_string())
        );

        // HashMap - no dependency (values are primitive-typed)
        let hashmap = MemberType::HashMap {
            key: "String".to_string(),
            value: "String".to_string(),
        };
        assert_eq!(analyzer.extract_dependency_from_member(&hashmap), None);

        // PatternProperties - no dependency
        let pattern_props = MemberType::PatternProperties { validators: vec![] };
        assert_eq!(
            analyzer.extract_dependency_from_member(&pattern_props),
            None
        );
    }

    #[test]
    fn test_should_enum_optimize_to_primitive_with_optimization_disabled() {
        // Test should_enum_optimize_to_primitive when optimization is disabled
        use serde_json::json;

        let mut discovery = SchemaDiscovery::new();
        let config = OptimizationConfig::o0(); // O0 = no optimization

        // Create simple enum
        let mut simple_enum = create_test_type("Status", TypeKind::Enum);
        simple_enum.add_member(crate::generator::schema_discovery::MemberInfo {
            name: "active".to_string(),
            member_type: MemberType::Primitive("()".to_string()),
            required: true,
            is_nullable: false,
            schema: json!({"const": "active"}).as_object().unwrap().clone(),
        });
        discovery.register_type(simple_enum.clone());

        let analyzer = TypeAnalyzer::new(discovery, config);

        // Should NOT optimize when optimization is disabled
        assert!(!analyzer.should_enum_optimize_to_primitive(&simple_enum));
    }

    #[test]
    fn test_should_enum_optimize_to_primitive_with_data_variant() {
        // Test should_enum_optimize_to_primitive with enum that has data variants
        use crate::generator::schema_discovery::MemberInfo;
        use serde_json::json;

        let mut discovery = SchemaDiscovery::new();
        let config = OptimizationConfig::o2(); // O2 = with optimization

        // Create enum with data variant (not a unit type)
        let mut data_enum = create_test_type("Result", TypeKind::Enum);
        data_enum.add_member(MemberInfo {
            name: "ok".to_string(),
            member_type: MemberType::Primitive("String".to_string()), // Has data
            required: true,
            is_nullable: false,
            schema: json!({"type": "string"}).as_object().unwrap().clone(),
        });
        discovery.register_type(data_enum.clone());

        let analyzer = TypeAnalyzer::new(discovery, config);

        // Should NOT optimize when enum has data variants (even with optimization enabled)
        assert!(!analyzer.should_enum_optimize_to_primitive(&data_enum));
    }
}
