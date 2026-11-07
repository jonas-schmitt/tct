//! Test utilities for comprehensive integration testing
//!
//! This module provides utilities for testing schema generation, optimization
//! behavior, and integration testing. These are internal testing utilities
//! and are not part of the public API.
//!
//! **WARNING**: This is NOT part of the public API and may change without notice.

pub mod integration_helpers;
pub mod optimization_assertions;
pub mod schema_generators;

// Re-export main utilities for backwards compatibility
pub use integration_helpers::*;
pub use optimization_assertions::*;
pub use schema_generators::SchemaTestUtils;
