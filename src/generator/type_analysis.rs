//! Type Analysis System for Multi-Pass Compilation
//!
//! This module provides pure dependency and trait analysis data structures that process
//! SchemaDiscovery results without generating any code. This is the second pass in our
//! multi-pass compilation system:
//!
//! SchemaDiscovery → **TypeAnalysis** → CodeGeneration
//!
//! The TypeAnalysis contains:
//! - Complete dependency graphs with cycle detection
//! - Mathematical trait derivability analysis for each type
//! - Types requiring Box wrapping (from cycle detection)
//! - Type generation order (topological sort)
//! - Universal decode strategies for schema-aware decoding

use rustc_hash::FxHashMap;
use std::collections::HashSet;

/// Pure type analysis results - NO code generation
#[derive(Debug, Clone)]
pub struct TypeAnalysis {
    /// Complete dependency graph with cycle detection
    pub dependency_graph: TypeDependencyGraph,
    /// Trait derivability analysis for each type
    pub trait_derivability: FxHashMap<String, TraitDerivability>,
    /// Types requiring Box wrapping (from cycle detection)
    pub boxed_types: HashSet<String>,
    /// Type generation order (topological sort)
    pub generation_order: Vec<String>,
    /// Universal decode strategies for each type (schema-aware)
    pub decode_strategies: FxHashMap<String, DecodeStrategy>,
    /// Analysis statistics
    pub stats: AnalysisStats,
}

/// Dependency graph for types with cycle detection
#[derive(Debug, Clone)]
pub struct TypeDependencyGraph {
    /// Direct dependencies for each type
    pub dependencies: FxHashMap<String, HashSet<String>>,
    /// Reverse dependencies (dependents) for each type
    pub dependents: FxHashMap<String, HashSet<String>>,
    /// Strongly connected components (cycles)
    pub cycles: Vec<Vec<String>>,
    /// Types in dependency order (topological sort)
    pub ordered_types: Vec<String>,
}

/// Mathematical analysis of trait derivability for a type
#[derive(Debug, Clone)]
pub struct TraitDerivability {
    /// Can this type derive Eq?
    pub can_derive_eq: bool,
    /// Can this type derive Ord?
    pub can_derive_ord: bool,
    /// Can this type derive Hash?
    pub can_derive_hash: bool,
    /// Mathematical reasoning for the derivability decisions
    pub reasoning: String,
    /// Confidence level (0.0-1.0) - 1.0 for algorithmic decisions
    pub confidence: f64,
    /// Types that prevented derivation (empty if derivable)
    pub blocking_types: HashSet<String>,
}

/// Universal decode strategy for a specific type
///
/// Contains all information needed to generate schema-aware decode logic,
/// eliminating the need for heuristics, specificity calculations, and blind trying.
#[derive(Debug, Clone)]
pub struct DecodeStrategy {
    /// For structs: emitted member order for schema-local token streams.
    ///
    /// This is not a free decode-reordering hint. Struct encode and full decode
    /// must consume fields in the same order unless a future compatibility
    /// contract updates both sides together.
    pub field_order: Vec<String>,

    /// For arrays: expected item structure and count hints
    pub item_strategy: Option<Box<DecodeStrategy>>,

    /// For primitives: validation constraints
    pub constraints: DecodeConstraints,
}

/// Decode constraints for primitive types and validation
#[derive(Debug, Clone, Default, PartialEq)]
pub struct DecodeConstraints {
    /// Minimum value for numeric types
    pub min_value: Option<i64>,
    /// Maximum value for numeric types
    pub max_value: Option<i64>,
    /// Pattern validation for strings
    pub pattern: Option<String>,
    /// Required boolean value
    pub required_bool: Option<bool>,
}

impl Default for DecodeStrategy {
    fn default() -> Self {
        Self::new()
    }
}

impl DecodeStrategy {
    /// Create a new decode strategy
    pub fn new() -> Self {
        Self {
            field_order: Vec::new(),
            item_strategy: None,
            constraints: DecodeConstraints::default(),
        }
    }

    /// Create a primitive decode strategy
    pub fn primitive() -> Self {
        Self::new()
    }

    /// Create a string decode strategy
    pub fn string() -> Self {
        Self::new()
    }

    /// Create an array decode strategy
    pub fn array() -> Self {
        Self::new()
    }

    /// Create an object decode strategy
    pub fn object() -> Self {
        Self::new()
    }

    /// Record emitted field order for struct types
    pub fn with_field_order(mut self, fields: Vec<String>) -> Self {
        self.field_order = fields;
        self
    }
}

/// Analysis statistics (no code generation stats)
#[derive(Debug, Clone, Default)]
pub struct AnalysisStats {
    /// Total types analyzed
    pub total_types: usize,
    /// Types that can derive each trait
    pub eq_derivable: usize,
    /// Number of types that can derive Ord
    pub ord_derivable: usize,
    /// Number of types that can derive Hash
    pub hash_derivable: usize,
    /// Dependency analysis
    pub total_dependencies: usize,
    /// Number of circular dependency cycles detected
    pub cycles_detected: usize,
    /// Maximum depth of type dependency chain
    pub max_dependency_depth: usize,
    /// Boxing requirements
    pub types_requiring_boxing: usize,
    /// Analysis confidence
    pub algorithmic_decisions: usize,
    /// Number of decisions made using heuristics
    pub heuristic_decisions: usize,
}

impl Default for TypeAnalysis {
    fn default() -> Self {
        Self::new()
    }
}

impl TypeAnalysis {
    /// Create a new type analysis
    pub fn new() -> Self {
        Self {
            dependency_graph: TypeDependencyGraph::new(),
            trait_derivability: FxHashMap::default(),
            boxed_types: HashSet::new(),
            generation_order: Vec::new(),
            decode_strategies: FxHashMap::default(),
            stats: AnalysisStats::default(),
        }
    }

    /// Check if a type can derive a specific trait
    pub fn can_derive_trait(&self, type_name: &str, trait_name: &str) -> bool {
        if let Some(derivability) = self.trait_derivability.get(type_name) {
            match trait_name {
                "Eq" => derivability.can_derive_eq,
                "Ord" => derivability.can_derive_ord,
                "Hash" => derivability.can_derive_hash,
                _ => false,
            }
        } else {
            false
        }
    }

    /// Get trait derivability for a type
    pub fn get_trait_derivability(&self, type_name: &str) -> Option<&TraitDerivability> {
        self.trait_derivability.get(type_name)
    }

    /// Set trait derivability for a type
    pub fn set_trait_derivability(&mut self, type_name: String, derivability: TraitDerivability) {
        self.trait_derivability.insert(type_name, derivability);
    }

    /// Check if a type needs Box wrapping (for cycle breaking)
    pub fn needs_boxing(&self, type_name: &str) -> bool {
        self.boxed_types.contains(type_name)
    }

    /// Check if analysis is reliable (all decisions algorithmic)
    pub fn is_reliable(&self) -> bool {
        self.stats.total_types > 0
            && self.stats.algorithmic_decisions > 0
            && self.stats.heuristic_decisions == 0
    }
}

impl Default for TypeDependencyGraph {
    fn default() -> Self {
        Self::new()
    }
}

impl TypeDependencyGraph {
    /// Create a new dependency graph
    pub fn new() -> Self {
        Self {
            dependencies: FxHashMap::default(),
            dependents: FxHashMap::default(),
            cycles: Vec::new(),
            ordered_types: Vec::new(),
        }
    }

    /// Add a dependency relationship
    pub fn add_dependency(&mut self, dependent: String, dependency: String) {
        // Add to dependencies map
        self.dependencies
            .entry(dependent.clone())
            .or_default()
            .insert(dependency.clone());

        // Add to reverse dependencies map
        self.dependents
            .entry(dependency)
            .or_default()
            .insert(dependent);
    }

    /// Get direct dependencies of a type
    pub fn get_dependencies(&self, type_name: &str) -> Option<&HashSet<String>> {
        self.dependencies.get(type_name)
    }

    /// Get types that depend on this type
    pub fn get_dependents(&self, type_name: &str) -> Option<&HashSet<String>> {
        self.dependents.get(type_name)
    }

    /// Check if there are any dependency cycles
    pub fn has_cycles(&self) -> bool {
        !self.cycles.is_empty()
    }
}

impl TraitDerivability {
    /// Create a trait derivability with algorithmic confidence (1.0)
    pub fn algorithmic(
        can_derive_eq: bool,
        can_derive_ord: bool,
        can_derive_hash: bool,
        reasoning: String,
    ) -> Self {
        Self {
            can_derive_eq,
            can_derive_ord,
            can_derive_hash,
            reasoning,
            confidence: 1.0,
            blocking_types: HashSet::new(),
        }
    }

    /// Create a trait derivability with custom confidence level
    pub fn heuristic(
        can_derive_eq: bool,
        can_derive_ord: bool,
        can_derive_hash: bool,
        reasoning: String,
        confidence: f64,
    ) -> Self {
        Self {
            can_derive_eq,
            can_derive_ord,
            can_derive_hash,
            reasoning,
            confidence,
            blocking_types: HashSet::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_decode_strategy_creation() {
        // Test primitive strategy
        let prim_strategy = DecodeStrategy::primitive();
        assert!(prim_strategy.field_order.is_empty());
        assert!(prim_strategy.item_strategy.is_none());
        assert_eq!(prim_strategy.constraints, DecodeConstraints::default());

        // Test string strategy
        let string_strategy = DecodeStrategy::string();
        assert!(string_strategy.field_order.is_empty());
        assert!(string_strategy.item_strategy.is_none());

        // Test array strategy
        let array_strategy = DecodeStrategy::array();
        assert!(array_strategy.field_order.is_empty());
        assert!(array_strategy.item_strategy.is_none());

        // Test object strategy
        let object_strategy = DecodeStrategy::object();
        assert!(object_strategy.field_order.is_empty());
        assert!(object_strategy.item_strategy.is_none());
    }

    #[test]
    fn test_decode_strategy_builder_pattern() {
        // Test field order builder pattern
        let fields = vec![
            "field1".to_string(),
            "field2".to_string(),
            "field3".to_string(),
        ];
        let struct_strategy = DecodeStrategy::object().with_field_order(fields.clone());

        assert_eq!(struct_strategy.field_order, fields);
        assert!(struct_strategy.item_strategy.is_none());
        assert_eq!(struct_strategy.constraints, DecodeConstraints::default());

        // Test new() constructor
        let basic_strategy = DecodeStrategy::new();
        assert!(basic_strategy.field_order.is_empty());
        assert!(basic_strategy.item_strategy.is_none());
        assert_eq!(basic_strategy.constraints, DecodeConstraints::default());
    }

    #[test]
    fn test_type_analysis_with_decode_strategies() {
        let mut analysis = TypeAnalysis::new();

        // Add a decode strategy
        let strategy = DecodeStrategy::string();
        analysis
            .decode_strategies
            .insert("MyString".to_string(), strategy.clone());

        // Verify it's stored correctly
        assert_eq!(analysis.decode_strategies.len(), 1);
        assert!(analysis.decode_strategies.contains_key("MyString"));

        let retrieved = analysis.decode_strategies.get("MyString").unwrap();
        assert!(retrieved.field_order.is_empty());
        assert!(retrieved.item_strategy.is_none());
        assert_eq!(retrieved.constraints, DecodeConstraints::default());
    }

    #[test]
    fn test_decode_constraints() {
        let mut constraints = DecodeConstraints::default();
        assert!(constraints.min_value.is_none());
        assert!(constraints.max_value.is_none());
        assert!(constraints.pattern.is_none());
        assert!(constraints.required_bool.is_none());

        // Test setting constraints
        constraints.min_value = Some(10);
        constraints.max_value = Some(100);
        constraints.pattern = Some("^[a-z]+$".to_string());
        constraints.required_bool = Some(true);

        assert_eq!(constraints.min_value, Some(10));
        assert_eq!(constraints.max_value, Some(100));
        assert_eq!(constraints.pattern, Some("^[a-z]+$".to_string()));
        assert_eq!(constraints.required_bool, Some(true));
    }

    // === Phase 4.4: TypeAnalysis Coverage Tests ===
    // Batch 1: TypeAnalysis Core Methods

    #[test]
    fn test_type_analysis_can_derive_trait() {
        // Test can_derive_trait() for Eq/Ord/Hash lookups
        let mut analysis = TypeAnalysis::new();

        // Add a type with full trait derivability
        let full_derivability = TraitDerivability {
            can_derive_eq: true,
            can_derive_ord: true,
            can_derive_hash: true,
            reasoning: "All traits derivable".to_string(),
            confidence: 1.0,
            blocking_types: HashSet::new(),
        };
        analysis.set_trait_derivability("FullyDerivable".to_string(), full_derivability);

        // Add a type with partial trait derivability
        let partial_derivability = TraitDerivability {
            can_derive_eq: true,
            can_derive_ord: false,
            can_derive_hash: true,
            reasoning: "Ord blocked by f64 field".to_string(),
            confidence: 1.0,
            blocking_types: HashSet::new(),
        };
        analysis.set_trait_derivability("PartiallyDerivable".to_string(), partial_derivability);

        // Test full derivability
        assert!(analysis.can_derive_trait("FullyDerivable", "Eq"));
        assert!(analysis.can_derive_trait("FullyDerivable", "Ord"));
        assert!(analysis.can_derive_trait("FullyDerivable", "Hash"));
        assert!(!analysis.can_derive_trait("FullyDerivable", "Unknown"));

        // Test partial derivability
        assert!(analysis.can_derive_trait("PartiallyDerivable", "Eq"));
        assert!(!analysis.can_derive_trait("PartiallyDerivable", "Ord"));
        assert!(analysis.can_derive_trait("PartiallyDerivable", "Hash"));

        // Test non-existent type returns false
        assert!(!analysis.can_derive_trait("NonExistent", "Eq"));
        assert!(!analysis.can_derive_trait("NonExistent", "Ord"));
        assert!(!analysis.can_derive_trait("NonExistent", "Hash"));
    }

    #[test]
    fn test_type_analysis_trait_derivability_storage() {
        // Test set_trait_derivability() and get_trait_derivability()
        let mut analysis = TypeAnalysis::new();

        // Create trait derivability with blocking types
        let mut blocking_types = HashSet::new();
        blocking_types.insert("HashMap".to_string());
        blocking_types.insert("BTreeMap".to_string());

        let derivability = TraitDerivability {
            can_derive_eq: true,
            can_derive_ord: false,
            can_derive_hash: false,
            reasoning: "HashMap and BTreeMap don't implement Hash".to_string(),
            confidence: 1.0,
            blocking_types: blocking_types.clone(),
        };

        // Store derivability
        analysis.set_trait_derivability("ComplexType".to_string(), derivability);

        // Retrieve and verify
        let retrieved = analysis.get_trait_derivability("ComplexType");
        assert!(retrieved.is_some());

        let retrieved = retrieved.unwrap();
        assert!(retrieved.can_derive_eq);
        assert!(!retrieved.can_derive_ord);
        assert!(!retrieved.can_derive_hash);
        assert_eq!(
            retrieved.reasoning,
            "HashMap and BTreeMap don't implement Hash"
        );
        assert_eq!(retrieved.confidence, 1.0);
        assert_eq!(retrieved.blocking_types.len(), 2);
        assert!(retrieved.blocking_types.contains("HashMap"));
        assert!(retrieved.blocking_types.contains("BTreeMap"));

        // Test non-existent type
        assert!(analysis.get_trait_derivability("NonExistent").is_none());

        // Test multiple types
        let another_derivability = TraitDerivability {
            can_derive_eq: true,
            can_derive_ord: true,
            can_derive_hash: true,
            reasoning: "Simple struct".to_string(),
            confidence: 1.0,
            blocking_types: HashSet::new(),
        };
        analysis.set_trait_derivability("SimpleType".to_string(), another_derivability);

        assert_eq!(analysis.trait_derivability.len(), 2);
        assert!(analysis.get_trait_derivability("ComplexType").is_some());
        assert!(analysis.get_trait_derivability("SimpleType").is_some());
    }

    #[test]
    fn test_type_analysis_boxing_and_reliability() {
        // Test needs_boxing() for cycle breaking
        let mut analysis = TypeAnalysis::new();

        // Initially no types need boxing
        assert!(!analysis.needs_boxing("Node"));
        assert!(!analysis.needs_boxing("Tree"));

        // Add types requiring boxing
        analysis.boxed_types.insert("Node".to_string());
        analysis.boxed_types.insert("Tree".to_string());

        // Verify boxing checks
        assert!(analysis.needs_boxing("Node"));
        assert!(analysis.needs_boxing("Tree"));
        assert!(!analysis.needs_boxing("Leaf"));

        // Test is_reliable() for algorithmic vs heuristic decisions
        let mut reliable_analysis = TypeAnalysis::new();
        reliable_analysis.stats.total_types = 10;
        reliable_analysis.stats.algorithmic_decisions = 10;
        reliable_analysis.stats.heuristic_decisions = 0;

        assert!(reliable_analysis.is_reliable());

        // Test unreliable analysis (has heuristic decisions)
        let mut unreliable_analysis = TypeAnalysis::new();
        unreliable_analysis.stats.total_types = 10;
        unreliable_analysis.stats.algorithmic_decisions = 7;
        unreliable_analysis.stats.heuristic_decisions = 3;

        assert!(!unreliable_analysis.is_reliable());

        // Test edge case: no types analyzed
        let empty_analysis = TypeAnalysis::new();
        assert!(!empty_analysis.is_reliable());

        // Test edge case: types but no algorithmic decisions
        let mut no_algo_analysis = TypeAnalysis::new();
        no_algo_analysis.stats.total_types = 5;
        no_algo_analysis.stats.algorithmic_decisions = 0;
        no_algo_analysis.stats.heuristic_decisions = 5;

        assert!(!no_algo_analysis.is_reliable());
    }

    // Batch 2: TypeDependencyGraph Methods

    #[test]
    fn test_dependency_graph_add_dependency() {
        // Test add_dependency() creates forward and reverse links
        let mut graph = TypeDependencyGraph::new();

        // Initially empty
        assert!(graph.dependencies.is_empty());
        assert!(graph.dependents.is_empty());

        // Add single dependency: Struct1 depends on Struct2
        graph.add_dependency("Struct1".to_string(), "Struct2".to_string());

        // Verify forward dependency
        let deps = graph.get_dependencies("Struct1");
        assert!(deps.is_some());
        assert_eq!(deps.unwrap().len(), 1);
        assert!(deps.unwrap().contains("Struct2"));

        // Verify reverse dependency
        let dependents = graph.get_dependents("Struct2");
        assert!(dependents.is_some());
        assert_eq!(dependents.unwrap().len(), 1);
        assert!(dependents.unwrap().contains("Struct1"));

        // Add multiple dependencies for same type
        graph.add_dependency("Struct1".to_string(), "Struct3".to_string());
        graph.add_dependency("Struct1".to_string(), "Struct4".to_string());

        let deps = graph.get_dependencies("Struct1");
        assert_eq!(deps.unwrap().len(), 3);
        assert!(deps.unwrap().contains("Struct2"));
        assert!(deps.unwrap().contains("Struct3"));
        assert!(deps.unwrap().contains("Struct4"));

        // Add multiple dependents for same type
        graph.add_dependency("Struct5".to_string(), "Struct2".to_string());

        let dependents = graph.get_dependents("Struct2");
        assert_eq!(dependents.unwrap().len(), 2);
        assert!(dependents.unwrap().contains("Struct1"));
        assert!(dependents.unwrap().contains("Struct5"));
    }

    #[test]
    fn test_dependency_graph_get_methods() {
        // Test get_dependencies() and get_dependents() retrieval
        let mut graph = TypeDependencyGraph::new();

        // Build dependency graph:
        // Node -> Parent, Left, Right
        // Parent -> GrandParent
        // Left -> Value
        // Right -> Value
        graph.add_dependency("Node".to_string(), "Parent".to_string());
        graph.add_dependency("Node".to_string(), "Left".to_string());
        graph.add_dependency("Node".to_string(), "Right".to_string());
        graph.add_dependency("Parent".to_string(), "GrandParent".to_string());
        graph.add_dependency("Left".to_string(), "Value".to_string());
        graph.add_dependency("Right".to_string(), "Value".to_string());

        // Test get_dependencies()
        let node_deps = graph.get_dependencies("Node");
        assert!(node_deps.is_some());
        assert_eq!(node_deps.unwrap().len(), 3);
        assert!(node_deps.unwrap().contains("Parent"));
        assert!(node_deps.unwrap().contains("Left"));
        assert!(node_deps.unwrap().contains("Right"));

        let parent_deps = graph.get_dependencies("Parent");
        assert!(parent_deps.is_some());
        assert_eq!(parent_deps.unwrap().len(), 1);
        assert!(parent_deps.unwrap().contains("GrandParent"));

        // Test get_dependents()
        let value_dependents = graph.get_dependents("Value");
        assert!(value_dependents.is_some());
        assert_eq!(value_dependents.unwrap().len(), 2);
        assert!(value_dependents.unwrap().contains("Left"));
        assert!(value_dependents.unwrap().contains("Right"));

        let parent_dependents = graph.get_dependents("Parent");
        assert!(parent_dependents.is_some());
        assert_eq!(parent_dependents.unwrap().len(), 1);
        assert!(parent_dependents.unwrap().contains("Node"));

        // Test non-existent types return None
        assert!(graph.get_dependencies("NonExistent").is_none());
        assert!(graph.get_dependents("NonExistent").is_none());

        // Leaf nodes have no dependencies
        assert!(graph.get_dependencies("GrandParent").is_none());
        assert!(graph.get_dependencies("Value").is_none());
    }

    #[test]
    fn test_dependency_graph_cycle_detection() {
        // Test has_cycles() returns false when no cycles
        let mut no_cycle_graph = TypeDependencyGraph::new();
        no_cycle_graph.add_dependency("A".to_string(), "B".to_string());
        no_cycle_graph.add_dependency("B".to_string(), "C".to_string());
        no_cycle_graph.add_dependency("C".to_string(), "D".to_string());

        // Initially no cycles detected (cycles vector empty)
        assert!(!no_cycle_graph.has_cycles());
        assert_eq!(no_cycle_graph.cycles.len(), 0);

        // Test has_cycles() returns true when cycles exist
        let mut cycle_graph = TypeDependencyGraph::new();

        // Create a cycle: Node1 -> Node2 -> Node3 -> Node1
        cycle_graph.add_dependency("Node1".to_string(), "Node2".to_string());
        cycle_graph.add_dependency("Node2".to_string(), "Node3".to_string());
        cycle_graph.add_dependency("Node3".to_string(), "Node1".to_string());

        // Manually add cycle for testing (normally detected by analysis pass)
        cycle_graph.cycles.push(vec![
            "Node1".to_string(),
            "Node2".to_string(),
            "Node3".to_string(),
        ]);

        assert!(cycle_graph.has_cycles());
        assert_eq!(cycle_graph.cycles.len(), 1);
        assert_eq!(cycle_graph.cycles[0].len(), 3);

        // Test multiple cycles
        let mut multi_cycle_graph = TypeDependencyGraph::new();
        multi_cycle_graph
            .cycles
            .push(vec!["A".to_string(), "B".to_string()]);
        multi_cycle_graph
            .cycles
            .push(vec!["X".to_string(), "Y".to_string(), "Z".to_string()]);

        assert!(multi_cycle_graph.has_cycles());
        assert_eq!(multi_cycle_graph.cycles.len(), 2);
    }

    // Batch 3: TraitDerivability Factory Methods

    #[test]
    fn test_trait_derivability_algorithmic_factory() {
        // Test TraitDerivability::algorithmic() creates with confidence=1.0
        let derivability = TraitDerivability::algorithmic(
            true,
            true,
            true,
            "All traits derivable from type analysis".to_string(),
        );

        // Verify all traits enabled
        assert!(derivability.can_derive_eq);
        assert!(derivability.can_derive_ord);
        assert!(derivability.can_derive_hash);

        // Verify algorithmic confidence
        assert_eq!(derivability.confidence, 1.0);

        // Verify reasoning stored
        assert_eq!(
            derivability.reasoning,
            "All traits derivable from type analysis"
        );

        // Verify empty blocking types (algorithmic factory doesn't set them)
        assert!(derivability.blocking_types.is_empty());

        // Test partial trait derivability
        let partial = TraitDerivability::algorithmic(
            true,
            false,
            false,
            "Ord/Hash blocked by f64 field".to_string(),
        );

        assert!(partial.can_derive_eq);
        assert!(!partial.can_derive_ord);
        assert!(!partial.can_derive_hash);
        assert_eq!(partial.confidence, 1.0);
        assert_eq!(partial.reasoning, "Ord/Hash blocked by f64 field");

        // Test no traits derivable
        let none = TraitDerivability::algorithmic(
            false,
            false,
            false,
            "Type contains serde_json::Value".to_string(),
        );

        assert!(!none.can_derive_eq);
        assert!(!none.can_derive_ord);
        assert!(!none.can_derive_hash);
        assert_eq!(none.confidence, 1.0);
    }

    #[test]
    fn test_trait_derivability_heuristic_factory() {
        // Test TraitDerivability::heuristic() with custom confidence
        let high_confidence = TraitDerivability::heuristic(
            true,
            true,
            false,
            "Likely derivable based on pattern".to_string(),
            0.9,
        );

        // Verify traits
        assert!(high_confidence.can_derive_eq);
        assert!(high_confidence.can_derive_ord);
        assert!(!high_confidence.can_derive_hash);

        // Verify custom confidence
        assert_eq!(high_confidence.confidence, 0.9);

        // Verify reasoning
        assert_eq!(
            high_confidence.reasoning,
            "Likely derivable based on pattern"
        );

        // Verify empty blocking types (heuristic factory doesn't set them)
        assert!(high_confidence.blocking_types.is_empty());

        // Test medium confidence
        let medium_confidence = TraitDerivability::heuristic(
            true,
            false,
            false,
            "Uncertain about Ord derivability".to_string(),
            0.7,
        );

        assert_eq!(medium_confidence.confidence, 0.7);
        assert!(medium_confidence.can_derive_eq);
        assert!(!medium_confidence.can_derive_ord);

        // Test low confidence
        let low_confidence = TraitDerivability::heuristic(
            false,
            false,
            false,
            "Complex type, conservative approach".to_string(),
            0.5,
        );

        assert_eq!(low_confidence.confidence, 0.5);
        assert!(!low_confidence.can_derive_eq);
        assert!(!low_confidence.can_derive_ord);
        assert!(!low_confidence.can_derive_hash);

        // Test different confidence levels
        let conf_0_3 =
            TraitDerivability::heuristic(true, true, true, "Low confidence guess".to_string(), 0.3);
        assert_eq!(conf_0_3.confidence, 0.3);

        let conf_1_0 = TraitDerivability::heuristic(
            true,
            true,
            true,
            "Heuristic but very confident".to_string(),
            1.0,
        );
        assert_eq!(conf_1_0.confidence, 1.0);
    }
}
