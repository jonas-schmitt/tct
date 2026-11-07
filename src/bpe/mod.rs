//! Byte Pair Encoding (BPE) compression for TCT token sequences
//!
//! This module provides runtime token compression using BPE merges trained
//! on schema-specific data. BPE compresses TCT's structured token sequences
//! by merging frequently co-occurring token pairs.
//!
//! Architecture:
//! - BPE merges are trained offline on instance data
//! - Merge tables are loaded from JSON files at compile-time
//! - Generated code embeds merge table as const data
//! - Encoding: Base tokens → Apply merges → BPE tokens
//! - Decoding: BPE tokens → Reverse merges → Base tokens
//!
//! Example merge file (schemas/bpe/github-workflow-tct-bpe-500.json):
//! ```json
//! {
//!   "schema": "github-workflow",
//!   "base_vocab_size": 80,
//!   "merges": [
//!     {"pair": [5, 12], "new_token": 80, "frequency": 45230},
//!     {"pair": [80, 7], "new_token": 81, "frequency": 38914}
//!   ],
//!   "metadata": {
//!     "trained_on_instances": 998,
//!     "training_date": "2025-10-05",
//!     "max_merges": 2048
//!   }
//! }
//! ```

use rustc_hash::{FxHashMap, FxHashSet};
use serde::{Deserialize, Serialize};
use std::error::Error;
use std::fmt;
use std::fs;
use std::path::Path;

pub mod encoder;
pub mod trainer;

/// Type alias for BPE encoding map (pair -> merged token)
pub type EncodingMap = FxHashMap<(u32, u32), u32>;

/// Type alias for BPE decoding map (merged token -> pair)
pub type DecodingMap = FxHashMap<u32, (u32, u32)>;

/// First token ID available for BPE merges.
///
/// BPE is trained directly on TCT base tokens. Legacy dictionary-trained BPE
/// tables are rejected because dictionary compression is no longer supported.
pub fn bpe_start_token(base_vocab_size: u32) -> u32 {
    base_vocab_size
}

/// A single BPE merge operation
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct BpeMerge {
    /// The token pair to merge [token1, token2]
    pub pair: [u32; 2],
    /// The new token ID that replaces the pair
    pub new_token: u32,
    /// Frequency of this pair in training data (for diagnostics)
    pub frequency: u64,
}

/// Metadata about BPE training
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct BpeMetadata {
    /// Number of instances used for training
    pub trained_on_instances: usize,
    /// Date when BPE was trained (ISO 8601 format)
    pub training_date: String,
    /// Maximum number of merges in this file
    pub max_merges: usize,
    /// Legacy dictionary size used during BPE training.
    ///
    /// Missing and zero values are accepted for compatibility with BPE-only
    /// artifacts. Nonzero values are rejected because dictionary compression is
    /// no longer supported.
    #[serde(default)] // For backward compatibility with old BPE files
    pub dict_size: Option<usize>,
}

/// Complete BPE merge table loaded from file
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct BpeMergeTable {
    /// Schema name (for verification)
    pub schema: String,
    /// Size of base vocabulary (before BPE)
    pub base_vocab_size: u32,
    /// Ordered list of merges (most frequent first)
    pub merges: Vec<BpeMerge>,
    /// Training metadata
    pub metadata: BpeMetadata,
}

impl BpeMergeTable {
    /// Load BPE merge table from JSON file
    ///
    /// # Arguments
    /// * `path` - Path to BPE merge JSON file
    ///
    /// # Returns
    /// Result containing the loaded merge table or error
    ///
    /// # Example
    /// ```no_run
    /// use tct::bpe::BpeMergeTable;
    ///
    /// let table = BpeMergeTable::from_file("schemas/bpe/github-workflow-tct-bpe-500.json")?;
    /// assert_eq!(table.schema, "github-workflow");
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn from_file<P: AsRef<Path>>(path: P) -> Result<Self, BpeError> {
        let content = fs::read_to_string(path.as_ref()).map_err(|e| BpeError::IoError {
            path: path.as_ref().to_path_buf(),
            source: e,
        })?;

        let table: BpeMergeTable =
            serde_json::from_str(&content).map_err(|e| BpeError::ParseError {
                path: path.as_ref().to_path_buf(),
                source: e,
            })?;

        // Validate merge table
        table.validate()?;

        Ok(table)
    }

    /// Validate merge table consistency.
    pub fn validate(&self) -> Result<(), BpeError> {
        if self.dict_size() > 0 {
            return Err(BpeError::InvalidMergeTable {
                reason: format!(
                    "BPE table has legacy dict_size {}; dictionary-trained BPE artifacts are no longer supported",
                    self.dict_size()
                ),
            });
        }

        let bpe_start = self.bpe_start();
        let mut seen_pairs =
            FxHashSet::with_capacity_and_hasher(self.merges.len(), Default::default());

        // Check that new_token IDs start after base vocabulary.
        for (i, merge) in self.merges.iter().enumerate() {
            let expected_token = bpe_start + i as u32;
            if merge.new_token != expected_token {
                return Err(BpeError::InvalidMergeTable {
                    reason: format!(
                        "Merge {} has new_token {} but expected {}",
                        i, merge.new_token, expected_token
                    ),
                });
            }

            let pair = (merge.pair[0], merge.pair[1]);
            if !seen_pairs.insert(pair) {
                return Err(BpeError::InvalidMergeTable {
                    reason: format!("Merge {} repeats pair ({}, {})", i, pair.0, pair.1),
                });
            }

            for token in merge.pair {
                if token >= expected_token {
                    return Err(BpeError::InvalidMergeTable {
                        reason: format!(
                            "Merge {} pair token {} must be a base token or earlier merge token below {}",
                            i, token, expected_token
                        ),
                    });
                }
            }
        }

        // Check metadata consistency
        if self.metadata.max_merges != self.merges.len() {
            return Err(BpeError::InvalidMergeTable {
                reason: format!(
                    "Metadata claims {} max_merges but table has {} merges",
                    self.metadata.max_merges,
                    self.merges.len()
                ),
            });
        }

        Ok(())
    }

    /// Legacy dictionary size from metadata, treating zero as no legacy dictionary.
    pub fn dict_size(&self) -> usize {
        self.metadata.dict_size.filter(|&d| d > 0).unwrap_or(0)
    }

    /// Token ID where BPE merges begin for this table.
    pub fn bpe_start(&self) -> u32 {
        bpe_start_token(self.base_vocab_size)
    }

    /// Get subset of merges up to max_vocab_size
    ///
    /// # Arguments
    /// * `max_vocab_size` - Maximum vocabulary size (base + BPE tokens)
    ///
    /// # Returns
    /// Slice of merges to apply
    ///
    /// # Example
    /// ```no_run
    /// # use tct::bpe::BpeMergeTable;
    /// # let table = BpeMergeTable::from_file("schemas/bpe/github-workflow-tct-bpe-500.json")?;
    /// // Use only first 256 merges (vocab = 80 + 256 = 336 tokens)
    /// let merges = table.get_merges(336);
    /// assert!(merges.len() <= 256);
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn get_merges(&self, max_vocab_size: u32) -> &[BpeMerge] {
        let bpe_start = self.bpe_start();
        if max_vocab_size <= bpe_start {
            // No BPE merges if max_vocab_size <= base vocab
            return &[];
        }

        let max_merges = (max_vocab_size - bpe_start) as usize;
        let actual_merges = max_merges.min(self.merges.len());
        &self.merges[..actual_merges]
    }

    /// Build lookup maps for efficient encoding/decoding
    ///
    /// # Arguments
    /// * `max_vocab_size` - Maximum vocabulary size
    ///
    /// # Returns
    /// Tuple of (pair_to_token map, token_to_pair map)
    pub fn build_lookup_maps(&self, max_vocab_size: u32) -> (EncodingMap, DecodingMap) {
        let merges = self.get_merges(max_vocab_size);

        let mut pair_to_token =
            FxHashMap::with_capacity_and_hasher(merges.len(), Default::default());
        let mut token_to_pair =
            FxHashMap::with_capacity_and_hasher(merges.len(), Default::default());

        for merge in merges {
            let pair = (merge.pair[0], merge.pair[1]);
            pair_to_token.insert(pair, merge.new_token);
            token_to_pair.insert(merge.new_token, pair);
        }

        (pair_to_token, token_to_pair)
    }
}

/// BPE-specific errors
#[derive(Debug)]
pub enum BpeError {
    IoError {
        path: std::path::PathBuf,
        source: std::io::Error,
    },

    ParseError {
        path: std::path::PathBuf,
        source: serde_json::Error,
    },

    InvalidMergeTable {
        reason: String,
    },
}

impl fmt::Display for BpeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            BpeError::IoError { path, source } => {
                write!(f, "Failed to read BPE merge file {:?}: {}", path, source)
            }
            BpeError::ParseError { path, source } => {
                write!(f, "Failed to parse BPE merge file {:?}: {}", path, source)
            }
            BpeError::InvalidMergeTable { reason } => {
                write!(f, "Invalid BPE merge table: {}", reason)
            }
        }
    }
}

impl Error for BpeError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            BpeError::IoError { source, .. } => Some(source),
            BpeError::ParseError { source, .. } => Some(source),
            BpeError::InvalidMergeTable { .. } => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_merge_table() -> BpeMergeTable {
        BpeMergeTable {
            schema: "test-schema".to_string(),
            base_vocab_size: 80,
            merges: vec![
                BpeMerge {
                    pair: [5, 12],
                    new_token: 80,
                    frequency: 1000,
                },
                BpeMerge {
                    pair: [80, 7],
                    new_token: 81,
                    frequency: 500,
                },
                BpeMerge {
                    pair: [3, 3],
                    new_token: 82,
                    frequency: 250,
                },
            ],
            metadata: BpeMetadata {
                trained_on_instances: 100,
                training_date: "2025-10-05".to_string(),
                max_merges: 3,
                dict_size: None,
            },
        }
    }

    #[test]
    fn test_merge_table_validation_success() {
        let table = create_test_merge_table();
        assert!(table.validate().is_ok());
    }

    #[test]
    fn test_merge_table_validation_wrong_token_id() {
        let mut table = create_test_merge_table();
        table.merges[1].new_token = 99; // Wrong ID
        assert!(table.validate().is_err());
    }

    #[test]
    fn test_merge_table_validation_rejects_duplicate_pairs() {
        let mut table = create_test_merge_table();
        table.merges[1].pair = table.merges[0].pair;

        let err = table.validate().unwrap_err();
        match err {
            BpeError::InvalidMergeTable { reason } => {
                assert!(reason.contains("repeats pair"));
            }
            _ => panic!("Expected InvalidMergeTable error"),
        }
    }

    #[test]
    fn test_merge_table_validation_rejects_future_pair_tokens() {
        let mut table = create_test_merge_table();
        table.merges[0].pair = [5, 80];

        let err = table.validate().unwrap_err();
        match err {
            BpeError::InvalidMergeTable { reason } => {
                assert!(reason.contains("base token or earlier merge token"));
            }
            _ => panic!("Expected InvalidMergeTable error"),
        }
    }

    #[test]
    fn test_merge_table_validation_metadata_mismatch() {
        let mut table = create_test_merge_table();
        table.metadata.max_merges = 10; // Doesn't match actual merges
        assert!(table.validate().is_err());
    }

    #[test]
    fn bpe_error_display_and_sources_use_manual_error_impls() {
        let invalid = BpeError::InvalidMergeTable {
            reason: "merge token 80 is missing".to_string(),
        };
        assert_eq!(
            invalid.to_string(),
            "Invalid BPE merge table: merge token 80 is missing"
        );
        assert!(std::error::Error::source(&invalid).is_none());

        let missing = BpeMergeTable::from_file("/nonexistent/path/bpe_merges.json")
            .expect_err("missing file should produce an IO error");
        assert!(
            missing
                .to_string()
                .contains("Failed to read BPE merge file")
        );
        assert!(std::error::Error::source(&missing).is_some());
    }

    #[test]
    fn test_get_merges_full() {
        let table = create_test_merge_table();
        let merges = table.get_merges(100); // 80 + 20 = plenty of room
        assert_eq!(merges.len(), 3); // All 3 merges
    }

    #[test]
    fn test_get_merges_limited() {
        let table = create_test_merge_table();
        let merges = table.get_merges(82); // 80 + 2 = only 2 merges
        assert_eq!(merges.len(), 2);
        assert_eq!(merges[0].new_token, 80);
        assert_eq!(merges[1].new_token, 81);
    }

    #[test]
    fn test_get_merges_none() {
        let table = create_test_merge_table();
        let merges = table.get_merges(80); // Exactly base vocab, no BPE
        assert_eq!(merges.len(), 0);
    }

    #[test]
    fn test_build_lookup_maps() {
        let table = create_test_merge_table();
        let (pair_to_token, token_to_pair) = table.build_lookup_maps(100);

        // Check encoding map
        assert_eq!(pair_to_token.get(&(5, 12)), Some(&80));
        assert_eq!(pair_to_token.get(&(80, 7)), Some(&81));
        assert_eq!(pair_to_token.get(&(3, 3)), Some(&82));

        // Check decoding map
        assert_eq!(token_to_pair.get(&80), Some(&(5, 12)));
        assert_eq!(token_to_pair.get(&81), Some(&(80, 7)));
        assert_eq!(token_to_pair.get(&82), Some(&(3, 3)));
    }

    // ============================================================================
    // Additional Tests for Error Paths and Edge Cases
    // ============================================================================

    #[test]
    fn test_from_file_missing_file() {
        // Test IoError path when file doesn't exist
        let result = BpeMergeTable::from_file("/nonexistent/path/bpe_merges.json");
        assert!(result.is_err());
        match result {
            Err(BpeError::IoError { path, source: _ }) => {
                assert!(path.to_string_lossy().contains("nonexistent"));
            }
            _ => panic!("Expected IoError"),
        }
    }

    #[test]
    fn test_from_file_invalid_json() {
        use std::io::Write;
        // Test ParseError path when JSON is invalid
        let temp_dir = std::env::temp_dir();
        let test_file = temp_dir.join("invalid_bpe.json");

        // Write invalid JSON
        {
            let mut file = std::fs::File::create(&test_file).unwrap();
            write!(file, "{{invalid json syntax").unwrap();
        }

        let result = BpeMergeTable::from_file(&test_file);
        assert!(result.is_err());
        match result {
            Err(BpeError::ParseError { path, source: _ }) => {
                assert!(path.to_string_lossy().contains("invalid_bpe"));
            }
            _ => panic!("Expected ParseError"),
        }

        // Cleanup
        let _ = std::fs::remove_file(&test_file);
    }

    #[test]
    fn test_from_file_validation_failure() {
        use std::io::Write;
        // Test that from_file validates merge table after parsing
        let temp_dir = std::env::temp_dir();
        let test_file = temp_dir.join("invalid_validation_bpe.json");

        // Create merge table with invalid token IDs
        let invalid_table = r#"{
            "schema": "test",
            "base_vocab_size": 80,
            "merges": [
                {"pair": [5, 12], "new_token": 80, "frequency": 1000},
                {"pair": [80, 7], "new_token": 999, "frequency": 500}
            ],
            "metadata": {
                "trained_on_instances": 10,
                "training_date": "2025-10-05",
                "max_merges": 2,
                "dict_size": null
            }
        }"#;

        {
            let mut file = std::fs::File::create(&test_file).unwrap();
            write!(file, "{}", invalid_table).unwrap();
        }

        let result = BpeMergeTable::from_file(&test_file);
        assert!(result.is_err());
        match result {
            Err(BpeError::InvalidMergeTable { reason }) => {
                assert!(reason.contains("new_token"));
                assert!(reason.contains("999"));
            }
            _ => panic!("Expected InvalidMergeTable error"),
        }

        // Cleanup
        let _ = std::fs::remove_file(&test_file);
    }

    #[test]
    fn test_validate_rejects_legacy_dict_size() {
        let table = BpeMergeTable {
            schema: "test".to_string(),
            base_vocab_size: 80,
            merges: vec![
                BpeMerge {
                    pair: [5, 12],
                    new_token: 1001,
                    frequency: 1000,
                },
                BpeMerge {
                    pair: [1001, 7],
                    new_token: 1002, // Next token
                    frequency: 500,
                },
            ],
            metadata: BpeMetadata {
                trained_on_instances: 100,
                training_date: "2025-10-05".to_string(),
                max_merges: 2,
                dict_size: Some(1000),
            },
        };

        let err = table.validate().unwrap_err();
        match err {
            BpeError::InvalidMergeTable { reason } => {
                assert!(reason.contains("legacy dict_size 1000"));
            }
            _ => panic!("Expected InvalidMergeTable error"),
        }
    }

    #[test]
    fn test_validate_with_dict_size_zero() {
        // Test that dict_size = 0 is treated as None
        let table = BpeMergeTable {
            schema: "test".to_string(),
            base_vocab_size: 80,
            merges: vec![BpeMerge {
                pair: [5, 12],
                new_token: 80, // Should start at base_vocab_size since dict_size=0
                frequency: 1000,
            }],
            metadata: BpeMetadata {
                trained_on_instances: 100,
                training_date: "2025-10-05".to_string(),
                max_merges: 1,
                dict_size: Some(0), // Treated as None
            },
        };

        assert!(table.validate().is_ok());
    }

    #[test]
    fn test_get_merges_below_base_vocab() {
        let table = create_test_merge_table();
        // Request vocab smaller than base vocab
        let merges = table.get_merges(50);
        assert_eq!(merges.len(), 0); // No merges should be returned
    }

    #[test]
    fn test_get_merges_exact_boundary() {
        let table = create_test_merge_table();
        // Request exactly base vocab + 1 merge
        let merges = table.get_merges(81); // 80 + 1
        assert_eq!(merges.len(), 1);
        assert_eq!(merges[0].new_token, 80);
    }

    #[test]
    fn test_build_lookup_maps_limited() {
        let table = create_test_merge_table();
        // Build maps with limited vocab size
        let (pair_to_token, token_to_pair) = table.build_lookup_maps(81);

        // Should only have first merge
        assert_eq!(pair_to_token.len(), 1);
        assert_eq!(token_to_pair.len(), 1);
        assert_eq!(pair_to_token.get(&(5, 12)), Some(&80));
        assert_eq!(token_to_pair.get(&80), Some(&(5, 12)));
    }

    #[test]
    fn test_get_merges_uses_base_vocab_start() {
        let table = BpeMergeTable {
            schema: "test".to_string(),
            base_vocab_size: 80,
            merges: vec![
                BpeMerge {
                    pair: [5, 12],
                    new_token: 80,
                    frequency: 1000,
                },
                BpeMerge {
                    pair: [80, 7],
                    new_token: 81,
                    frequency: 500,
                },
            ],
            metadata: BpeMetadata {
                trained_on_instances: 100,
                training_date: "2025-10-05".to_string(),
                max_merges: 2,
                dict_size: None,
            },
        };

        assert_eq!(table.get_merges(80).len(), 0);
        assert_eq!(table.get_merges(81).len(), 1);
        assert_eq!(table.get_merges(82).len(), 2);
    }

    #[test]
    fn test_serialization_round_trip() {
        let table = create_test_merge_table();

        // Serialize to JSON
        let json = serde_json::to_string_pretty(&table).unwrap();

        // Deserialize back
        let deserialized: BpeMergeTable = serde_json::from_str(&json).unwrap();

        assert_eq!(table, deserialized);
    }

    #[test]
    fn test_dict_size_backward_compatibility() {
        // Test that old BPE files without dict_size field deserialize correctly
        let json_without_dict_size = r#"{
            "schema": "test",
            "base_vocab_size": 80,
            "merges": [
                {"pair": [5, 12], "new_token": 80, "frequency": 1000}
            ],
            "metadata": {
                "trained_on_instances": 100,
                "training_date": "2025-10-05",
                "max_merges": 1
            }
        }"#;

        let table: BpeMergeTable = serde_json::from_str(json_without_dict_size).unwrap();
        assert_eq!(table.metadata.dict_size, None);
        assert!(table.validate().is_ok());
    }

    #[test]
    fn test_validate_rejects_legacy_dict_size_before_token_layout() {
        let table = BpeMergeTable {
            schema: "test".to_string(),
            base_vocab_size: 80,
            merges: vec![BpeMerge {
                pair: [5, 12],
                new_token: 80,
                frequency: 1000,
            }],
            metadata: BpeMetadata {
                trained_on_instances: 100,
                training_date: "2025-10-05".to_string(),
                max_merges: 1,
                dict_size: Some(1000),
            },
        };

        let result = table.validate();
        assert!(result.is_err());
        match result {
            Err(BpeError::InvalidMergeTable { reason }) => {
                assert!(reason.contains("legacy dict_size 1000"));
            }
            _ => panic!("Expected InvalidMergeTable error"),
        }
    }
}
