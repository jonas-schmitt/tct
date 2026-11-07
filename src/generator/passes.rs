//! Extensible multi-pass compilation system
//!
//! This module provides a generic Pass trait system that supports:
//! - DiscoveryPass: walks schema, discovers types (no code generation)
//! - AnalysisPass: builds dependency graphs, runs trait analysis
//! - GenerationPass: generates final code from discovered types
//! - Future passes: validation, custom analysis, etc.

use crate::TctResult;
use crate::generator::code_generator::{CodeGenerator, CodeGeneratorOptions};
use crate::generator::encoding_decision::EncodingStrategy;
use crate::generator::generation_output::CodeGeneration;
use crate::generator::schema_discovery::SchemaDiscovery;
use crate::generator::schema_walker::SchemaWalker;
use crate::generator::type_analysis::TypeAnalysis;
use crate::generator::type_analyzer::TypeAnalyzer;
use crate::grammar::Definitions;
use crate::optimization_pass::OptimizationConfig;
use rustc_hash::FxHashMap;
use serde_json::Value;
use std::collections::HashSet;
use std::time::Duration;
use std::time::Instant;

/// Context passed between compilation passes to share state
pub struct PassContext {
    /// Generated code definitions (populated by GenerationPass)
    pub definitions: Definitions,
    /// Generated type names to prevent collisions
    pub generated_type_names: std::collections::HashSet<String>,
    /// Reference type mapping for resolving $refs
    pub ref_type_map: FxHashMap<String, String>,
    /// Root schema name for resolving "#" references
    pub root_schema_name: Option<String>,
    /// Types requiring Box wrapping due to recursion
    pub recursive_types: std::collections::HashSet<String>,
    /// Generated code strings (for verification)
    pub generated_code: Vec<String>,
    /// Analysis results from AnalysisPass
    pub analysis_results: Option<TypeAnalysis>,
    /// Schema discovery results from DiscoveryPass
    pub schema_discovery: Option<SchemaDiscovery>,
    /// Code generation results from GenerationPass
    pub code_generation: Option<CodeGeneration>,
    /// Optimization configuration for all passes
    pub optimization_config: Option<OptimizationConfig>,
    /// Encoding strategy from the optimization phase (Phase 3)
    pub encoding_strategy: Option<EncodingStrategy>,
    /// Measured elapsed time for each completed pass.
    pub pass_timings: FxHashMap<String, Duration>,
}

impl Default for PassContext {
    fn default() -> Self {
        Self::new()
    }
}

impl PassContext {
    /// Creates a new empty pass context
    pub fn new() -> Self {
        Self {
            definitions: Definitions {
                structs: FxHashMap::default(),
                enums: Vec::new(),
                constants: Vec::new(),
                type_aliases: Vec::new(),
            },
            generated_type_names: HashSet::new(),
            ref_type_map: FxHashMap::default(),
            root_schema_name: None,
            recursive_types: HashSet::new(),
            generated_code: Vec::new(),
            analysis_results: None,
            schema_discovery: None,
            code_generation: None,
            optimization_config: None,
            encoding_strategy: None,
            pass_timings: FxHashMap::default(),
        }
    }
}

/// Result of a compilation pass
#[derive(Debug)]
pub enum PassResult {
    /// Discovery pass result with schema discovery data
    Discovery(SchemaDiscovery),
    /// Analysis pass result with type analysis data
    Analysis(TypeAnalysis),
    /// Optimization pass result with encoding decisions
    Optimization(EncodingStrategy),
    /// Generation pass result with final code generation
    Generation(CodeGeneration),
    /// Custom pass result (for future extensions)
    Custom(FxHashMap<String, serde_json::Value>),
}

/// Generic trait for compilation passes
pub trait CompilationPass {
    /// Name of this pass for logging and debugging
    fn name(&self) -> &'static str;

    /// Run this pass on the schema with the given context
    fn run(&mut self, schema: &Value, context: &mut PassContext) -> TctResult<PassResult>;

    /// Dependencies: which passes must run before this one
    fn dependencies(&self) -> Vec<&'static str> {
        Vec::new()
    }

    /// Whether this pass modifies the schema (most don't)
    fn modifies_schema(&self) -> bool {
        false
    }
}

/// Discovery pass - walks schema and populates TypeRegistry without generating code
pub struct DiscoveryPass {
    /// Root type name for this compilation
    root_type_name: String,
}

impl Default for DiscoveryPass {
    fn default() -> Self {
        Self::new()
    }
}

impl DiscoveryPass {
    /// Creates a new discovery pass with default root type name "Root"
    pub fn new() -> Self {
        Self {
            root_type_name: "Root".to_string(),
        }
    }

    /// Creates a new discovery pass with a custom root type name
    pub fn with_root_name(root_name: String) -> Self {
        Self {
            root_type_name: root_name,
        }
    }
}

impl CompilationPass for DiscoveryPass {
    fn name(&self) -> &'static str {
        "Discovery"
    }

    fn run(&mut self, schema: &Value, context: &mut PassContext) -> TctResult<PassResult> {
        // Use SchemaWalker for pure discovery - NO code generation
        let max_schema_depth = context
            .optimization_config
            .as_ref()
            .map(crate::optimization_pass::OptimizationConfig::get_max_schema_depth)
            .unwrap_or(crate::schema_utils::DEFAULT_MAX_SCHEMA_TRAVERSAL_DEPTH);
        let discovery =
            SchemaWalker::walk_with_max_depth(schema, &self.root_type_name, max_schema_depth)?;

        // Store discovery results in context for later passes
        context.schema_discovery = Some(discovery.clone());

        // Also set root schema name in context for reference resolution
        if let Some(ref root_info) = discovery.root_info {
            context.root_schema_name = Some(root_info.type_name.clone());
        }

        // Store ref mappings for later passes
        for (json_path, type_name) in &discovery.ref_mappings {
            context
                .ref_type_map
                .insert(json_path.clone(), type_name.clone());
        }

        Ok(PassResult::Discovery(discovery))
    }
}

/// Analysis pass - builds dependency graphs and runs trait analysis
pub struct AnalysisPass;

impl Default for AnalysisPass {
    fn default() -> Self {
        Self::new()
    }
}

impl AnalysisPass {
    /// Creates a new analysis pass
    pub fn new() -> Self {
        Self
    }
}

impl CompilationPass for AnalysisPass {
    fn name(&self) -> &'static str {
        "Analysis"
    }

    fn dependencies(&self) -> Vec<&'static str> {
        vec!["Discovery"]
    }

    fn run(&mut self, _schema: &Value, context: &mut PassContext) -> TctResult<PassResult> {
        // Get schema discovery results from previous pass
        let discovery = context.schema_discovery.as_ref().ok_or_else(|| {
            crate::error::TctError::CodeGeneration {
                message: "AnalysisPass requires SchemaDiscovery from DiscoveryPass".to_string(),
                schema_path: "#/".to_string(),
                suggestion: "Make sure DiscoveryPass runs before AnalysisPass".to_string(),
                phase: crate::error::GenerationPhase::SchemaAnalysis,
            }
        })?;

        // Get optimization config from context, default to O0 if not provided
        let optimization_config = context.optimization_config.clone().unwrap_or_else(|| {
            crate::log_debug!("⚠️ No optimization config in context, defaulting to O0");
            crate::optimization_pass::OptimizationConfig::o0()
        });

        // Use TypeAnalyzer for pure analysis - NO code generation
        let analysis = TypeAnalyzer::analyze(discovery.clone(), optimization_config)?;

        // Store analysis results in context for later passes
        context.analysis_results = Some(analysis.clone());

        // Update context with boxing information for legacy compatibility
        context.recursive_types = analysis.boxed_types.clone();

        Ok(PassResult::Analysis(analysis))
    }
}

/// Generation pass - generates final code from TypeRegistry and analysis results
pub struct GenerationPass;

impl Default for GenerationPass {
    fn default() -> Self {
        Self::new()
    }
}

impl GenerationPass {
    /// Creates a new code generation pass
    pub fn new() -> Self {
        Self
    }
}

impl CompilationPass for GenerationPass {
    fn name(&self) -> &'static str {
        "Generation"
    }

    fn dependencies(&self) -> Vec<&'static str> {
        vec!["Discovery", "Analysis", "Optimization"]
    }

    fn run(&mut self, _schema: &Value, context: &mut PassContext) -> TctResult<PassResult> {
        // Get schema discovery results from DiscoveryPass
        let discovery = context.schema_discovery.as_ref().ok_or_else(|| {
            crate::error::TctError::CodeGeneration {
                message: "GenerationPass requires SchemaDiscovery from DiscoveryPass".to_string(),
                schema_path: "#/".to_string(),
                suggestion: "Make sure DiscoveryPass runs before GenerationPass".to_string(),
                phase: crate::error::GenerationPhase::TypeGeneration,
            }
        })?;

        // Get analysis results from AnalysisPass
        let analysis = context.analysis_results.as_ref().ok_or_else(|| {
            crate::error::TctError::CodeGeneration {
                message: "GenerationPass requires TypeAnalysis from AnalysisPass".to_string(),
                schema_path: "#/".to_string(),
                suggestion: "Make sure AnalysisPass runs before GenerationPass".to_string(),
                phase: crate::error::GenerationPhase::TypeGeneration,
            }
        })?;

        // Get encoding strategy from the optimization phase (optional for backwards compatibility)
        let encoding_strategy = context.encoding_strategy.clone();

        // No max_vocab constraint for structural vocabulary
        let max_vocab = None;

        // Extract max_base_vocab from optimization config (for multi-token encoding base)
        let max_base_vocab = context
            .optimization_config
            .as_ref()
            .and_then(|config| config.max_base_vocab);

        // Extract compilation constraints that affect emitted token ranges.
        let compilation_constraints = context
            .optimization_config
            .as_ref()
            .and_then(|config| config.compilation_constraints.clone())
            .unwrap_or_default();

        // Extract streaming_encode flag (for prefix decoding support)
        let streaming_encode = context
            .optimization_config
            .as_ref()
            .map(|config| config.streaming_encode)
            .unwrap_or(false);

        let max_collection_length = context
            .optimization_config
            .as_ref()
            .map(|config| config.get_max_collection_length())
            .unwrap_or(2048);

        let decimal_number_encoding = context
            .optimization_config
            .as_ref()
            .map(|config| config.decimal_number_encoding)
            .unwrap_or(false);

        // Note: max_string_length removed - end-marker encoding eliminates length constraints

        // Use CodeGenerator for pure code generation
        let generation = CodeGenerator::generate(
            discovery.clone(),
            analysis.clone(),
            encoding_strategy,
            CodeGeneratorOptions {
                max_vocab,
                max_base_vocab,
                compilation_constraints,
                streaming_encode,
                max_collection_length,
                decimal_number_encoding,
            },
        )?;

        // Store generation results in context for final Grammar building
        context.code_generation = Some(generation.clone());

        Ok(PassResult::Generation(generation))
    }
}

/// Optimization pass wrapper - makes encoding decisions
pub struct OptimizationPassWrapper {
    pass: crate::generator::optimization_phase::OptimizationPhase,
}

impl OptimizationPassWrapper {
    /// Creates a new optimization pass with configuration
    pub fn new(config: OptimizationConfig) -> Self {
        Self {
            pass: crate::generator::optimization_phase::OptimizationPhase::new(config),
        }
    }
}

impl CompilationPass for OptimizationPassWrapper {
    fn name(&self) -> &'static str {
        "Optimization"
    }

    fn dependencies(&self) -> Vec<&'static str> {
        vec!["Discovery", "Analysis"]
    }

    fn run(&mut self, schema: &Value, context: &mut PassContext) -> TctResult<PassResult> {
        // Get discovery results
        let discovery = context.schema_discovery.as_ref().ok_or_else(|| {
            crate::error::TctError::CodeGeneration {
                message: "Optimization phase requires SchemaDiscovery from DiscoveryPass"
                    .to_string(),
                schema_path: "#/".to_string(),
                suggestion: "Make sure DiscoveryPass runs before the optimization phase"
                    .to_string(),
                phase: crate::error::GenerationPhase::TypeGeneration,
            }
        })?;

        // Get analysis results
        let analysis = context.analysis_results.as_ref().ok_or_else(|| {
            crate::error::TctError::CodeGeneration {
                message: "Optimization phase requires TypeAnalysis from AnalysisPass".to_string(),
                schema_path: "#/".to_string(),
                suggestion: "Make sure AnalysisPass runs before the optimization phase".to_string(),
                phase: crate::error::GenerationPhase::TypeGeneration,
            }
        })?;

        // Run optimization to get encoding strategy and generation-facing metadata.
        let encoding_strategy = self.pass.run(schema, discovery, analysis)?;

        // Store encoding strategy in context for GenerationPass
        context.encoding_strategy = Some(encoding_strategy.clone());

        Ok(PassResult::Optimization(encoding_strategy))
    }
}

/// Orchestrates multiple compilation passes
pub struct PassOrchestrator {
    passes: Vec<Box<dyn CompilationPass>>,
}

impl Default for PassOrchestrator {
    fn default() -> Self {
        Self::new()
    }
}

impl PassOrchestrator {
    /// Creates a new empty pass orchestrator
    pub fn new() -> Self {
        Self { passes: Vec::new() }
    }

    /// Adds a compilation pass to the pipeline
    pub fn add_pass<P: CompilationPass + 'static>(&mut self, pass: P) {
        self.passes.push(Box::new(pass));
    }

    /// Run all passes in dependency order
    pub fn run_all(&mut self, schema: &Value) -> TctResult<PassContext> {
        let mut context = PassContext::new();
        self.run_passes(schema, &mut context)?;
        Ok(context)
    }

    /// Run all passes with optimization configuration
    pub fn run_all_with_config(
        &mut self,
        schema: &Value,
        optimization_config: OptimizationConfig,
    ) -> TctResult<PassContext> {
        let mut context = PassContext::new();
        context.optimization_config = Some(optimization_config);
        self.run_passes(schema, &mut context)?;
        Ok(context)
    }

    /// Internal method to run passes with provided context
    fn run_passes(&mut self, schema: &Value, context: &mut PassContext) -> TctResult<()> {
        self.validate_pipeline()?;

        for pass in &mut self.passes {
            let pass_name = pass.name();

            crate::log_debug!("🔄 Running pass: {}", pass_name);

            let pass_start = Instant::now();
            let result = pass.run(schema, context)?;
            context
                .pass_timings
                .insert(pass_name.to_string(), pass_start.elapsed());

            match result {
                PassResult::Discovery(discovery) => {
                    crate::log_debug!(
                        "✅ Discovery pass completed: {} types discovered",
                        discovery.discovered_types.len()
                    );
                }
                PassResult::Analysis(analysis) => {
                    crate::log_debug!(
                        "✅ Analysis pass completed: {} types analyzed, {} cycles detected",
                        analysis.trait_derivability.len(),
                        analysis.dependency_graph.cycles.len()
                    );
                }
                PassResult::Optimization(encoding_strategy) => {
                    crate::log_debug!(
                        "✅ Optimization pass completed: {} optimized fields, {} tokens saved ({:.1}% reduction)",
                        encoding_strategy.stats.optimized_fields,
                        encoding_strategy.stats.tokens_saved,
                        encoding_strategy.stats.reduction_percentage
                    );
                }
                // PassResult::SemanticAnalysis deprecated
                // PassResult::SemanticAnalysis(semantic) => {
                //     crate::log_debug!("✅ Semantic analysis pass completed: {} signatures computed, {} enums analyzed",
                //         semantic.token_signatures.len(),
                //         semantic.enum_decode_strategies.len());
                // }
                PassResult::Generation(generation) => {
                    crate::log_debug!(
                        "✅ Generation pass completed: {} types generated",
                        generation.type_definitions.len()
                    );
                }
                PassResult::Custom(_) => {
                    crate::log_debug!("✅ Custom pass completed");
                }
            }
        }

        crate::log_debug!("🎯 All passes completed successfully");
        Ok(())
    }

    fn validate_pipeline(&self) -> TctResult<()> {
        let mut known_passes = HashSet::new();
        for pass in &self.passes {
            let pass_name = pass.name();
            if !known_passes.insert(pass_name) {
                return pipeline_validation_error(
                    format!("Pass pipeline contains duplicate pass name '{pass_name}'"),
                    format!(
                        "Use unique pass names so dependencies on '{pass_name}' are unambiguous"
                    ),
                );
            }
        }

        let mut completed_passes = HashSet::new();
        for pass in &self.passes {
            let pass_name = pass.name();
            for dependency in pass.dependencies() {
                if dependency == pass_name {
                    return pipeline_validation_error(
                        format!("Pass '{pass_name}' cannot depend on itself"),
                        format!("Remove '{dependency}' from '{pass_name}' dependencies"),
                    );
                }

                if !known_passes.contains(dependency) {
                    return pipeline_validation_error(
                        format!(
                            "Pass '{pass_name}' requires dependency '{dependency}', but that pass is not in the pipeline"
                        ),
                        format!(
                            "Add '{dependency}' before '{pass_name}', or remove the unused dependency"
                        ),
                    );
                }

                if !completed_passes.contains(dependency) {
                    return pipeline_validation_error(
                        format!(
                            "Pass '{pass_name}' requires dependency '{dependency}' to run first"
                        ),
                        format!(
                            "Move '{dependency}' before '{pass_name}', or use the standard Discovery -> Analysis -> Optimization -> Generation pipeline"
                        ),
                    );
                }
            }

            completed_passes.insert(pass_name);
        }

        Ok(())
    }
}

fn pipeline_validation_error<T>(message: String, suggestion: String) -> TctResult<T> {
    Err(crate::error::TctError::CodeGeneration {
        message,
        schema_path: "#/".to_string(),
        suggestion,
        phase: crate::error::GenerationPhase::OptimizationApplication,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    // ============================================================================
    // Batch 1 - PassContext Tests (4 tests)
    // ============================================================================

    #[test]
    fn test_pass_context_new() {
        let context = PassContext::new();
        assert!(context.definitions.structs.is_empty());
        assert!(context.definitions.enums.is_empty());
        assert!(context.generated_type_names.is_empty());
        assert!(context.ref_type_map.is_empty());
        assert!(context.root_schema_name.is_none());
        assert!(context.recursive_types.is_empty());
        assert!(context.generated_code.is_empty());
        assert!(context.analysis_results.is_none());
        assert!(context.schema_discovery.is_none());
        assert!(context.code_generation.is_none());
        assert!(context.optimization_config.is_none());
        assert!(context.encoding_strategy.is_none());
        assert!(context.pass_timings.is_empty());
    }

    #[test]
    fn test_pass_context_default() {
        let context1 = PassContext::default();
        let context2 = PassContext::new();

        assert_eq!(
            context1.definitions.structs.len(),
            context2.definitions.structs.len()
        );
        assert_eq!(
            context1.definitions.enums.len(),
            context2.definitions.enums.len()
        );
        assert_eq!(
            context1.generated_type_names.len(),
            context2.generated_type_names.len()
        );
    }

    #[test]
    fn test_pass_context_fields() {
        let mut context = PassContext::new();

        // Test root_schema_name field
        context.root_schema_name = Some("MyRoot".to_string());
        assert_eq!(context.root_schema_name, Some("MyRoot".to_string()));

        // Test generated_type_names field
        context.generated_type_names.insert("User".to_string());
        context.generated_type_names.insert("Product".to_string());
        assert_eq!(context.generated_type_names.len(), 2);
        assert!(context.generated_type_names.contains("User"));

        // Test recursive_types field
        context.recursive_types.insert("Node".to_string());
        assert!(context.recursive_types.contains("Node"));

        // Test generated_code field
        context.generated_code.push("struct Foo {}".to_string());
        assert_eq!(context.generated_code.len(), 1);
    }

    #[test]
    fn test_pass_context_ref_mappings() {
        let mut context = PassContext::new();

        // Add ref mappings
        context
            .ref_type_map
            .insert("#/definitions/User".to_string(), "User".to_string());
        context
            .ref_type_map
            .insert("#/definitions/Product".to_string(), "Product".to_string());

        assert_eq!(context.ref_type_map.len(), 2);
        assert_eq!(
            context.ref_type_map.get("#/definitions/User"),
            Some(&"User".to_string())
        );
        assert_eq!(
            context.ref_type_map.get("#/definitions/Product"),
            Some(&"Product".to_string())
        );
    }

    // ============================================================================
    // Batch 2 - Individual Pass Tests (8 tests)
    // ============================================================================

    #[test]
    fn test_discovery_pass_new() {
        let pass = DiscoveryPass::new();
        assert_eq!(pass.root_type_name, "Root");
        assert_eq!(pass.name(), "Discovery");
        assert!(pass.dependencies().is_empty());
        assert!(!pass.modifies_schema());
    }

    #[test]
    fn test_discovery_pass_default() {
        let pass1 = DiscoveryPass::default();
        let pass2 = DiscoveryPass::new();
        assert_eq!(pass1.root_type_name, pass2.root_type_name);
    }

    #[test]
    fn test_discovery_pass_with_root_name() {
        let pass = DiscoveryPass::with_root_name("CustomRoot".to_string());
        assert_eq!(pass.root_type_name, "CustomRoot");
        assert_eq!(pass.name(), "Discovery");
    }

    #[test]
    fn test_discovery_pass_execution() {
        let mut pass = DiscoveryPass::new();
        let mut context = PassContext::new();
        let schema = json!({
            "type": "object",
            "properties": {
                "name": {"type": "string"},
                "age": {"type": "integer"}
            }
        });

        let result = pass.run(&schema, &mut context);
        assert!(result.is_ok());

        // Verify context is populated
        assert!(context.schema_discovery.is_some());
        assert!(context.root_schema_name.is_some());

        // Verify PassResult::Discovery is returned
        match result.unwrap() {
            PassResult::Discovery(discovery) => {
                assert!(!discovery.discovered_types.is_empty());
            }
            _ => panic!("Expected PassResult::Discovery"),
        }
    }

    #[test]
    fn test_analysis_pass_new() {
        let pass = AnalysisPass::new();
        assert_eq!(pass.name(), "Analysis");
        assert_eq!(pass.dependencies(), vec!["Discovery"]);
        assert!(!pass.modifies_schema());
    }

    #[test]
    fn test_analysis_pass_default() {
        let _pass = AnalysisPass;
        assert_eq!(AnalysisPass::new().name(), "Analysis");
    }

    #[test]
    fn test_generation_pass_new() {
        let pass = GenerationPass::new();
        assert_eq!(pass.name(), "Generation");
        assert_eq!(
            pass.dependencies(),
            vec!["Discovery", "Analysis", "Optimization"]
        );
        assert!(!pass.modifies_schema());
    }

    #[test]
    fn test_generation_pass_default() {
        let _pass = GenerationPass;
        assert_eq!(GenerationPass::new().name(), "Generation");
    }

    #[test]
    fn test_optimization_pass_wrapper() {
        let config = OptimizationConfig::o0();
        let pass = OptimizationPassWrapper::new(config);
        assert_eq!(pass.name(), "Optimization");
        assert_eq!(pass.dependencies(), vec!["Discovery", "Analysis"]);
        assert!(!pass.modifies_schema());
    }

    #[test]
    fn test_discovery_and_analysis_do_not_create_generation_side_effects() {
        let schema = json!({
            "type": "object",
            "properties": {
                "name": {"type": "string"},
                "age": {"type": "integer"}
            }
        });
        let mut context = PassContext::new();

        let mut discovery = DiscoveryPass::new();
        let discovery_result = discovery.run(&schema, &mut context).unwrap();
        assert!(matches!(discovery_result, PassResult::Discovery(_)));
        assert!(context.schema_discovery.is_some());
        assert_no_generation_side_effects(&context);
        assert!(context.analysis_results.is_none());
        assert!(context.encoding_strategy.is_none());
        assert!(context.code_generation.is_none());

        let mut analysis = AnalysisPass::new();
        let analysis_result = analysis.run(&schema, &mut context).unwrap();
        assert!(matches!(analysis_result, PassResult::Analysis(_)));
        assert!(context.analysis_results.is_some());
        assert_no_generation_side_effects(&context);
        assert!(context.encoding_strategy.is_none());
        assert!(context.code_generation.is_none());
    }

    // ============================================================================
    // Batch 3 - Error Handling Tests (5 tests)
    // ============================================================================

    #[test]
    fn test_analysis_pass_missing_discovery() {
        let mut pass = AnalysisPass::new();
        let mut context = PassContext::new();
        let schema = json!({"type": "object"});

        let result = pass.run(&schema, &mut context);
        assert!(result.is_err());

        let err = result.unwrap_err();
        match err {
            crate::error::TctError::CodeGeneration { message, .. } => {
                assert!(message.contains("AnalysisPass requires SchemaDiscovery"));
            }
            _ => panic!("Expected CodeGeneration error"),
        }
    }

    #[test]
    fn test_generation_pass_missing_discovery() {
        let mut pass = GenerationPass::new();
        let mut context = PassContext::new();
        let schema = json!({"type": "object"});

        let result = pass.run(&schema, &mut context);
        assert!(result.is_err());

        let err = result.unwrap_err();
        match err {
            crate::error::TctError::CodeGeneration { message, .. } => {
                assert!(message.contains("GenerationPass requires SchemaDiscovery"));
            }
            _ => panic!("Expected CodeGeneration error"),
        }
    }

    #[test]
    fn test_generation_pass_missing_analysis() {
        let mut pass = GenerationPass::new();
        let mut context = PassContext::new();

        // Populate discovery but not analysis
        let discovery = SchemaWalker::walk(&json!({"type": "object"}), "Root").unwrap();
        context.schema_discovery = Some(discovery);

        let schema = json!({"type": "object"});
        let result = pass.run(&schema, &mut context);
        assert!(result.is_err());

        let err = result.unwrap_err();
        match err {
            crate::error::TctError::CodeGeneration { message, .. } => {
                assert!(message.contains("GenerationPass requires TypeAnalysis"));
            }
            _ => panic!("Expected CodeGeneration error"),
        }
    }

    #[test]
    fn test_optimization_pass_missing_discovery() {
        let mut pass = OptimizationPassWrapper::new(OptimizationConfig::o0());
        let mut context = PassContext::new();
        let schema = json!({"type": "object"});

        let result = pass.run(&schema, &mut context);
        assert!(result.is_err());

        let err = result.unwrap_err();
        match err {
            crate::error::TctError::CodeGeneration { message, .. } => {
                assert!(message.contains("Optimization phase requires SchemaDiscovery"));
            }
            _ => panic!("Expected CodeGeneration error"),
        }
    }

    #[test]
    fn test_optimization_pass_missing_analysis() {
        let mut pass = OptimizationPassWrapper::new(OptimizationConfig::o0());
        let mut context = PassContext::new();

        // Populate discovery but not analysis
        let discovery = SchemaWalker::walk(&json!({"type": "object"}), "Root").unwrap();
        context.schema_discovery = Some(discovery);

        let schema = json!({"type": "object"});
        let result = pass.run(&schema, &mut context);
        assert!(result.is_err());

        let err = result.unwrap_err();
        match err {
            crate::error::TctError::CodeGeneration { message, .. } => {
                assert!(message.contains("Optimization phase requires TypeAnalysis"));
            }
            _ => panic!("Expected CodeGeneration error"),
        }
    }

    // ============================================================================
    // Batch 4 - PassOrchestrator Tests
    // ============================================================================

    #[test]
    fn test_orchestrator_new() {
        let orchestrator = PassOrchestrator::new();
        assert_eq!(orchestrator.passes.len(), 0);
    }

    #[test]
    fn test_orchestrator_default() {
        let orchestrator = PassOrchestrator::default();
        assert_eq!(orchestrator.passes.len(), 0);
    }

    #[test]
    fn test_orchestrator_add_pass() {
        let mut orchestrator = PassOrchestrator::new();
        orchestrator.add_pass(DiscoveryPass::new());
        orchestrator.add_pass(AnalysisPass::new());
        assert_eq!(orchestrator.passes.len(), 2);
    }

    #[test]
    fn test_orchestrator_run_all_basic() {
        let mut orchestrator = PassOrchestrator::new();
        orchestrator.add_pass(DiscoveryPass::new());

        let schema = json!({
            "type": "object",
            "properties": {
                "name": {"type": "string"}
            }
        });

        let result = orchestrator.run_all(&schema);
        assert!(result.is_ok());

        let context = result.unwrap();
        assert!(context.schema_discovery.is_some());
    }

    #[test]
    fn test_orchestrator_run_all_with_config() {
        let mut orchestrator = PassOrchestrator::new();
        orchestrator.add_pass(DiscoveryPass::new());
        orchestrator.add_pass(AnalysisPass::new());

        let schema = json!({
            "type": "object",
            "properties": {
                "name": {"type": "string"}
            }
        });

        let config = OptimizationConfig::o2();
        let result = orchestrator.run_all_with_config(&schema, config);
        assert!(result.is_ok());

        let context = result.unwrap();
        assert!(context.optimization_config.is_some());
        assert!(context.schema_discovery.is_some());
        assert!(context.analysis_results.is_some());
    }

    #[test]
    fn test_orchestrator_full_pipeline() {
        let mut orchestrator = PassOrchestrator::new();
        orchestrator.add_pass(DiscoveryPass::new());
        orchestrator.add_pass(AnalysisPass::new());
        orchestrator.add_pass(OptimizationPassWrapper::new(OptimizationConfig::o0()));
        orchestrator.add_pass(GenerationPass::new());

        let schema = json!({
            "type": "object",
            "properties": {
                "name": {"type": "string"},
                "age": {"type": "integer"}
            }
        });

        let result = orchestrator.run_all(&schema);
        assert!(result.is_ok());

        let context = result.unwrap();
        assert!(context.schema_discovery.is_some());
        assert!(context.analysis_results.is_some());
        assert!(context.encoding_strategy.is_some());
        assert!(context.code_generation.is_some());
    }

    #[test]
    fn test_orchestrator_dependency_order() {
        let mut orchestrator = PassOrchestrator::new();

        // Add passes in correct dependency order
        orchestrator.add_pass(DiscoveryPass::new());
        orchestrator.add_pass(AnalysisPass::new());

        let schema = json!({"type": "object", "properties": {"x": {"type": "string"}}});

        let result = orchestrator.run_all(&schema);
        assert!(result.is_ok());

        let context = result.unwrap();
        // AnalysisPass depends on DiscoveryPass, so both should succeed
        assert!(context.schema_discovery.is_some());
        assert!(context.analysis_results.is_some());
    }

    #[test]
    fn test_orchestrator_rejects_missing_dependencies_before_running_pass() {
        let mut orchestrator = PassOrchestrator::new();
        orchestrator.add_pass(AnalysisPass::new());
        orchestrator.add_pass(DiscoveryPass::new());

        let schema = json!({"type": "object", "properties": {"x": {"type": "string"}}});
        let result = orchestrator.run_all(&schema);

        match result {
            Err(crate::error::TctError::CodeGeneration {
                message,
                suggestion,
                ..
            }) => {
                assert!(message.contains("Analysis"));
                assert!(message.contains("Discovery"));
                assert!(suggestion.contains("Discovery"));
            }
            Err(other) => panic!("Expected dependency CodeGeneration error, got {other:?}"),
            Ok(_) => panic!("Expected dependency error for out-of-order passes"),
        }
    }

    #[test]
    fn test_orchestrator_rejects_missing_pipeline_dependency_before_any_pass_runs() {
        let mut orchestrator = PassOrchestrator::new();
        orchestrator.add_pass(TestPass::mutating("Mutating"));
        orchestrator.add_pass(TestPass::with_dependencies("NeedsMissing", vec!["Missing"]));

        let schema = json!({"type": "object"});
        let mut context = PassContext::new();
        let result = orchestrator.run_passes(&schema, &mut context);

        match result {
            Err(crate::error::TctError::CodeGeneration { message, .. }) => {
                assert!(message.contains("NeedsMissing"));
                assert!(message.contains("Missing"));
                assert!(
                    context.generated_code.is_empty(),
                    "preflight validation must reject invalid pipelines before any pass runs"
                );
            }
            Err(other) => panic!("Expected pipeline CodeGeneration error, got {other:?}"),
            Ok(_) => panic!("Expected missing dependency error"),
        }
    }

    #[test]
    fn test_orchestrator_rejects_duplicate_pass_names_before_running() {
        let mut orchestrator = PassOrchestrator::new();
        orchestrator.add_pass(TestPass::mutating("Duplicate"));
        orchestrator.add_pass(TestPass::new("Duplicate"));

        let schema = json!({"type": "object"});
        let mut context = PassContext::new();
        let result = orchestrator.run_passes(&schema, &mut context);

        match result {
            Err(crate::error::TctError::CodeGeneration { message, .. }) => {
                assert!(message.contains("duplicate"));
                assert!(message.contains("Duplicate"));
                assert!(context.generated_code.is_empty());
            }
            Err(other) => panic!("Expected duplicate pass CodeGeneration error, got {other:?}"),
            Ok(_) => panic!("Expected duplicate pass name error"),
        }
    }

    #[test]
    fn test_orchestrator_rejects_self_dependency_before_running() {
        let mut orchestrator = PassOrchestrator::new();
        orchestrator.add_pass(TestPass::with_dependencies("Selfish", vec!["Selfish"]));

        let schema = json!({"type": "object"});
        let mut context = PassContext::new();
        let result = orchestrator.run_passes(&schema, &mut context);

        match result {
            Err(crate::error::TctError::CodeGeneration { message, .. }) => {
                assert!(message.contains("Selfish"));
                assert!(message.contains("itself"));
                assert!(context.generated_code.is_empty());
            }
            Err(other) => panic!("Expected self-dependency CodeGeneration error, got {other:?}"),
            Ok(_) => panic!("Expected self-dependency error"),
        }
    }

    // ============================================================================
    // Batch 5 - Pass Trait Tests (4 tests)
    // ============================================================================

    #[test]
    fn test_pass_name_methods() {
        let discovery = DiscoveryPass::new();
        let analysis = AnalysisPass::new();
        let generation = GenerationPass::new();
        let optimization = OptimizationPassWrapper::new(OptimizationConfig::o0());

        assert_eq!(discovery.name(), "Discovery");
        assert_eq!(analysis.name(), "Analysis");
        assert_eq!(generation.name(), "Generation");
        assert_eq!(optimization.name(), "Optimization");
    }

    #[test]
    fn test_pass_dependencies() {
        let discovery = DiscoveryPass::new();
        let analysis = AnalysisPass::new();
        let generation = GenerationPass::new();
        let optimization = OptimizationPassWrapper::new(OptimizationConfig::o0());

        assert!(discovery.dependencies().is_empty());
        assert_eq!(analysis.dependencies(), vec!["Discovery"]);
        assert_eq!(
            generation.dependencies(),
            vec!["Discovery", "Analysis", "Optimization"]
        );
        assert_eq!(optimization.dependencies(), vec!["Discovery", "Analysis"]);
    }

    #[test]
    fn test_pass_modifies_schema() {
        let discovery = DiscoveryPass::new();
        let analysis = AnalysisPass::new();
        let generation = GenerationPass::new();

        // All current passes don't modify schema (default false)
        assert!(!discovery.modifies_schema());
        assert!(!analysis.modifies_schema());
        assert!(!generation.modifies_schema());
    }

    #[test]
    fn test_pass_result_variants() {
        // Test that we can construct each PassResult variant

        // Discovery
        let discovery = SchemaWalker::walk(&json!({"type": "object"}), "Root").unwrap();
        let _result = PassResult::Discovery(discovery.clone());

        // Analysis
        let analysis = TypeAnalyzer::analyze(discovery.clone(), OptimizationConfig::o0()).unwrap();
        let _result = PassResult::Analysis(analysis.clone());

        // Optimization (requires actual EncodingStrategy)
        let encoding_strategy = EncodingStrategy::default();
        let _result = PassResult::Optimization(encoding_strategy);

        // Generation
        let generation =
            CodeGenerator::generate(discovery, analysis, None, CodeGeneratorOptions::default())
                .unwrap();
        let _result = PassResult::Generation(generation);

        // Custom
        let mut custom_data = FxHashMap::default();
        custom_data.insert("key".to_string(), json!("value"));
        let _result = PassResult::Custom(custom_data);
    }

    struct TestPass {
        name: &'static str,
        dependencies: Vec<&'static str>,
        mutates_context: bool,
    }

    impl TestPass {
        fn new(name: &'static str) -> Self {
            Self {
                name,
                dependencies: Vec::new(),
                mutates_context: false,
            }
        }

        fn mutating(name: &'static str) -> Self {
            Self {
                name,
                dependencies: Vec::new(),
                mutates_context: true,
            }
        }

        fn with_dependencies(name: &'static str, dependencies: Vec<&'static str>) -> Self {
            Self {
                name,
                dependencies,
                mutates_context: false,
            }
        }
    }

    impl CompilationPass for TestPass {
        fn name(&self) -> &'static str {
            self.name
        }

        fn run(&mut self, _schema: &Value, context: &mut PassContext) -> TctResult<PassResult> {
            if self.mutates_context {
                context.generated_code.push(self.name.to_string());
            }

            Ok(PassResult::Custom(FxHashMap::default()))
        }

        fn dependencies(&self) -> Vec<&'static str> {
            self.dependencies.clone()
        }
    }

    fn assert_no_generation_side_effects(context: &PassContext) {
        assert!(context.definitions.structs.is_empty());
        assert!(context.definitions.enums.is_empty());
        assert!(context.definitions.constants.is_empty());
        assert!(context.definitions.type_aliases.is_empty());
        assert!(context.generated_type_names.is_empty());
        assert!(context.generated_code.is_empty());
    }
}
