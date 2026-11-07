//! Compatibility-only optimization configuration.
//!
//! This module preserves the historic `tct::optimization_pass` API for
//! optimization profiles, built-in pass names, and existing callers. It is not
//! the active extension point for new compiler architecture work.
//!
//! The active compiler pipeline is the four-pass path described in
//! `ARCHITECTURE.md`: discovery and analysis produce schema metadata,
//! optimization produces structured `EncodingStrategy` decisions, and only
//! generation emits Rust token streams. New optimizer behavior must add
//! structured metadata or encoding decisions consumed by generation, not custom
//! rendered-code transformations through this legacy trait surface.
//!
//! `OptimizationPass` and the concrete pass structs remain public for backward
//! compatibility. Custom `TokenStream`-returning passes are not preserved by
//! `OptimizationConfig::clone()` and are not accepted by
//! `TctCompiler::with_optimization_pass`, which intentionally accepts named
//! built-ins only.

#![allow(deprecated)]

pub mod config;
#[doc(hidden)]
pub mod core;
#[doc(hidden)]
pub mod passes;

// Re-export main profile/configuration types for the supported public API.
pub use config::{OptimizationConfig, PassConfig};
pub use core::OptimizationLevel;

// Legacy pass-construction types are hidden from generated docs so new callers
// do not treat rendered-token pass hooks as the compiler extension model.
#[doc(hidden)]
pub use core::{AnalysisResult, IntegerConstraints, OptimizationContext, OptimizationPass};
#[doc(hidden)]
pub use passes::{
    BoundedIntegerOptimizationPass, EnumIndexOptimizationPass, PatternOptimizationPass,
};

#[cfg(test)]
mod tests {
    use super::*;
    use rustc_hash::FxHashMap;

    #[test]
    fn test_optimization_config_creation() {
        let o0_config = OptimizationConfig::o0();
        assert_eq!(o0_config.level, OptimizationLevel::O0);
        assert!(o0_config.passes.is_empty());

        let o2_config = OptimizationConfig::o2();
        assert_eq!(o2_config.level, OptimizationLevel::O2);
        assert_eq!(o2_config.passes.len(), 2);
    }

    #[test]
    fn test_enum_optimization_pass() {
        let pass = EnumIndexOptimizationPass::new();
        assert_eq!(pass.name(), "enum_index_optimization");

        let enum_values = vec![
            "guest".to_string(),
            "member".to_string(),
            "admin".to_string(),
        ];
        let context = OptimizationContext {
            schema: serde_json::json!({}),
            optimization_level: OptimizationLevel::O2,
            max_vocabulary_size: None,
            analysis_results: FxHashMap::default(),
        };

        let enum_name = syn::parse_str::<syn::Ident>("UserType").unwrap();
        let result = pass.optimize_enum_encoding(&enum_name, &enum_values, &context);
        assert!(result.is_some());

        let tokens = result.unwrap();
        let code = tokens.to_string();
        assert!(code.contains("0u32"));
        assert!(code.contains("1u32"));
        assert!(code.contains("2u32"));
    }
}
