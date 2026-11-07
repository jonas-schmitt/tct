//! Focused snapshots for generated Rust output.
//!
//! These tests complement semantic generated-crate round trips by making small
//! emitted-code contracts reviewable without broad substring assertions.

use std::io::Write;

use expect_test::expect;
use serde_json::json;
use tct::bpe::{BpeMerge, BpeMergeTable, BpeMetadata};
use tct::{OptimizationLevel, TctCompiler};

#[test]
fn root_string_enum_generated_output_snapshot() {
    let schema = json!({
        "title": "SnapshotEnum",
        "type": "string",
        "enum": ["guest", "admin"]
    });

    let generated_code = TctCompiler::new(schema, "SnapshotEnum")
        .expect("snapshot schema should initialize")
        .with_optimization_level(OptimizationLevel::O3)
        .compile()
        .expect("snapshot schema should compile")
        .generate_code()
        .expect("snapshot schema should render");

    let snapshot = extract_generated_type_encoder_snapshot(&generated_code, "SnapshotEnum");
    expect![[r##"
        #[derive(
            Debug,
            PartialEq,
            Eq,
            Clone,
            PartialOrd,
            Ord,
            Hash,
            serde::Serialize,
            serde::Deserialize
        )]
        pub enum SnapshotEnum {
            #[serde(rename = "guest")]
            Guest,
            #[serde(rename = "admin")]
            Admin,
        }
        impl crate::tokenizer::JsonTokenEncoder for SnapshotEnum {
            fn encode(&self, tokens: &mut Vec<u32>) {
                match self {
                    Self::Guest => tokens.push(0u32),
                    Self::Admin => tokens.push(1u32),
                }
            }
            fn decode(
                tokens: &mut impl Iterator<Item = u32>,
            ) -> Result<Self, crate::tokenizer::DecodeError> {
                let variant_index = tokens
                    .next()
                    .ok_or(crate::tokenizer::DecodeError::UnexpectedEndOfStream {
                        context: crate::tokenizer::EOFContext::new(
                            "SnapshotEnum",
                            Some("variant_index"),
                            "enum variant index",
                        ),
                    })?;
                crate::tokenizer::role_export::mark_discriminator();
                let variant_index = variant_index % 2u32;
                match variant_index {
                    0u32 => Ok(Self::Guest),
                    1u32 => Ok(Self::Admin),
                    _ => unreachable!("Modulo operation ensures valid token range"),
                }
            }
            fn decode_strict(
                tokens: &mut impl Iterator<Item = u32>,
            ) -> Result<Self, crate::tokenizer::DecodeError> {
                let variant_index = tokens
                    .next()
                    .ok_or(crate::tokenizer::DecodeError::UnexpectedEndOfStream {
                        context: crate::tokenizer::EOFContext::new(
                            "SnapshotEnum",
                            Some("variant_index"),
                            "enum variant index",
                        ),
                    })?;
                match variant_index {
                    0u32 => Ok(Self::Guest),
                    1u32 => Ok(Self::Admin),
                    _ => {
                        Err(crate::tokenizer::DecodeError::InvalidTokenInContext {
                            token: variant_index,
                            context: crate::tokenizer::EOFContext::new(
                                "SnapshotEnum",
                                Some("variant_index"),
                                "enum variant index in range",
                            ),
                        })
                    }
                }
            }
            fn token_count(&self) -> usize {
                match self {
                    Self::Guest => 1,
                    Self::Admin => 1,
                }
            }
        }
    "##]]
    .assert_eq(&snapshot);
}

#[test]
fn required_optional_struct_generated_output_snapshot() {
    let schema = json!({
        "type": "object",
        "additionalProperties": false,
        "properties": {
            "name": { "type": "string" },
            "age": { "type": "integer" }
        },
        "required": ["name"]
    });

    let generated_code = TctCompiler::new(schema, "SnapshotPerson")
        .expect("snapshot schema should initialize")
        .with_optimization_level(OptimizationLevel::O3)
        .compile()
        .expect("snapshot schema should compile")
        .generate_code()
        .expect("snapshot schema should render");

    let snapshot = extract_generated_type_encoder_snapshot(&generated_code, "SnapshotPerson");
    expect![[r##"
        #[derive(
            Debug,
            PartialEq,
            Eq,
            Clone,
            PartialOrd,
            Ord,
            Hash,
            serde::Serialize,
            serde::Deserialize
        )]
        #[serde(deny_unknown_fields)]
        pub struct SnapshotPerson {
            #[serde(rename = "name")]
            pub name: String,
            #[serde(
                rename = "age",
                default,
                skip_serializing_if = "Option::is_none",
                deserialize_with = "crate::tokenizer::deserialize_option_reject_null"
            )]
            pub age: Option<crate::tokenizer::JsonInteger>,
        }
        impl crate::tokenizer::JsonTokenEncoder for SnapshotPerson {
            fn encode(&self, tokens: &mut Vec<u32>) {
                self.name.encode(tokens);
                if let Some(ref value) = self.age {
                    tokens.push(1u32);
                    value.encode(tokens);
                } else {
                    tokens.push(0u32);
                }
            }
            fn decode(
                tokens: &mut impl Iterator<Item = u32>,
            ) -> Result<Self, crate::tokenizer::DecodeError> {
                let name = crate::tokenizer::terminal_key_export::with_object_property(
                    "name",
                    || {
                        crate::tokenizer::with_schema_field_decode_context(
                            crate::tokenizer::JsonTokenEncoder::decode(tokens),
                            "SnapshotPerson",
                            "name",
                        )
                    },
                )?;
                let age = match tokens.next() {
                    Some(presence_token) => {
                        if presence_token % 2 == 1 {
                            Some(
                                crate::tokenizer::terminal_key_export::with_object_property(
                                    "age",
                                    || {
                                        crate::tokenizer::with_schema_field_decode_context(
                                            crate::tokenizer::JsonTokenEncoder::decode(tokens),
                                            "SnapshotPerson",
                                            "age",
                                        )
                                    },
                                )?,
                            )
                        } else {
                            None
                        }
                    }
                    None => {
                        return Err(crate::tokenizer::DecodeError::UnexpectedEndOfStream {
                            context: crate::tokenizer::EOFContext::new(
                                "SnapshotPerson",
                                Some("age"),
                                "presence flag",
                            ),
                        });
                    }
                };
                let decoded = Self { name, age };
                Ok(decoded)
            }
            fn decode_strict(
                tokens: &mut impl Iterator<Item = u32>,
            ) -> Result<Self, crate::tokenizer::DecodeError> {
                let name = crate::tokenizer::terminal_key_export::with_object_property(
                    "name",
                    || {
                        crate::tokenizer::with_schema_field_decode_context(
                            crate::tokenizer::JsonTokenEncoder::decode_strict(tokens),
                            "SnapshotPerson",
                            "name",
                        )
                    },
                )?;
                let age = match tokens.next() {
                    Some(1) => {
                        Some(
                            crate::tokenizer::terminal_key_export::with_object_property(
                                "age",
                                || {
                                    crate::tokenizer::with_schema_field_decode_context(
                                        crate::tokenizer::JsonTokenEncoder::decode_strict(tokens),
                                        "SnapshotPerson",
                                        "age",
                                    )
                                },
                            )?,
                        )
                    }
                    Some(0) => None,
                    Some(token) => {
                        return Err(crate::tokenizer::DecodeError::InvalidTokenInContext {
                            token,
                            context: crate::tokenizer::EOFContext::new(
                                "SnapshotPerson",
                                Some("age"),
                                "presence flag 0 or 1",
                            ),
                        });
                    }
                    None => {
                        return Err(crate::tokenizer::DecodeError::UnexpectedEndOfStream {
                            context: crate::tokenizer::EOFContext::new(
                                "SnapshotPerson",
                                Some("age"),
                                "presence flag",
                            ),
                        });
                    }
                };
                let decoded = Self { name, age };
                Ok(decoded)
            }
            fn token_count(&self) -> usize {
                0 + self.name.token_count() + 1
                    + self.age.as_ref().map_or(0, |v| v.token_count())
            }
        }
    "##]]
    .assert_eq(&snapshot);
}

#[test]
fn unconstrained_additional_properties_generated_output_snapshot() {
    let schema = json!({
        "type": "object",
        "properties": {
            "name": { "type": "string" }
        },
        "additionalProperties": true
    });

    let generated_code = TctCompiler::new(schema, "SnapshotOpenObject")
        .expect("snapshot schema should initialize")
        .with_optimization_level(OptimizationLevel::O3)
        .compile()
        .expect("snapshot schema should compile")
        .generate_code()
        .expect("snapshot schema should render");

    let snapshot = extract_generated_type_encoder_snapshot(&generated_code, "SnapshotOpenObject");
    expect![[r##"
        #[derive(Debug, PartialEq, Clone, serde::Serialize, serde::Deserialize)]
        pub struct SnapshotOpenObject {
            #[serde(
                rename = "name",
                default,
                skip_serializing_if = "Option::is_none",
                deserialize_with = "crate::tokenizer::deserialize_option_reject_null"
            )]
            pub name: Option<String>,
            #[serde(flatten)]
            pub additional_properties: rustc_hash::FxHashMap<String, serde_json::Value>,
        }
        impl crate::tokenizer::JsonTokenEncoder for SnapshotOpenObject {
            fn encode(&self, tokens: &mut Vec<u32>) {
                if let Some(ref value) = self.name {
                    tokens.push(1u32);
                    value.encode(tokens);
                } else {
                    tokens.push(0u32);
                }
                self.additional_properties.encode(tokens);
            }
            fn decode(
                tokens: &mut impl Iterator<Item = u32>,
            ) -> Result<Self, crate::tokenizer::DecodeError> {
                let name = match tokens.next() {
                    Some(presence_token) => {
                        if presence_token % 2 == 1 {
                            Some(
                                crate::tokenizer::terminal_key_export::with_object_property(
                                    "name",
                                    || {
                                        crate::tokenizer::with_schema_field_decode_context(
                                            crate::tokenizer::JsonTokenEncoder::decode(tokens),
                                            "SnapshotOpenObject",
                                            "name",
                                        )
                                    },
                                )?,
                            )
                        } else {
                            None
                        }
                    }
                    None => {
                        return Err(crate::tokenizer::DecodeError::UnexpectedEndOfStream {
                            context: crate::tokenizer::EOFContext::new(
                                "SnapshotOpenObject",
                                Some("name"),
                                "presence flag",
                            ),
                        });
                    }
                };
                let additional_properties = crate::tokenizer::terminal_key_export::with_object_property(
                    "additional_properties",
                    || {
                        crate::tokenizer::with_schema_field_decode_context(
                            crate::tokenizer::JsonTokenEncoder::decode(tokens),
                            "SnapshotOpenObject",
                            "additional_properties",
                        )
                    },
                )?;
                let decoded = Self {
                    name,
                    additional_properties,
                };
                Ok(decoded)
            }
            fn decode_strict(
                tokens: &mut impl Iterator<Item = u32>,
            ) -> Result<Self, crate::tokenizer::DecodeError> {
                let name = match tokens.next() {
                    Some(1) => {
                        Some(
                            crate::tokenizer::terminal_key_export::with_object_property(
                                "name",
                                || {
                                    crate::tokenizer::with_schema_field_decode_context(
                                        crate::tokenizer::JsonTokenEncoder::decode_strict(tokens),
                                        "SnapshotOpenObject",
                                        "name",
                                    )
                                },
                            )?,
                        )
                    }
                    Some(0) => None,
                    Some(token) => {
                        return Err(crate::tokenizer::DecodeError::InvalidTokenInContext {
                            token,
                            context: crate::tokenizer::EOFContext::new(
                                "SnapshotOpenObject",
                                Some("name"),
                                "presence flag 0 or 1",
                            ),
                        });
                    }
                    None => {
                        return Err(crate::tokenizer::DecodeError::UnexpectedEndOfStream {
                            context: crate::tokenizer::EOFContext::new(
                                "SnapshotOpenObject",
                                Some("name"),
                                "presence flag",
                            ),
                        });
                    }
                };
                let additional_properties = crate::tokenizer::terminal_key_export::with_object_property(
                    "additional_properties",
                    || {
                        crate::tokenizer::with_schema_field_decode_context(
                            crate::tokenizer::JsonTokenEncoder::decode_strict(tokens),
                            "SnapshotOpenObject",
                            "additional_properties",
                        )
                    },
                )?;
                let decoded = Self {
                    name,
                    additional_properties,
                };
                Ok(decoded)
            }
            fn token_count(&self) -> usize {
                0 + 1 + self.name.as_ref().map_or(0, |v| v.token_count())
                    + self.additional_properties.token_count()
            }
        }
    "##]]
    .assert_eq(&snapshot);
}

#[test]
fn bounded_integer_struct_generated_output_snapshot() {
    let schema = json!({
        "type": "object",
        "additionalProperties": false,
        "properties": {
            "score": {
                "type": "integer",
                "minimum": -5,
                "maximum": 5
            }
        },
        "required": ["score"]
    });

    let generated_code = TctCompiler::new(schema, "SnapshotScore")
        .expect("snapshot schema should initialize")
        .with_optimization_level(OptimizationLevel::O3)
        .compile()
        .expect("snapshot schema should compile")
        .generate_code()
        .expect("snapshot schema should render");

    let snapshot = extract_generated_type_encoder_snapshot(&generated_code, "SnapshotScoreScore");
    expect![[r#"
#[derive(
    Debug,
    Clone,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Hash,
    serde::Serialize,
    serde::Deserialize
)]
#[serde(transparent)]
pub struct SnapshotScoreScore(pub i64);
impl SnapshotScoreScore {
    /// Get the inner value
    pub fn value(&self) -> &i64 {
        &self.0
    }
    /// Get the inner value (consuming)
    pub fn into_value(self) -> i64 {
        self.0
    }
    /// Create from inner value
    pub fn new(value: i64) -> Self {
        Self(value)
    }
}
impl crate::tokenizer::JsonTokenEncoder for SnapshotScoreScore {
    fn encode(&self, tokens: &mut Vec<u32>) {
        const ENCODING_BASE: u32 = 256u32;
        let offset = (self.0 - -5i64) as u32;
        if offset < 256u32 {
            tokens.push(0u32);
            tokens.push(offset % ENCODING_BASE);
        } else {
            if offset < 65536u32 {
                tokens.push(1u32);
                tokens.push((offset / 256u32) % ENCODING_BASE);
                tokens.push(offset % ENCODING_BASE);
            } else {
                if offset < 16777216u32 {
                    tokens.push(2u32);
                    tokens.push((offset / 65536u32) % ENCODING_BASE);
                    tokens.push((offset / 256u32) % ENCODING_BASE);
                    tokens.push(offset % ENCODING_BASE);
                } else {
                    tokens.push(3u32);
                    tokens.push((offset / 16777216u32) % ENCODING_BASE);
                    tokens.push((offset / 65536u32) % ENCODING_BASE);
                    tokens.push((offset / 256u32) % ENCODING_BASE);
                    tokens.push(offset % ENCODING_BASE);
                }
            }
        }
    }
    fn decode(
        tokens: &mut impl Iterator<Item = u32>,
    ) -> Result<Self, crate::tokenizer::DecodeError> {
        let offset = crate::tokenizer::variable_base::decode_u32_fixed_base(
            tokens,
            stringify!(SnapshotScoreScore),
        )?;
        let value = (offset as i64) + -5i64;
        Ok(Self(value))
    }
    fn decode_strict(
        tokens: &mut impl Iterator<Item = u32>,
    ) -> Result<Self, crate::tokenizer::DecodeError> {
        let offset = crate::tokenizer::variable_base::decode_u32_fixed_base_strict(
            tokens,
            stringify!(SnapshotScoreScore),
        )?;
        if offset >= 11u32 {
            return Err(
                crate::tokenizer::DecodeError::InvalidData(
                    format!(
                        "bounded integer offset {} exceeds schema range 0..{}", offset,
                        11u32.saturating_sub(1)
                    ),
                ),
            );
        }
        let value = (offset as i64) + -5i64;
        Ok(Self(value))
    }
    fn token_count(&self) -> usize {
        let offset = (self.0 - -5i64) as u32;
        if offset < 256u32 {
            2usize
        } else {
            if offset < 65536u32 {
                3usize
            } else {
                if offset < 16777216u32 { 4usize } else { 5usize }
            }
        }
    }
}
"#]]
    .assert_eq(&snapshot);
}

#[test]
fn bpe_module_header_generated_output_snapshot() {
    let schema = json!({"type": "string"});
    let bpe_file = write_snapshot_bpe_merge_file();

    let generated_code = TctCompiler::new(schema, "SnapshotBpeString")
        .expect("snapshot schema should initialize")
        .with_bpe_merges(bpe_file.path())
        .expect("snapshot BPE merge file should load")
        .compile()
        .expect("snapshot schema should compile")
        .generate_code()
        .expect("snapshot schema should render");

    let snapshot = extract_bpe_module_header_snapshot(&generated_code);
    expect![[r#"
pub mod bpe {
    use rustc_hash::FxHashMap;
    /// BPE merge table embedded at compile time
    const BASE_VOCAB_SIZE: u32 = 257u32;
    /// Token ID where BPE merges begin (after the base range)
    const BPE_START: u32 = 257u32;
    const SCHEMA_NAME: &str = "snapshot-bpe";
    const MAX_MERGES: usize = 1usize;
    /// Sentinel used in linked-position arrays for "no previous" or "no next".
    ///
    /// Generated BPE encoding stores live positions as `u32` indices. Inputs
    /// too large for `u32` indices use the full-scan fallback, so this value
    /// cannot collide with a valid linked-list position.
    const NO_LINK: u32 = u32::MAX;
    /// Mask for the low 32 bits of packed initial-pair candidates.
    ///
    /// Initial candidates are packed as `(merge_idx << 32) | left_position`.
    /// Sorting the packed values groups candidates by merge priority first
    /// while preserving the linked-list left position in the low bits.
    const COMPACT_CANDIDATE_LEFT_MASK: u64 = u32::MAX as u64;
    /// Mutable linked-token index used while applying priority-ordered merges.
    ///
    /// `next` and `prev` use `NO_LINK` as their sentinel. `alive` lets the
    /// encoder reject stale positions after adjacent merges rewrite the linked
    /// list, and `future_positions` holds new boundary candidates keyed by the
    /// later merge that can consume them.
    struct PairIndexState<'a> {
        current: &'a mut [u32],
        next: &'a mut [u32],
        prev: &'a mut [u32],
        alive: &'a mut [bool],
        future_positions: &'a mut FxHashMap<u32, PositionBucket>,
        position_pool: &'a mut Vec<Vec<u32>>,
    }
    /// Pending linked-list positions for a future merge.
    ///
    /// Buckets stay in the allocation-free `One` state for the common case.
    /// They upgrade to `Many` only when multiple boundary candidates target
    /// the same later merge; the vector is sorted, deduplicated, and recycled
    /// through `position_pool` before merge application.
    enum PositionBucket {
        One(u32),
        Many(Vec<u32>),
    }
    /// Merge pairs: [(left_token, right_token), ...]
    const MERGE_PAIRS: [(u32, u32); 1usize] = [(97u32, 98u32)];
    /// New tokens corresponding to each merge
    const MERGE_TOKENS: [u32; 1usize] = [257u32];
    /// Sorted pair-key to merge-index lookup table.
    ///
    /// Pair keys are `(left << 32) | right`. The table is sorted by
    /// pair key so generated tokenizers can look up merge indices
    /// without building a runtime hash map.
    const MERGE_PAIR_INDEX: [(u64, usize); 1usize] = [(416611827810u64, 0usize)];
    /// Exact iterative decode-step budget for each BPE merge token.
    ///
    /// Index `i` corresponds to token `BPE_START + i`. The values are
    /// generated from the merge table, avoiding a first-decode cache
    /// allocation in generated tokenizers.
    const DECODE_STEP_BUDGETS: [usize; 1usize] = [3usize];
"#]]
    .assert_eq(&snapshot);
}

fn extract_generated_type_encoder_snapshot(code: &str, type_name: &str) -> String {
    let type_pos = find_type_definition(code, "enum", type_name)
        .or_else(|| find_type_definition(code, "struct", type_name))
        .unwrap_or_else(|| panic!("generated code should contain {type_name}"));
    let type_start = code[..type_pos].rfind("#[derive(").unwrap_or(type_pos);

    let impl_marker = format!("impl crate::tokenizer::JsonTokenEncoder for {type_name}");
    let impl_pos = code
        .find(&impl_marker)
        .unwrap_or_else(|| panic!("generated code should contain {impl_marker}"));
    let impl_end = find_braced_item_end(code, impl_pos);

    format!("{}\n", code[type_start..impl_end].trim())
}

fn write_snapshot_bpe_merge_file() -> tempfile::NamedTempFile {
    let mut file = tempfile::NamedTempFile::new().expect("snapshot BPE temp file should open");
    let table = BpeMergeTable {
        schema: "snapshot-bpe".to_string(),
        base_vocab_size: 257,
        merges: vec![BpeMerge {
            pair: [97, 98],
            new_token: 257,
            frequency: 42,
        }],
        metadata: BpeMetadata {
            trained_on_instances: 1,
            training_date: "2026-05-03".to_string(),
            max_merges: 1,
            dict_size: None,
        },
    };
    serde_json::to_writer_pretty(&mut file, &table)
        .expect("snapshot BPE merge table should serialize");
    file.flush().expect("snapshot BPE temp file should flush");
    file
}

fn extract_bpe_module_header_snapshot(code: &str) -> String {
    let module_pos = code
        .find("pub mod bpe {")
        .expect("generated code should contain a BPE module");
    let encode_doc_pos = code[module_pos..]
        .find("    /// Encode base tokens")
        .map(|pos| module_pos + pos)
        .expect("generated BPE module should contain encode docs");

    format!("{}\n", code[module_pos..encode_doc_pos].trim())
}

fn find_type_definition(code: &str, kind: &str, type_name: &str) -> Option<usize> {
    let marker = format!("pub {kind} {type_name}");
    let mut search_start = 0usize;
    while let Some(relative_pos) = code[search_start..].find(&marker) {
        let pos = search_start + relative_pos;
        let next = code[pos + marker.len()..].chars().next();
        if next.is_some_and(is_type_name_delimiter) {
            return Some(pos);
        }
        search_start = pos + marker.len();
    }
    None
}

fn is_type_name_delimiter(ch: char) -> bool {
    ch.is_whitespace() || matches!(ch, '{' | '(' | '<' | ';')
}

fn find_braced_item_end(code: &str, item_start: usize) -> usize {
    let mut depth = 0usize;
    let mut saw_open_brace = false;
    for (offset, ch) in code[item_start..].char_indices() {
        match ch {
            '{' => {
                depth += 1;
                saw_open_brace = true;
            }
            '}' => {
                depth = depth
                    .checked_sub(1)
                    .expect("generated item should not close before opening");
                if saw_open_brace && depth == 0 {
                    return item_start + offset + ch.len_utf8();
                }
            }
            _ => {}
        }
    }
    panic!("generated item starting at byte {item_start} was not closed");
}
