//! Schema Vocabulary Analysis Module
//!
//! This module provides functionality to analyze JSON schemas and determine
//! vocabulary requirements for each position in the tokenization process.
//! It identifies optimal vocabulary sizes and provides compression insights.

use rustc_hash::FxHashMap;
use serde_json::{Value, json};
use std::collections::HashSet;
use std::error::Error;
use std::fmt;
use std::hash::{Hash, Hasher};

mod recommendations;

fn inclusive_i64_range_size(minimum: i64, maximum: i64) -> Result<u128, AnalysisError> {
    if minimum > maximum {
        return Err(AnalysisError::InvalidSchema(format!(
            "integer minimum {minimum} is greater than maximum {maximum}"
        )));
    }

    Ok((i128::from(maximum) - i128::from(minimum) + 1) as u128)
}

fn u128_to_usize_saturating(value: u128) -> usize {
    usize::try_from(value).unwrap_or(usize::MAX)
}

/// Vocabulary analysis results for a JSON schema
#[derive(Debug, Clone)]
pub struct VocabularyAnalysis {
    /// Total number of distinct positions in the schema
    pub total_positions: usize,
    /// Maximum structural vocabulary size required at any single position
    /// This is the maximum vocabulary needed for schema-specific structural elements
    /// (enums, bounded integers, etc.) WITHOUT string encoding baseline.
    pub max_structural_vocab: usize,
    /// Total vocabulary size including string encoding baseline
    /// Calculated as: max(string_encoding_baseline, max_structural_vocab)
    /// - UTF-8 mode: max(257, max_structural_vocab)
    /// - ASCII mode: max(129, max_structural_vocab)
    pub max_vocabulary_size: usize,
    /// String encoding baseline (129 for ASCII, 257 for UTF-8)
    pub string_encoding_baseline: usize,
    /// Position that requires the maximum vocabulary
    pub max_vocabulary_position: String,
    /// Detailed analysis for each position
    pub position_analysis: FxHashMap<String, PositionAnalysis>,
    /// Overall schema complexity metrics
    pub complexity_metrics: ComplexityMetrics,
}

/// Analysis results for a specific position in the schema
#[derive(Debug, Clone)]
pub struct PositionAnalysis {
    /// JSON path to this position (e.g., "root.user.status")
    pub path: String,
    /// Type of data at this position
    pub data_type: DataType,
    /// Required vocabulary size for this position
    pub vocabulary_size: usize,
    /// Description of vocabulary requirements
    pub description: String,
    /// Suggested encoding strategy
    pub encoding_strategy: EncodingStrategy,
}

/// Position token overhead for windowed generation
#[derive(Debug, Clone)]
pub struct PositionTokenOverhead {
    /// Base vocabulary size (before position tokens)
    pub base_vocab_size: usize,
    /// Number of schema position tokens
    pub schema_position_tokens: usize,
    /// Number of gap position tokens
    pub gap_position_tokens: usize,
    /// Total vocabulary size including position tokens
    pub total_vocab_size: usize,
}

impl PositionTokenOverhead {
    /// Create position token overhead for given max sequence length
    pub fn new(base_vocab_size: usize, max_seq_len: usize) -> Self {
        let position_tokens = max_seq_len.saturating_mul(2);
        Self {
            base_vocab_size,
            schema_position_tokens: max_seq_len,
            gap_position_tokens: max_seq_len,
            total_vocab_size: base_vocab_size.saturating_add(position_tokens),
        }
    }
}

/// Complexity metrics for the overall schema
#[derive(Debug, Clone)]
pub struct ComplexityMetrics {
    /// Total number of object properties across the schema
    pub total_properties: usize,
    /// Number of enum types
    pub enum_count: usize,
    /// Number of string types
    pub string_count: usize,
    /// Number of numeric types (integer/number)
    pub numeric_count: usize,
    /// Maximum nesting depth
    pub max_depth: usize,
    /// Number of array types
    pub array_count: usize,
    /// Overall complexity score (0.0 to 1.0)
    pub complexity_score: f64,
    /// Average vocabulary size across all positions
    pub average_vocabulary_size: f64,
    /// Position token overhead (for windowed generation)
    ///
    /// When windowed generation is enabled, this tracks the additional
    /// vocabulary tokens required for position tracking:
    /// - Schema position tokens: Indicate window position in sequence
    /// - Gap position tokens: Indicate prediction position within window
    pub position_token_overhead: Option<PositionTokenOverhead>,
}

/// Data type classification for vocabulary analysis
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DataType {
    /// String type with optional constraints
    String {
        /// Minimum string length (from minLength constraint)
        min_length: Option<usize>,
        /// Maximum string length (from maxLength constraint)
        max_length: Option<usize>,
    },
    /// Integer type with optional bounds
    Integer {
        /// Minimum value (from minimum constraint)
        minimum: Option<i64>,
        /// Maximum value (from maximum constraint)
        maximum: Option<i64>,
    },
    /// Number type (floating point)
    Number,
    /// Boolean type
    Boolean,
    /// Enum with specific values
    Enum {
        /// Allowed enum values
        values: Vec<String>,
    },
    /// Array with item type
    Array {
        /// Type of array elements
        item_type: Box<DataType>,
    },
    /// Object with properties
    Object {
        /// Object properties mapped by name
        properties: FxHashMap<String, DataType>,
    },
    /// Union type (oneOf/anyOf)
    Union {
        /// Possible variant types
        variants: Vec<DataType>,
    },
    /// Null type
    Null,
    /// Unknown or unsupported type
    Unknown,
}

impl Hash for DataType {
    fn hash<H: Hasher>(&self, state: &mut H) {
        std::mem::discriminant(self).hash(state);
        match self {
            DataType::String {
                min_length,
                max_length,
            } => {
                min_length.hash(state);
                max_length.hash(state);
            }
            DataType::Integer { minimum, maximum } => {
                minimum.hash(state);
                maximum.hash(state);
            }
            DataType::Enum { values } => {
                values.hash(state);
            }
            DataType::Array { item_type } => {
                item_type.hash(state);
            }
            DataType::Object { properties: _ } => {
                // For objects, just hash the discriminant - don't hash the full HashMap
                // since it's complex and for our use case we just need to group objects
                "object".hash(state);
            }
            DataType::Union { variants } => {
                variants.hash(state);
            }
            _ => {} // Other variants don't have additional data
        }
    }
}

/// Suggested encoding strategy for a position
#[derive(Debug, Clone, PartialEq)]
pub enum EncodingStrategy {
    /// Direct token mapping (most efficient)
    Direct,
    /// Index-based encoding for enums
    IndexEncoding,
    /// Bounded integer encoding with offset
    BoundedInteger {
        /// Offset to subtract from values before encoding
        offset: i64,
    },
    /// Multi-token encoding for large vocabularies
    MultiToken {
        /// Number of tokens required per value
        tokens_per_value: usize,
    },
    /// String encoding with character-level tokens
    StringEncoding,
}

/// Vocabulary analysis error types
#[derive(Debug)]
pub enum AnalysisError {
    /// Invalid schema structure
    InvalidSchema(String),
    /// Unsupported schema feature
    UnsupportedFeature(String),
    /// Analysis computation error
    ComputationError(String),
}

impl fmt::Display for AnalysisError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AnalysisError::InvalidSchema(msg) => write!(f, "Invalid schema: {}", msg),
            AnalysisError::UnsupportedFeature(msg) => write!(f, "Unsupported feature: {}", msg),
            AnalysisError::ComputationError(msg) => write!(f, "Computation error: {}", msg),
        }
    }
}

impl Error for AnalysisError {}

/// Optimization recommendations based on vocabulary analysis
#[derive(Debug, Clone)]
pub struct OptimizationRecommendations {
    /// Recommended optimal vocabulary size for this schema
    pub optimal_vocab_size: Option<usize>,
    /// List of compression strategies to consider
    pub compression_strategies: Vec<CompressionStrategy>,
    /// Efficiency analysis across different vocabulary sizes
    pub efficiency_analysis: Vec<VocabularyEfficiency>,
    /// Warnings about potential issues or limitations
    pub warnings: Vec<String>,
}

/// Analysis of vocabulary efficiency for a specific vocabulary size
#[derive(Debug, Clone)]
pub struct VocabularyEfficiency {
    /// The vocabulary size being analyzed
    pub vocab_size: usize,
    /// Maximum vocabulary required by any position
    pub max_required_vocab: usize,
    /// Number of positions that would require multi-token encoding
    pub constrained_positions: usize,
    /// Total number of positions in the schema
    pub total_positions: usize,
    /// Compression ratio achieved (0.0 = no compression, 1.0 = maximum compression)
    pub compression_ratio: f64,
    /// Efficiency score (0-100, higher is better)
    pub efficiency_score: f64,
    /// Whether multi-token encoding would be needed
    pub multi_token_encoding_needed: bool,
}

/// Compression strategy recommendation
#[derive(Debug, Clone)]
pub struct CompressionStrategy {
    /// Type of optimization strategy
    pub strategy_type: String,
    /// Description of how the strategy works
    pub description: String,
    /// Expected vocabulary savings
    pub expected_savings: String,
    /// Implementation complexity level
    pub complexity: String,
}

/// Vocabulary budgets compared when the caller does not choose any, by
/// `tct --analyze` and by the Python `tct.analyze`.
pub const DEFAULT_COMPARISON_SIZES: [usize; 4] = [64, 128, 256, 512];

/// Build the JSON report printed by `tct --analyze --format json` and returned
/// by the Python `tct.analyze`.
///
/// `schema_label` is echoed as `schema_file`: a path for the command line, a
/// placeholder for schemas that never lived in a file.
pub fn analysis_report_json(
    analysis: &VocabularyAnalysis,
    recommendations: &OptimizationRecommendations,
    schema_label: &str,
) -> Value {
    json!({
        "schema_file": schema_label,
        "analysis": {
            "total_positions": analysis.total_positions,
            "max_vocabulary_size": analysis.max_vocabulary_size,
            "max_vocabulary_position": analysis.max_vocabulary_position,
            "complexity_metrics": {
                "total_properties": analysis.complexity_metrics.total_properties,
                "enum_count": analysis.complexity_metrics.enum_count,
                "string_count": analysis.complexity_metrics.string_count,
                "numeric_count": analysis.complexity_metrics.numeric_count,
                "max_depth": analysis.complexity_metrics.max_depth,
                "array_count": analysis.complexity_metrics.array_count,
                "complexity_score": analysis.complexity_metrics.complexity_score,
                "average_vocabulary_size": analysis.complexity_metrics.average_vocabulary_size
            }
        },
        "recommendations": {
            "optimal_vocab_size": recommendations.optimal_vocab_size,
            "compression_strategies": recommendations.compression_strategies.iter().map(|s| json!({
                "strategy_type": s.strategy_type,
                "description": s.description,
                "expected_savings": s.expected_savings,
                "complexity": s.complexity
            })).collect::<Vec<_>>(),
            "efficiency_analysis": recommendations.efficiency_analysis.iter().map(|e| json!({
                "vocab_size": e.vocab_size,
                "max_required_vocab": e.max_required_vocab,
                "constrained_positions": e.constrained_positions,
                "total_positions": e.total_positions,
                "compression_ratio": e.compression_ratio,
                "efficiency_score": e.efficiency_score,
                "multi_token_encoding_needed": e.multi_token_encoding_needed
            })).collect::<Vec<_>>(),
            "warnings": recommendations.warnings
        }
    })
}

/// Analyze `schema` against the given vocabulary budgets and return the JSON
/// report of [`analysis_report_json`].
pub fn analyze_to_report(
    schema: &Value,
    vocab_sizes: &[usize],
    schema_label: &str,
) -> Result<Value, AnalysisError> {
    let mut analyzer = VocabularyAnalyzer::new();
    let analysis = analyzer.analyze(schema)?;
    let recommendations = analyzer.generate_recommendations(&analysis, vocab_sizes);
    Ok(analysis_report_json(
        &analysis,
        &recommendations,
        schema_label,
    ))
}

/// Main vocabulary analyzer
pub struct VocabularyAnalyzer {
    /// Current analysis depth (for recursion tracking)
    current_depth: usize,
    /// Maximum allowed depth to prevent infinite recursion
    max_depth: usize,
    /// Visited schemas for circular reference detection
    visited_schemas: HashSet<String>,
    /// Reference-resolution warnings discovered during the latest analysis
    reference_warnings: Vec<String>,
    /// Deduplication keys for reference warnings from the latest analysis
    reported_reference_warnings: HashSet<String>,
    /// Root schema used to resolve local JSON Schema references during analysis
    root_schema: Option<Value>,
}

impl VocabularyAnalyzer {
    /// Create a new vocabulary analyzer
    pub fn new() -> Self {
        Self {
            current_depth: 0,
            max_depth: 100, // Reasonable limit for schema depth
            visited_schemas: HashSet::new(),
            reference_warnings: Vec::new(),
            reported_reference_warnings: HashSet::new(),
            root_schema: None,
        }
    }

    /// Analyze a JSON schema and return vocabulary requirements
    pub fn analyze(&mut self, schema: &Value) -> Result<VocabularyAnalysis, AnalysisError> {
        self.analyze_with_config(schema, None)
    }

    /// Analyze a JSON schema with windowed generation configuration
    ///
    /// # Arguments
    /// * `schema` - The JSON schema to analyze
    /// * `max_seq_len` - Optional maximum sequence length for windowed generation
    ///
    /// When `max_seq_len` is provided, position token overhead will be included
    /// in the complexity metrics.
    pub fn analyze_with_config(
        &mut self,
        schema: &Value,
        max_seq_len: Option<usize>,
    ) -> Result<VocabularyAnalysis, AnalysisError> {
        self.current_depth = 0;
        self.visited_schemas.clear();
        self.reference_warnings.clear();
        self.reported_reference_warnings.clear();
        self.root_schema = Some(schema.clone());

        let mut position_analysis = FxHashMap::default();
        let mut complexity_metrics = ComplexityMetrics {
            total_properties: 0,
            enum_count: 0,
            string_count: 0,
            numeric_count: 0,
            max_depth: 0,
            array_count: 0,
            complexity_score: 0.0,
            average_vocabulary_size: 0.0,
            position_token_overhead: None,
        };

        // Analyze the root schema
        let root_analysis = self.analyze_position(schema, "root", &mut complexity_metrics)?;
        let max_structural_vocab = root_analysis.vocabulary_size;
        let max_vocabulary_position = root_analysis.path.clone();

        position_analysis.insert("root".to_string(), root_analysis);

        // Find maximum structural vocabulary requirements across all positions
        let (max_structural, max_vocab_pos) = position_analysis
            .iter()
            .max_by_key(|(_, analysis)| analysis.vocabulary_size)
            .map(|(_, analysis)| (analysis.vocabulary_size, analysis.path.clone()))
            .unwrap_or((max_structural_vocab, max_vocabulary_position));

        // Calculate string encoding baseline
        // Default to UTF-8 (257 tokens: 0-255 bytes + END_STRING_MARKER at 256)
        // Note: In the future, this could be configurable (ASCII = 129 tokens)
        let string_encoding_baseline = 257; // UTF-8 mode

        // Calculate total vocabulary: max(string_encoding_baseline, max_structural_vocab)
        // This matches the actual generated tokenizer's calculation in grammar.rs
        let total_vocab = string_encoding_baseline.max(max_structural);

        // Calculate final complexity metrics
        complexity_metrics.average_vocabulary_size = if !position_analysis.is_empty() {
            position_analysis
                .values()
                .map(|analysis| analysis.vocabulary_size as f64)
                .sum::<f64>()
                / position_analysis.len() as f64
        } else {
            0.0
        };

        // Calculate position token overhead if windowed generation is enabled
        if let Some(max_len) = max_seq_len {
            let base_vocab = total_vocab;
            complexity_metrics.position_token_overhead =
                Some(PositionTokenOverhead::new(base_vocab, max_len));
        }

        // Calculate complexity score based on various factors
        complexity_metrics.complexity_score = {
            let depth_factor = (complexity_metrics.max_depth as f64) / 10.0; // Normalize to ~0-1
            let type_diversity = (complexity_metrics.enum_count
                + complexity_metrics.string_count
                + complexity_metrics.numeric_count
                + complexity_metrics.array_count) as f64
                / 10.0;
            let vocab_factor = complexity_metrics.average_vocabulary_size / 1000.0; // Normalize large vocabs

            ((depth_factor + type_diversity + vocab_factor) / 3.0).min(1.0)
        };

        Ok(VocabularyAnalysis {
            total_positions: position_analysis.len(),
            max_structural_vocab: max_structural,
            max_vocabulary_size: total_vocab,
            string_encoding_baseline,
            max_vocabulary_position: max_vocab_pos,
            position_analysis,
            complexity_metrics,
        })
    }

    /// Analyze a specific position in the schema
    fn analyze_position(
        &mut self,
        schema: &Value,
        path: &str,
        complexity: &mut ComplexityMetrics,
    ) -> Result<PositionAnalysis, AnalysisError> {
        // Update depth tracking
        self.current_depth += 1;
        if self.current_depth > self.max_depth {
            return Err(AnalysisError::ComputationError(
                "Schema depth exceeds maximum allowed depth".to_string(),
            ));
        }

        complexity.max_depth = complexity.max_depth.max(self.current_depth);

        let result = self.analyze_position_internal(schema, path, complexity);

        self.current_depth -= 1;
        result
    }

    /// Internal position analysis implementation
    fn analyze_position_internal(
        &mut self,
        schema: &Value,
        path: &str,
        complexity: &mut ComplexityMetrics,
    ) -> Result<PositionAnalysis, AnalysisError> {
        // Handle schema references
        if let Some(ref_value) = schema.get("$ref").and_then(Value::as_str) {
            return self.handle_reference(ref_value, path, complexity);
        }

        // Determine data type
        let data_type = self.determine_data_type(schema, path, complexity)?;
        let (vocabulary_size, description, encoding_strategy) =
            self.calculate_vocabulary_requirements(&data_type)?;

        Ok(PositionAnalysis {
            path: path.to_string(),
            data_type,
            vocabulary_size,
            description,
            encoding_strategy,
        })
    }

    /// Handle schema references ($ref)
    fn handle_reference(
        &mut self,
        ref_value: &str,
        path: &str,
        complexity: &mut ComplexityMetrics,
    ) -> Result<PositionAnalysis, AnalysisError> {
        if let Some(resolved_schema) = self.resolve_local_ref(ref_value) {
            if !self.visited_schemas.insert(ref_value.to_string()) {
                self.record_cyclic_reference_warning(ref_value, path);
                return Ok(Self::unresolved_reference_analysis(path));
            }

            let result = self.analyze_position(&resolved_schema, path, complexity);
            self.visited_schemas.remove(ref_value);
            return result;
        }

        Ok(Self::unresolved_reference_analysis(path))
    }

    fn unresolved_reference_analysis(path: &str) -> PositionAnalysis {
        PositionAnalysis {
            path: path.to_string(),
            data_type: DataType::Unknown,
            vocabulary_size: 100, // Conservative estimate
            description: "Referenced schema (unresolved)".to_string(),
            encoding_strategy: EncodingStrategy::MultiToken {
                tokens_per_value: 2,
            },
        }
    }

    fn resolve_local_ref(&self, ref_path: &str) -> Option<Value> {
        let root_schema = self.root_schema.as_ref()?;
        crate::schema_utils::resolve_local_ref(root_schema, ref_path).cloned()
    }

    fn record_cyclic_reference_warning(&mut self, ref_value: &str, path: &str) {
        let key = format!("{path}\0{ref_value}");
        if self.reported_reference_warnings.insert(key) {
            self.reference_warnings.push(format!(
                "Cyclic local $ref detected at {path}: {ref_value}. Vocabulary analysis used the conservative unresolved-reference estimate for this position."
            ));
        }
    }

    fn determine_reference_data_type(
        &mut self,
        ref_value: &str,
        path: &str,
        complexity: &mut ComplexityMetrics,
    ) -> Result<DataType, AnalysisError> {
        if let Some(resolved_schema) = self.resolve_local_ref(ref_value) {
            if !self.visited_schemas.insert(ref_value.to_string()) {
                self.record_cyclic_reference_warning(ref_value, path);
                return Ok(DataType::Unknown);
            }

            let result = self.determine_data_type(&resolved_schema, path, complexity);
            self.visited_schemas.remove(ref_value);
            return result;
        }

        Ok(DataType::Unknown)
    }

    /// Determine the data type from a schema
    fn determine_data_type(
        &mut self,
        schema: &Value,
        path: &str,
        complexity: &mut ComplexityMetrics,
    ) -> Result<DataType, AnalysisError> {
        if let Some(ref_value) = schema.get("$ref").and_then(Value::as_str) {
            return self.determine_reference_data_type(ref_value, path, complexity);
        }

        // Handle enum types first
        if let Some(enum_values) = schema.get("enum").and_then(Value::as_array) {
            complexity.enum_count += 1;
            let values: Vec<String> = enum_values.iter().map(|v| v.to_string()).collect();
            return Ok(DataType::Enum { values });
        }

        // Handle typed schemas
        if let Some(type_str) = schema.get("type").and_then(Value::as_str) {
            match type_str {
                "string" => {
                    complexity.string_count += 1;
                    let min_length = schema
                        .get("minLength")
                        .and_then(Value::as_u64)
                        .map(|v| v as usize);
                    let max_length = schema
                        .get("maxLength")
                        .and_then(Value::as_u64)
                        .map(|v| v as usize);
                    Ok(DataType::String {
                        min_length,
                        max_length,
                    })
                }
                "integer" => {
                    complexity.numeric_count += 1;
                    let minimum = schema.get("minimum").and_then(Value::as_i64);
                    let maximum = schema.get("maximum").and_then(Value::as_i64);
                    Ok(DataType::Integer { minimum, maximum })
                }
                "number" => {
                    complexity.numeric_count += 1;
                    Ok(DataType::Number)
                }
                "boolean" => Ok(DataType::Boolean),
                "null" => Ok(DataType::Null),
                "array" => {
                    complexity.array_count += 1;
                    if let Some(items_schema) = schema.get("items") {
                        let item_type = self.determine_data_type(
                            items_schema,
                            &format!("{}[*]", path),
                            complexity,
                        )?;
                        Ok(DataType::Array {
                            item_type: Box::new(item_type),
                        })
                    } else {
                        Ok(DataType::Array {
                            item_type: Box::new(DataType::Unknown),
                        })
                    }
                }
                "object" => {
                    if let Some(properties) = schema.get("properties").and_then(Value::as_object) {
                        complexity.total_properties += properties.len();
                        let mut prop_types = FxHashMap::default();

                        // Recursively analyze each property to ensure proper depth tracking
                        for (prop_name, prop_schema) in properties {
                            let prop_path = format!("{}.{}", path, prop_name);
                            // Use analyze_position to ensure depth tracking works correctly
                            let _prop_analysis =
                                self.analyze_position(prop_schema, &prop_path, complexity)?;
                            let prop_type =
                                self.determine_data_type(prop_schema, &prop_path, complexity)?;
                            prop_types.insert(prop_name.clone(), prop_type);
                        }

                        Ok(DataType::Object {
                            properties: prop_types,
                        })
                    } else {
                        Ok(DataType::Object {
                            properties: FxHashMap::default(),
                        })
                    }
                }
                _ => Ok(DataType::Unknown),
            }
        } else if schema.get("oneOf").is_some() || schema.get("anyOf").is_some() {
            // Handle union types
            let variants_key = if schema.get("oneOf").is_some() {
                "oneOf"
            } else {
                "anyOf"
            };
            if let Some(variants_array) = schema.get(variants_key).and_then(Value::as_array) {
                let mut variants = Vec::new();
                for (i, variant_schema) in variants_array.iter().enumerate() {
                    let variant_path = format!("{}|variant{}", path, i);
                    let variant_type =
                        self.determine_data_type(variant_schema, &variant_path, complexity)?;
                    variants.push(variant_type);
                }
                Ok(DataType::Union { variants })
            } else {
                Ok(DataType::Unknown)
            }
        } else {
            Ok(DataType::Unknown)
        }
    }

    /// Calculate vocabulary requirements for a data type
    #[allow(clippy::only_used_in_recursion)]
    fn calculate_vocabulary_requirements(
        &self,
        data_type: &DataType,
    ) -> Result<(usize, String, EncodingStrategy), AnalysisError> {
        match data_type {
            DataType::Boolean => Ok((
                2,
                "Boolean: 2 values (true, false)".to_string(),
                EncodingStrategy::Direct,
            )),
            DataType::Null => Ok((1, "Null: 1 value".to_string(), EncodingStrategy::Direct)),
            DataType::Enum { values } => {
                let size = values.len();
                let description = format!("Enum: {} values", size);
                let strategy = if size <= 256 {
                    EncodingStrategy::IndexEncoding
                } else {
                    EncodingStrategy::MultiToken {
                        tokens_per_value: 2,
                    }
                };
                Ok((size, description, strategy))
            }
            DataType::Integer { minimum, maximum } => {
                if let (Some(min), Some(max)) = (minimum, maximum) {
                    let range_size = inclusive_i64_range_size(*min, *max)?;
                    let range = u128_to_usize_saturating(range_size);
                    let description = format!(
                        "Bounded integer: {} to {} ({} values)",
                        min, max, range_size
                    );
                    let strategy = if range_size <= 1024 {
                        EncodingStrategy::BoundedInteger { offset: *min }
                    } else {
                        EncodingStrategy::MultiToken {
                            tokens_per_value: 2,
                        }
                    };
                    Ok((range, description, strategy))
                } else {
                    Ok((
                        65536,
                        "Unbounded integer: ~65K common values".to_string(),
                        EncodingStrategy::MultiToken {
                            tokens_per_value: 2,
                        },
                    ))
                }
            }
            DataType::Number => Ok((
                65536,
                "Number: ~65K common values".to_string(),
                EncodingStrategy::MultiToken {
                    tokens_per_value: 2,
                },
            )),
            DataType::String {
                min_length: _,
                max_length,
            } => {
                let estimated_size = if let Some(max_len) = max_length {
                    // Estimate based on maximum length and character set with overflow protection
                    let char_set_size: u32 = 128; // ASCII assumption
                    let safe_exponent = (*max_len as u32 / 2).min(10); // Cap exponent to prevent overflow
                    let max_combinations = char_set_size.saturating_pow(safe_exponent);
                    (max_combinations as usize).min(100000) // Cap at reasonable size
                } else {
                    10000 // Default estimate for unbounded strings
                };
                Ok((
                    estimated_size,
                    format!("String: ~{} estimated combinations", estimated_size),
                    EncodingStrategy::StringEncoding,
                ))
            }
            DataType::Array { item_type } => {
                let item_vocab = self.calculate_vocabulary_requirements(item_type)?.0;
                let size = 100.max(item_vocab);
                // Arrays typically require structural tokens + item tokens
                Ok((
                    size,
                    format!(
                        "Array: ~100 structural tokens, max item vocabulary {}",
                        item_vocab
                    ),
                    EncodingStrategy::MultiToken {
                        tokens_per_value: 2,
                    },
                ))
            }
            DataType::Object { properties } => {
                let structural_size = properties.len().saturating_mul(10); // Rough estimate
                let mut max_property_vocab = 0;
                for property_type in properties.values() {
                    let property_vocab = self.calculate_vocabulary_requirements(property_type)?.0;
                    max_property_vocab = max_property_vocab.max(property_vocab);
                }
                let size = structural_size.max(max_property_vocab);
                Ok((
                    size,
                    format!(
                        "Object: {} properties (~{} structural tokens, max property vocabulary {})",
                        properties.len(),
                        structural_size,
                        max_property_vocab
                    ),
                    EncodingStrategy::MultiToken {
                        tokens_per_value: 3,
                    },
                ))
            }
            DataType::Union { variants } => {
                let mut max_variant_size = 100;
                for variant in variants {
                    let variant_vocab = self.calculate_vocabulary_requirements(variant)?.0;
                    max_variant_size = max_variant_size.max(variant_vocab);
                }
                Ok((
                    max_variant_size.saturating_add(variants.len()),
                    format!(
                        "Union: {} variants, max variant size {}",
                        variants.len(),
                        max_variant_size
                    ),
                    EncodingStrategy::MultiToken {
                        tokens_per_value: 2,
                    },
                ))
            }
            DataType::Unknown => Ok((
                100,
                "Unknown type: conservative estimate".to_string(),
                EncodingStrategy::MultiToken {
                    tokens_per_value: 2,
                },
            )),
        }
    }
}

impl Default for VocabularyAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_vocabulary_analyzer_basic_functionality() {
        let mut analyzer = VocabularyAnalyzer::new();
        let schema = json!({
            "type": "object",
            "properties": {
                "name": { "type": "string" },
                "status": { "enum": ["active", "pending", "completed"] }
            }
        });

        let result = analyzer.analyze(&schema);
        assert!(result.is_ok(), "Basic schema analysis should succeed");

        let analysis = result.unwrap();
        assert!(
            analysis.total_positions > 0,
            "Should analyze at least one position"
        );
        assert!(
            analysis.max_vocabulary_size > 0,
            "Should calculate vocabulary size"
        );
    }

    #[test]
    fn test_vocabulary_analyzer_enum_analysis() {
        let mut analyzer = VocabularyAnalyzer::new();
        let schema = json!({
            "enum": ["red", "green", "blue"]
        });

        let result = analyzer.analyze(&schema);
        assert!(result.is_ok(), "Enum analysis should succeed");

        let analysis = result.unwrap();
        assert_eq!(
            analysis.max_structural_vocab, 3,
            "Enum should have 3 values (structural)"
        );
        assert_eq!(
            analysis.max_vocabulary_size, 257,
            "Total vocabulary should be 257 (UTF-8 baseline)"
        );
    }

    #[test]
    fn test_vocabulary_analyzer_bounded_integer() {
        let mut analyzer = VocabularyAnalyzer::new();
        let schema = json!({
            "type": "integer",
            "minimum": 1,
            "maximum": 10
        });

        let result = analyzer.analyze(&schema);
        assert!(result.is_ok(), "Bounded integer analysis should succeed");

        let analysis = result.unwrap();
        assert_eq!(
            analysis.max_structural_vocab, 10,
            "Bounded integer should have 10 values (1-10) in structural vocab"
        );
        assert_eq!(
            analysis.max_vocabulary_size, 257,
            "Total vocabulary should be 257 (UTF-8 baseline)"
        );
    }

    #[test]
    fn test_vocabulary_analyzer_extreme_i64_integer_range_does_not_overflow() {
        let mut analyzer = VocabularyAnalyzer::new();
        let schema = json!({
            "type": "integer",
            "minimum": i64::MIN,
            "maximum": i64::MAX
        });

        let analysis = analyzer
            .analyze(&schema)
            .expect("extreme bounded integer analysis should not overflow");

        assert_eq!(analysis.max_structural_vocab, usize::MAX);
        assert_eq!(analysis.max_vocabulary_size, usize::MAX);
        assert!(
            analysis
                .complexity_metrics
                .average_vocabulary_size
                .is_finite()
        );
        assert!(matches!(
            &analysis.position_analysis["root"].encoding_strategy,
            EncodingStrategy::MultiToken {
                tokens_per_value: 2
            }
        ));
        assert!(
            analysis.position_analysis["root"]
                .description
                .contains("18446744073709551616 values")
        );
    }

    #[test]
    fn test_vocabulary_analyzer_invalid_integer_range_returns_error() {
        let mut analyzer = VocabularyAnalyzer::new();
        let schema = json!({
            "type": "integer",
            "minimum": 10,
            "maximum": 1
        });

        let error = analyzer
            .analyze(&schema)
            .expect_err("invalid integer range should fail analysis");

        assert!(matches!(error, AnalysisError::InvalidSchema(_)));
        assert!(
            error
                .to_string()
                .contains("minimum 10 is greater than maximum 1")
        );
    }

    #[test]
    fn test_vocabulary_analyzer_nested_invalid_integer_range_returns_error() {
        let mut analyzer = VocabularyAnalyzer::new();
        let schema = json!({
            "type": "object",
            "properties": {
                "bad": {
                    "type": "integer",
                    "minimum": 10,
                    "maximum": 1
                }
            }
        });

        let error = analyzer
            .analyze(&schema)
            .expect_err("invalid nested integer range should fail analysis");

        assert!(matches!(error, AnalysisError::InvalidSchema(_)));
        assert!(
            error
                .to_string()
                .contains("minimum 10 is greater than maximum 1")
        );
    }

    #[test]
    fn test_position_token_overhead_saturates_total_vocab_size() {
        let overhead = PositionTokenOverhead::new(usize::MAX, 10);

        assert_eq!(overhead.base_vocab_size, usize::MAX);
        assert_eq!(overhead.schema_position_tokens, 10);
        assert_eq!(overhead.gap_position_tokens, 10);
        assert_eq!(overhead.total_vocab_size, usize::MAX);
    }

    #[test]
    fn test_vocabulary_analyzer_complex_schema() {
        let mut analyzer = VocabularyAnalyzer::new();
        let schema = json!({
            "type": "object",
            "properties": {
                "user": {
                    "type": "object",
                    "properties": {
                        "name": { "type": "string" },
                        "age": { "type": "integer", "minimum": 0, "maximum": 150 }
                    }
                },
                "tags": {
                    "type": "array",
                    "items": { "type": "string" }
                }
            }
        });

        let result = analyzer.analyze(&schema);
        assert!(result.is_ok(), "Complex schema analysis should succeed");

        let analysis = result.unwrap();
        assert!(
            analysis.complexity_metrics.total_properties > 0,
            "Should count properties"
        );
        assert!(
            analysis.complexity_metrics.max_depth > 1,
            "Should detect nesting"
        );
    }

    #[test]
    fn test_vocabulary_recommendations_basic_functionality() {
        let mut analyzer = VocabularyAnalyzer::new();
        let schema = json!({
            "type": "object",
            "properties": {
                "status": { "enum": ["active", "pending", "completed"] },
                "priority": { "type": "integer", "minimum": 1, "maximum": 5 }
            }
        });

        let analysis = analyzer.analyze(&schema).unwrap();
        let vocab_sizes = vec![64, 128, 256, 512];
        let recommendations = analyzer.generate_recommendations(&analysis, &vocab_sizes);

        assert_eq!(
            recommendations.efficiency_analysis.len(),
            4,
            "Should analyze all vocabulary sizes"
        );
        assert!(
            recommendations.optimal_vocab_size.is_some(),
            "Should recommend an optimal size"
        );
        assert!(
            !recommendations.compression_strategies.is_empty(),
            "Should suggest compression strategies"
        );
    }

    #[test]
    fn test_vocabulary_efficiency_calculation() {
        let mut analyzer = VocabularyAnalyzer::new();
        let schema = json!({
            "type": "object",
            "properties": {
                "simple_enum": { "enum": ["a", "b", "c"] }
            }
        });

        let analysis = analyzer.analyze(&schema).unwrap();
        let efficiency = analyzer.calculate_vocabulary_efficiency(&analysis, 128);

        assert_eq!(efficiency.vocab_size, 128);
        assert!(efficiency.efficiency_score >= 0.0 && efficiency.efficiency_score <= 100.0);
        assert!(
            efficiency.max_required_vocab > 0,
            "Should detect vocabulary requirements"
        );
    }

    #[test]
    fn test_compression_strategy_generation() {
        let mut analyzer = VocabularyAnalyzer::new();
        let schema = json!({
            "type": "object",
            "properties": {
                "status1": { "enum": ["active", "pending"] },
                "status2": { "enum": ["new", "old"] },
                "priority": { "type": "integer", "minimum": 1, "maximum": 10 }
            }
        });

        let analysis = analyzer.analyze(&schema).unwrap();

        let strategies = analyzer.generate_compression_strategies(&analysis);

        assert!(
            !strategies.is_empty(),
            "Should generate compression strategies"
        );

        // Should recommend enum optimization since enums dominate
        let has_enum_strategy = strategies
            .iter()
            .any(|s| s.strategy_type == "enum_optimization");
        assert!(
            has_enum_strategy,
            "Should recommend enum optimization for enum-heavy schema"
        );
    }

    #[test]
    fn test_warning_generation() {
        let mut analyzer = VocabularyAnalyzer::new();

        // Create a schema that should trigger warnings
        let deep_schema = json!({
            "type": "object",
            "properties": {
                "level1": {
                    "type": "object",
                    "properties": {
                        "level2": {
                            "type": "object",
                            "properties": {
                                "level3": {
                                    "type": "object",
                                    "properties": {
                                        "level4": { "type": "string" }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        });

        let analysis = analyzer.analyze(&deep_schema).unwrap();
        let warnings = analyzer.generate_warnings(&analysis);

        // The exact warnings depend on the complexity metrics calculation
        // but we should get some feedback about the schema structure without errors
        assert!(
            warnings.len() <= 10,
            "Should not generate excessive warnings"
        );
    }

    #[test]
    fn test_position_token_overhead() {
        let mut analyzer = VocabularyAnalyzer::new();
        let schema = json!({
            "type": "object",
            "properties": {
                "status": { "enum": ["active", "pending", "completed"] }
            }
        });

        // Test without windowed generation
        let analysis_no_windowing = analyzer.analyze(&schema).unwrap();
        assert!(
            analysis_no_windowing
                .complexity_metrics
                .position_token_overhead
                .is_none(),
            "Should not have position token overhead without windowed generation"
        );

        // Test with windowed generation
        let max_seq_len = 512;
        let analysis_with_windowing = analyzer
            .analyze_with_config(&schema, Some(max_seq_len))
            .unwrap();

        assert!(
            analysis_with_windowing
                .complexity_metrics
                .position_token_overhead
                .is_some(),
            "Should have position token overhead with windowed generation"
        );

        let overhead = analysis_with_windowing
            .complexity_metrics
            .position_token_overhead
            .unwrap();

        // Verify position token allocation
        assert_eq!(
            overhead.schema_position_tokens, max_seq_len,
            "Should allocate max_seq_len schema position tokens"
        );
        assert_eq!(
            overhead.gap_position_tokens, max_seq_len,
            "Should allocate max_seq_len gap position tokens"
        );
        assert_eq!(
            overhead.total_vocab_size,
            overhead.base_vocab_size + 2 * max_seq_len,
            "Total vocab should be base + 2*max_seq_len"
        );

        // Example: base_vocab=3 (enum), max_seq_len=512
        // total_vocab = 3 + 2*512 = 1027
        assert_eq!(
            overhead.total_vocab_size,
            analysis_with_windowing.max_vocabulary_size + 2 * max_seq_len,
            "Total vocab should include position token overhead"
        );
    }

    // ========== Error Display Tests ==========

    #[test]
    fn test_analysis_error_invalid_schema_display() {
        let error = AnalysisError::InvalidSchema("test message".to_string());
        assert_eq!(error.to_string(), "Invalid schema: test message");
    }

    #[test]
    fn test_analysis_error_unsupported_feature_display() {
        let error = AnalysisError::UnsupportedFeature("advanced regex".to_string());
        assert_eq!(error.to_string(), "Unsupported feature: advanced regex");
    }

    #[test]
    fn test_analysis_error_computation_error_display() {
        let error = AnalysisError::ComputationError("overflow".to_string());
        assert_eq!(error.to_string(), "Computation error: overflow");
    }

    // ========== DataType Hash Tests ==========

    #[test]
    fn test_datatype_hash_string() {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let dt1 = DataType::String {
            min_length: Some(5),
            max_length: Some(10),
        };
        let dt2 = DataType::String {
            min_length: Some(5),
            max_length: Some(10),
        };

        let mut hasher1 = DefaultHasher::new();
        let mut hasher2 = DefaultHasher::new();
        dt1.hash(&mut hasher1);
        dt2.hash(&mut hasher2);

        assert_eq!(hasher1.finish(), hasher2.finish());
    }

    #[test]
    fn test_datatype_hash_integer() {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let dt1 = DataType::Integer {
            minimum: Some(0),
            maximum: Some(100),
        };
        let dt2 = DataType::Integer {
            minimum: Some(0),
            maximum: Some(100),
        };

        let mut hasher1 = DefaultHasher::new();
        let mut hasher2 = DefaultHasher::new();
        dt1.hash(&mut hasher1);
        dt2.hash(&mut hasher2);

        assert_eq!(hasher1.finish(), hasher2.finish());
    }

    #[test]
    fn test_datatype_hash_enum() {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let dt1 = DataType::Enum {
            values: vec!["a".to_string(), "b".to_string()],
        };
        let dt2 = DataType::Enum {
            values: vec!["a".to_string(), "b".to_string()],
        };

        let mut hasher1 = DefaultHasher::new();
        let mut hasher2 = DefaultHasher::new();
        dt1.hash(&mut hasher1);
        dt2.hash(&mut hasher2);

        assert_eq!(hasher1.finish(), hasher2.finish());
    }

    #[test]
    fn test_datatype_hash_array() {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let dt1 = DataType::Array {
            item_type: Box::new(DataType::String {
                min_length: None,
                max_length: None,
            }),
        };
        let dt2 = DataType::Array {
            item_type: Box::new(DataType::String {
                min_length: None,
                max_length: None,
            }),
        };

        let mut hasher1 = DefaultHasher::new();
        let mut hasher2 = DefaultHasher::new();
        dt1.hash(&mut hasher1);
        dt2.hash(&mut hasher2);

        assert_eq!(hasher1.finish(), hasher2.finish());
    }

    #[test]
    fn test_datatype_hash_object() {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        // Objects hash only their discriminant, not properties
        let mut props1 = FxHashMap::default();
        props1.insert("a".to_string(), DataType::Boolean);

        let mut props2 = FxHashMap::default();
        props2.insert("b".to_string(), DataType::Number);

        let dt1 = DataType::Object { properties: props1 };
        let dt2 = DataType::Object { properties: props2 };

        let mut hasher1 = DefaultHasher::new();
        let mut hasher2 = DefaultHasher::new();
        dt1.hash(&mut hasher1);
        dt2.hash(&mut hasher2);

        // Objects should hash the same regardless of properties
        assert_eq!(hasher1.finish(), hasher2.finish());
    }

    #[test]
    fn test_datatype_hash_union() {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let dt1 = DataType::Union {
            variants: vec![DataType::Boolean, DataType::Number],
        };
        let dt2 = DataType::Union {
            variants: vec![DataType::Boolean, DataType::Number],
        };

        let mut hasher1 = DefaultHasher::new();
        let mut hasher2 = DefaultHasher::new();
        dt1.hash(&mut hasher1);
        dt2.hash(&mut hasher2);

        assert_eq!(hasher1.finish(), hasher2.finish());
    }

    #[test]
    fn test_datatype_hash_simple_types() {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let types = vec![
            DataType::Boolean,
            DataType::Number,
            DataType::Null,
            DataType::Unknown,
        ];

        for dt in types {
            let mut hasher = DefaultHasher::new();
            dt.hash(&mut hasher);
            // Just ensure hashing doesn't panic
            assert!(hasher.finish() != 0 || hasher.finish() == 0);
        }
    }

    // ========== VocabularyAnalyzer Default ==========

    #[test]
    fn test_vocabulary_analyzer_default() {
        let analyzer = VocabularyAnalyzer::default();
        let schema = json!({"type": "boolean"});
        let result = VocabularyAnalyzer { ..analyzer }.analyze(&schema);
        assert!(result.is_ok());
    }

    // ========== Data Type Determination Tests ==========

    #[test]
    fn test_analyze_number_type() {
        let mut analyzer = VocabularyAnalyzer::new();
        let schema = json!({"type": "number"});
        let result = analyzer.analyze(&schema).unwrap();
        assert_eq!(result.max_vocabulary_size, 65536); // Number type (65536 > 257, so total = 65536)
    }

    #[test]
    fn test_analyze_boolean_type() {
        let mut analyzer = VocabularyAnalyzer::new();
        let schema = json!({"type": "boolean"});
        let result = analyzer.analyze(&schema).unwrap();
        assert_eq!(result.max_vocabulary_size, 257); // UTF-8 baseline dominates (2 < 257)
    }

    #[test]
    fn test_analyze_null_type() {
        let mut analyzer = VocabularyAnalyzer::new();
        let schema = json!({"type": "null"});
        let result = analyzer.analyze(&schema).unwrap();
        assert_eq!(result.max_vocabulary_size, 257); // UTF-8 baseline dominates (1 < 257)
    }

    #[test]
    fn test_analyze_unknown_type() {
        let mut analyzer = VocabularyAnalyzer::new();
        let schema = json!({"type": "unsupported_type"});
        let result = analyzer.analyze(&schema).unwrap();
        assert_eq!(result.max_vocabulary_size, 257); // UTF-8 baseline dominates (100 < 257)
    }

    #[test]
    fn test_analyze_string_with_min_length() {
        let mut analyzer = VocabularyAnalyzer::new();
        let schema = json!({
            "type": "string",
            "minLength": 5,
            "maxLength": 10
        });
        let result = analyzer.analyze(&schema).unwrap();
        assert!(result.max_vocabulary_size > 0);
    }

    #[test]
    fn test_analyze_string_unbounded() {
        let mut analyzer = VocabularyAnalyzer::new();
        let schema = json!({"type": "string"});
        let result = analyzer.analyze(&schema).unwrap();
        assert_eq!(result.max_vocabulary_size, 10000); // String estimate (10000 > 257, so total = 10000)
    }

    #[test]
    fn test_analyze_union_oneof() {
        let mut analyzer = VocabularyAnalyzer::new();
        let schema = json!({
            "oneOf": [
                {"type": "string"},
                {"type": "integer", "minimum": 0, "maximum": 10}
            ]
        });
        let result = analyzer.analyze(&schema).unwrap();
        // Union size = max variant (10000 for string) + 2 variants
        assert!(result.max_vocabulary_size >= 10000);
    }

    #[test]
    fn test_analyze_union_anyof() {
        let mut analyzer = VocabularyAnalyzer::new();
        let schema = json!({
            "anyOf": [
                {"type": "boolean"},
                {"type": "null"}
            ]
        });
        let result = analyzer.analyze(&schema).unwrap();
        // Union size = max variant (2 for boolean) + 2 variants
        assert!(result.max_vocabulary_size >= 2);
    }

    #[test]
    fn test_analyze_array_with_items() {
        let mut analyzer = VocabularyAnalyzer::new();
        let schema = json!({
            "type": "array",
            "items": {"type": "integer", "minimum": 0, "maximum": 5}
        });
        let result = analyzer.analyze(&schema).unwrap();
        assert_eq!(result.max_vocabulary_size, 257); // UTF-8 baseline dominates (100 < 257)
    }

    #[test]
    fn test_analyze_array_without_items() {
        let mut analyzer = VocabularyAnalyzer::new();
        let schema = json!({"type": "array"});
        let result = analyzer.analyze(&schema).unwrap();
        assert_eq!(result.max_vocabulary_size, 257); // UTF-8 baseline dominates (100 < 257)
    }

    #[test]
    fn test_analyze_object_empty_properties() {
        let mut analyzer = VocabularyAnalyzer::new();
        let schema = json!({"type": "object"});
        let result = analyzer.analyze(&schema).unwrap();
        assert_eq!(result.max_vocabulary_size, 257); // UTF-8 baseline dominates (0 < 257)
    }

    #[test]
    fn test_analyze_ref_handling() {
        let mut analyzer = VocabularyAnalyzer::new();
        let schema = json!({"$ref": "#/definitions/User"});
        let result = analyzer.analyze(&schema).unwrap();
        assert_eq!(result.max_vocabulary_size, 257); // UTF-8 baseline dominates (100 < 257)
    }

    #[test]
    fn test_analyze_ref_resolves_local_definition() {
        let mut analyzer = VocabularyAnalyzer::new();
        let values: Vec<Value> = (0..300).map(|i| json!(format!("status{}", i))).collect();
        let schema = json!({
            "$ref": "#/definitions/Status",
            "definitions": {
                "Status": {
                    "enum": values
                }
            }
        });

        let result = analyzer.analyze(&schema).unwrap();
        assert_eq!(result.max_structural_vocab, 300);
        assert_eq!(result.max_vocabulary_size, 300);
        assert_eq!(result.complexity_metrics.enum_count, 1);
        assert!(matches!(
            result.position_analysis.get("root").unwrap().data_type,
            DataType::Enum { .. }
        ));
    }

    #[test]
    fn test_analyze_ref_resolves_nested_and_indexed_paths() {
        let mut analyzer = VocabularyAnalyzer::new();
        let values: Vec<Value> = (0..300).map(|i| json!(format!("variant{}", i))).collect();
        let indexed_schema = json!({
            "$ref": "#/oneOf/1",
            "oneOf": [
                {"type": "boolean"},
                {"enum": values}
            ]
        });

        let indexed_result = analyzer.analyze(&indexed_schema).unwrap();
        assert_eq!(indexed_result.max_structural_vocab, 300);

        let nested_schema = json!({
            "$ref": "#/definitions/Envelope/properties/priority",
            "definitions": {
                "Envelope": {
                    "type": "object",
                    "properties": {
                        "priority": {
                            "type": "integer",
                            "minimum": 0,
                            "maximum": 300
                        }
                    }
                }
            }
        });

        let nested_result = analyzer.analyze(&nested_schema).unwrap();
        assert_eq!(nested_result.max_structural_vocab, 301);
        assert_eq!(
            nested_result
                .position_analysis
                .get("root")
                .unwrap()
                .encoding_strategy,
            EncodingStrategy::BoundedInteger { offset: 0 }
        );
    }

    #[test]
    fn test_analyze_ref_resolves_escaped_json_pointer_segments() {
        let mut analyzer = VocabularyAnalyzer::new();
        let schema = json!({
            "$ref": "#/definitions/status~1code~0range",
            "definitions": {
                "status/code~range": {
                    "type": "integer",
                    "minimum": 0,
                    "maximum": 300
                }
            }
        });

        let result = analyzer.analyze(&schema).unwrap();
        assert_eq!(result.max_structural_vocab, 301);
        assert_eq!(
            result
                .position_analysis
                .get("root")
                .unwrap()
                .encoding_strategy,
            EncodingStrategy::BoundedInteger { offset: 0 }
        );
    }

    #[test]
    fn test_analyze_ref_resolves_chained_local_references() {
        let mut analyzer = VocabularyAnalyzer::new();
        let schema = json!({
            "$ref": "#/definitions/A",
            "definitions": {
                "A": {"$ref": "#/definitions/B"},
                "B": {
                    "type": "integer",
                    "minimum": 0,
                    "maximum": 511
                }
            }
        });

        let result = analyzer.analyze(&schema).unwrap();
        assert_eq!(result.max_structural_vocab, 512);
        assert_eq!(
            result
                .position_analysis
                .get("root")
                .unwrap()
                .encoding_strategy,
            EncodingStrategy::BoundedInteger { offset: 0 }
        );
    }

    #[test]
    fn test_analyze_ref_nested_property_and_array_items_affect_root_vocab() {
        let mut analyzer = VocabularyAnalyzer::new();
        let values: Vec<Value> = (0..300).map(|i| json!(format!("status{}", i))).collect();
        let object_schema = json!({
            "type": "object",
            "properties": {
                "status": {"$ref": "#/definitions/Status"}
            },
            "definitions": {
                "Status": {"enum": values}
            }
        });

        let object_result = analyzer.analyze(&object_schema).unwrap();
        assert_eq!(object_result.max_structural_vocab, 300);
        assert_eq!(object_result.max_vocabulary_size, 300);

        let array_schema = json!({
            "type": "array",
            "items": {"$ref": "#/definitions/Score"},
            "definitions": {
                "Score": {
                    "type": "integer",
                    "minimum": 0,
                    "maximum": 511
                }
            }
        });

        let array_result = analyzer.analyze(&array_schema).unwrap();
        assert_eq!(array_result.max_structural_vocab, 512);
        assert_eq!(array_result.max_vocabulary_size, 512);
    }

    #[test]
    fn test_ref_estimates_match_inline_equivalents_for_representative_schemas() {
        const LEGACY_UNRESOLVED_REF_ESTIMATE: usize = 100;

        let enum_values: Vec<Value> = (0..300).map(|i| json!(format!("status{}", i))).collect();
        let cases = [
            (
                "definition enum",
                json!({
                    "$ref": "#/definitions/Status",
                    "definitions": {
                        "Status": {"enum": enum_values.clone()}
                    }
                }),
                json!({"enum": enum_values}),
                300,
            ),
            (
                "nested property bounded integer",
                json!({
                    "$ref": "#/definitions/Envelope/properties/priority",
                    "definitions": {
                        "Envelope": {
                            "type": "object",
                            "properties": {
                                "priority": {
                                    "type": "integer",
                                    "minimum": 0,
                                    "maximum": 300
                                }
                            }
                        }
                    }
                }),
                json!({"type": "integer", "minimum": 0, "maximum": 300}),
                301,
            ),
            (
                "array item bounded integer",
                json!({
                    "type": "array",
                    "items": {"$ref": "#/definitions/Score"},
                    "definitions": {
                        "Score": {
                            "type": "integer",
                            "minimum": 0,
                            "maximum": 511
                        }
                    }
                }),
                json!({
                    "type": "array",
                    "items": {
                        "type": "integer",
                        "minimum": 0,
                        "maximum": 511
                    }
                }),
                512,
            ),
            (
                "chained local bounded integer",
                json!({
                    "$ref": "#/definitions/A",
                    "definitions": {
                        "A": {"$ref": "#/definitions/B"},
                        "B": {
                            "type": "integer",
                            "minimum": 0,
                            "maximum": 511
                        }
                    }
                }),
                json!({"type": "integer", "minimum": 0, "maximum": 511}),
                512,
            ),
            (
                "escaped JSON Pointer bounded integer",
                json!({
                    "$ref": "#/definitions/status~1code~0range",
                    "definitions": {
                        "status/code~range": {
                            "type": "integer",
                            "minimum": 0,
                            "maximum": 300
                        }
                    }
                }),
                json!({"type": "integer", "minimum": 0, "maximum": 300}),
                301,
            ),
        ];

        for (name, ref_schema, inline_schema, expected_structural_vocab) in cases {
            let mut ref_analyzer = VocabularyAnalyzer::new();
            let ref_result = ref_analyzer.analyze(&ref_schema).unwrap();

            let mut inline_analyzer = VocabularyAnalyzer::new();
            let inline_result = inline_analyzer.analyze(&inline_schema).unwrap();

            assert_eq!(
                ref_result.max_structural_vocab, inline_result.max_structural_vocab,
                "{name}: local $ref estimate should match equivalent inline schema"
            );
            assert_eq!(
                ref_result.max_structural_vocab, expected_structural_vocab,
                "{name}: unexpected structural vocabulary estimate"
            );
            assert!(
                ref_result.max_structural_vocab > LEGACY_UNRESOLVED_REF_ESTIMATE,
                "{name}: representative schema should improve over the legacy unresolved-ref fallback"
            );
        }
    }

    #[test]
    fn test_analyze_ref_cycles_fall_back_without_overflow() {
        let mut analyzer = VocabularyAnalyzer::new();
        let schema = json!({
            "$ref": "#/definitions/A",
            "definitions": {
                "A": {"$ref": "#/definitions/B"},
                "B": {"$ref": "#/definitions/A"}
            }
        });

        let result = analyzer.analyze(&schema).unwrap();
        assert_eq!(result.max_structural_vocab, 100);
        assert_eq!(result.max_vocabulary_size, 257);
        assert!(matches!(
            result.position_analysis.get("root").unwrap().data_type,
            DataType::Unknown
        ));

        let recommendations = analyzer.generate_recommendations(&result, &[128, 257]);
        assert!(
            recommendations.warnings.iter().any(|warning| {
                warning.contains("Cyclic local $ref")
                    && warning.contains("#/definitions/A")
                    && warning.contains("root")
            }),
            "cyclic local refs should be surfaced in vocabulary-analysis warnings"
        );
    }

    #[test]
    fn test_analyze_ref_preserves_unresolved_reference_fallback() {
        let mut analyzer = VocabularyAnalyzer::new();
        let schema = json!({"$ref": "https://example.com/schema.json"});

        let result = analyzer.analyze(&schema).unwrap();
        assert_eq!(result.max_structural_vocab, 100);
        assert_eq!(result.max_vocabulary_size, 257);
        assert!(matches!(
            result.position_analysis.get("root").unwrap().data_type,
            DataType::Unknown
        ));
    }

    #[test]
    fn test_analyze_depth_limit_error() {
        let mut analyzer = VocabularyAnalyzer::new();
        analyzer.max_depth = 3; // Set very low limit

        // Create deeply nested schema
        let schema = json!({
            "type": "object",
            "properties": {
                "level1": {
                    "type": "object",
                    "properties": {
                        "level2": {
                            "type": "object",
                            "properties": {
                                "level3": {
                                    "type": "object",
                                    "properties": {
                                        "level4": {"type": "string"}
                                    }
                                }
                            }
                        }
                    }
                }
            }
        });

        let result = analyzer.analyze(&schema);
        assert!(result.is_err());
        if let Err(AnalysisError::ComputationError(msg)) = result {
            assert!(msg.contains("depth exceeds maximum"));
        }
    }

    // ========== Position Token Overhead Constructor ==========

    #[test]
    fn test_position_token_overhead_new() {
        let overhead = PositionTokenOverhead::new(1000, 256);
        assert_eq!(overhead.base_vocab_size, 1000);
        assert_eq!(overhead.schema_position_tokens, 256);
        assert_eq!(overhead.gap_position_tokens, 256);
        assert_eq!(overhead.total_vocab_size, 1000 + 2 * 256);
    }

    // ========== Encoding Strategy Coverage ==========

    #[test]
    fn test_encoding_strategy_direct() {
        let mut analyzer = VocabularyAnalyzer::new();
        let schema = json!({"type": "boolean"});
        let result = analyzer.analyze(&schema).unwrap();
        let pos = result.position_analysis.get("root").unwrap();
        assert_eq!(pos.encoding_strategy, EncodingStrategy::Direct);
    }

    #[test]
    fn test_encoding_strategy_index_encoding() {
        let mut analyzer = VocabularyAnalyzer::new();
        let schema = json!({"enum": ["a", "b", "c"]});
        let result = analyzer.analyze(&schema).unwrap();
        let pos = result.position_analysis.get("root").unwrap();
        assert_eq!(pos.encoding_strategy, EncodingStrategy::IndexEncoding);
    }

    #[test]
    fn test_encoding_strategy_bounded_integer() {
        let mut analyzer = VocabularyAnalyzer::new();
        let schema = json!({
            "type": "integer",
            "minimum": 10,
            "maximum": 100
        });
        let result = analyzer.analyze(&schema).unwrap();
        let pos = result.position_analysis.get("root").unwrap();
        assert_eq!(
            pos.encoding_strategy,
            EncodingStrategy::BoundedInteger { offset: 10 }
        );
    }

    #[test]
    fn test_encoding_strategy_multi_token_large_enum() {
        let mut analyzer = VocabularyAnalyzer::new();
        let values: Vec<Value> = (0..300).map(|i| json!(format!("value{}", i))).collect();
        let schema = json!({"enum": values});
        let result = analyzer.analyze(&schema).unwrap();
        let pos = result.position_analysis.get("root").unwrap();
        assert_eq!(
            pos.encoding_strategy,
            EncodingStrategy::MultiToken {
                tokens_per_value: 2
            }
        );
    }

    #[test]
    fn test_encoding_strategy_string_encoding() {
        let mut analyzer = VocabularyAnalyzer::new();
        let schema = json!({"type": "string"});
        let result = analyzer.analyze(&schema).unwrap();
        let pos = result.position_analysis.get("root").unwrap();
        assert_eq!(pos.encoding_strategy, EncodingStrategy::StringEncoding);
    }

    // ========== Optimal Vocabulary Size Tests ==========

    #[test]
    fn test_find_optimal_vocabulary_size_high_efficiency() {
        let analyzer = VocabularyAnalyzer::new();
        let efficiency_analysis = vec![
            VocabularyEfficiency {
                vocab_size: 128,
                max_required_vocab: 100,
                constrained_positions: 0,
                total_positions: 10,
                compression_ratio: 0.5,
                efficiency_score: 90.0,
                multi_token_encoding_needed: false,
            },
            VocabularyEfficiency {
                vocab_size: 256,
                max_required_vocab: 100,
                constrained_positions: 0,
                total_positions: 10,
                compression_ratio: 0.6,
                efficiency_score: 95.0,
                multi_token_encoding_needed: false,
            },
        ];

        let optimal = analyzer.find_optimal_vocabulary_size(&efficiency_analysis);
        assert_eq!(optimal, Some(128)); // Smallest with >=80% efficiency
    }

    #[test]
    fn test_find_optimal_vocabulary_size_low_efficiency_fallback() {
        let analyzer = VocabularyAnalyzer::new();
        let efficiency_analysis = vec![
            VocabularyEfficiency {
                vocab_size: 128,
                max_required_vocab: 1000,
                constrained_positions: 8,
                total_positions: 10,
                compression_ratio: 0.1,
                efficiency_score: 20.0,
                multi_token_encoding_needed: true,
            },
            VocabularyEfficiency {
                vocab_size: 512,
                max_required_vocab: 1000,
                constrained_positions: 5,
                total_positions: 10,
                compression_ratio: 0.3,
                efficiency_score: 50.0,
                multi_token_encoding_needed: true,
            },
        ];

        let optimal = analyzer.find_optimal_vocabulary_size(&efficiency_analysis);
        assert!(optimal.is_some()); // Should find best balance even if <80%
    }

    #[test]
    fn test_find_optimal_vocabulary_size_empty() {
        let analyzer = VocabularyAnalyzer::new();
        let efficiency_analysis = vec![];
        let optimal = analyzer.find_optimal_vocabulary_size(&efficiency_analysis);
        assert_eq!(optimal, None);
    }

    // ========== Compression Strategy Tests ==========

    #[test]
    fn test_compression_strategy_integer_optimization() {
        let mut analyzer = VocabularyAnalyzer::new();
        let schema = json!({
            "type": "object",
            "properties": {
                "count1": {"type": "integer", "minimum": 0, "maximum": 100},
                "count2": {"type": "integer", "minimum": 0, "maximum": 50}
            }
        });
        let analysis = analyzer.analyze(&schema).unwrap();
        let strategies = analyzer.generate_compression_strategies(&analysis);

        let has_integer_strategy = strategies
            .iter()
            .any(|s| s.strategy_type == "bounded_integer_optimization");
        assert!(has_integer_strategy);
    }

    #[test]
    fn test_compression_strategy_string_optimization() {
        let mut analyzer = VocabularyAnalyzer::new();
        let schema = json!({
            "type": "object",
            "properties": {
                "name": {"type": "string"},
                "title": {"type": "string"}
            }
        });
        let analysis = analyzer.analyze(&schema).unwrap();
        let strategies = analyzer.generate_compression_strategies(&analysis);

        let has_string_strategy = strategies
            .iter()
            .any(|s| s.strategy_type == "string_pattern_optimization");
        assert!(has_string_strategy);
    }

    #[test]
    fn test_compression_strategy_structural_optimization() {
        let mut analyzer = VocabularyAnalyzer::new();
        // Create deeply nested schema to trigger structural optimization
        let schema = json!({
            "type": "object",
            "properties": {
                "l1": {
                    "type": "object",
                    "properties": {
                        "l2": {
                            "type": "object",
                            "properties": {
                                "l3": {
                                    "type": "object",
                                    "properties": {
                                        "l4": {"type": "string"}
                                    }
                                }
                            }
                        }
                    }
                }
            }
        });
        let analysis = analyzer.analyze(&schema).unwrap();
        let strategies = analyzer.generate_compression_strategies(&analysis);

        let has_structural_strategy = strategies
            .iter()
            .any(|s| s.strategy_type == "structural_optimization");
        assert!(has_structural_strategy);
    }

    // ========== Warning Generation Tests ==========

    #[test]
    fn test_warning_large_vocabulary() {
        let mut analyzer = VocabularyAnalyzer::new();
        // Create schema with very large vocabulary
        let values: Vec<Value> = (0..15000).map(|i| json!(format!("v{}", i))).collect();
        let schema = json!({"enum": values});
        let analysis = analyzer.analyze(&schema).unwrap();
        let warnings = analyzer.generate_warnings(&analysis);

        let has_large_vocab_warning = warnings.iter().any(|w| w.contains("Very large vocabulary"));
        assert!(has_large_vocab_warning);
    }

    #[test]
    fn test_warning_deep_nesting() {
        let mut analyzer = VocabularyAnalyzer::new();
        // Create schema with deep nesting (>10 levels)
        let mut schema = json!({"type": "string"});
        for _ in 0..12 {
            schema = json!({
                "type": "object",
                "properties": {
                    "nested": schema
                }
            });
        }

        let analysis = analyzer.analyze(&schema).unwrap();
        let warnings = analyzer.generate_warnings(&analysis);

        let has_deep_nesting_warning = warnings.iter().any(|w| w.contains("Deep nesting"));
        assert!(has_deep_nesting_warning);
    }

    #[test]
    fn test_warning_high_complexity() {
        let mut analyzer = VocabularyAnalyzer::new();
        // Create highly complex schema
        let mut properties = serde_json::Map::new();
        for i in 0..20 {
            properties.insert(
                format!("enum{}", i),
                json!({"enum": ["a", "b", "c", "d", "e"]}),
            );
        }
        let schema = json!({
            "type": "object",
            "properties": properties
        });

        let analysis = analyzer.analyze(&schema).unwrap();
        let warnings = analyzer.generate_warnings(&analysis);

        // May trigger high complexity or large average vocabulary warning
        assert!(warnings.len() <= 10);
    }

    #[test]
    fn test_warning_large_average_vocabulary() {
        let mut analyzer = VocabularyAnalyzer::new();
        // Create single enum with very large vocabulary (>10000) to trigger both warnings
        let values: Vec<Value> = (0..12000).map(|i| json!(format!("value{}", i))).collect();
        let schema = json!({"enum": values});

        let analysis = analyzer.analyze(&schema).unwrap();

        // Should have large average vocabulary since there's only one position with 12000 values
        assert!(analysis.complexity_metrics.average_vocabulary_size > 1000.0);

        let warnings = analyzer.generate_warnings(&analysis);

        // Should trigger large vocabulary warning (and possibly average vocab warning)
        assert!(
            !warnings.is_empty(),
            "Should generate warnings for large vocabulary"
        );
    }

    // ========== count_data_types Tests ==========

    #[test]
    fn test_count_data_types_object_with_properties() {
        let analyzer = VocabularyAnalyzer::new();
        let mut type_counts = FxHashMap::default();

        let mut props = FxHashMap::default();
        props.insert(
            "name".to_string(),
            DataType::String {
                min_length: None,
                max_length: None,
            },
        );
        props.insert(
            "age".to_string(),
            DataType::Integer {
                minimum: None,
                maximum: None,
            },
        );

        let data_type = DataType::Object { properties: props };
        analyzer.count_data_types(&data_type, &mut type_counts);

        // Should count object + 2 properties
        assert!(type_counts.len() >= 2);
    }

    #[test]
    fn test_count_data_types_array() {
        let analyzer = VocabularyAnalyzer::new();
        let mut type_counts = FxHashMap::default();

        let data_type = DataType::Array {
            item_type: Box::new(DataType::Boolean),
        };
        analyzer.count_data_types(&data_type, &mut type_counts);

        assert!(type_counts.len() >= 2); // Array + Boolean
    }

    #[test]
    fn test_count_data_types_union() {
        let analyzer = VocabularyAnalyzer::new();
        let mut type_counts = FxHashMap::default();

        let data_type = DataType::Union {
            variants: vec![DataType::Boolean, DataType::Null, DataType::Number],
        };
        analyzer.count_data_types(&data_type, &mut type_counts);

        assert!(type_counts.len() >= 4); // Union + 3 variants
    }

    #[test]
    fn test_count_data_types_simple_types() {
        let analyzer = VocabularyAnalyzer::new();

        let types = vec![
            DataType::Boolean,
            DataType::Number,
            DataType::Null,
            DataType::Unknown,
        ];

        for dt in types {
            let mut type_counts = FxHashMap::default();
            analyzer.count_data_types(&dt, &mut type_counts);
            assert_eq!(type_counts.len(), 1);
        }
    }
}
