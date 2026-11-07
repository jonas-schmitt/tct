//! Python bindings of the root `tct` module.
//!
//! The module exposes the schema compiler (`tct.compile`, `tct.analyze`, from
//! the `compile_api` submodule) and the BPE encoder/decoder classes for
//! efficient tokenization from Python with streaming decode support.
//!
//! With the `experimental` feature the module also registers the SQL
//! schema-linking runtime bindings from the `sql_runtime` submodule.

use pyo3::prelude::*;
use pyo3::types::PyBytes;

use crate::bpe::{BpeMergeTable, encoder::BpeDecoder, encoder::SequentialBpeEncoder};

/// BPE encoder for UTF-8 byte sequences
#[pyclass]
pub struct Utf8BpeEncoder {
    encoder: SequentialBpeEncoder,
    base_vocab_size: u32,
}

#[pymethods]
impl Utf8BpeEncoder {
    /// Create a new UTF-8 BPE encoder from a merge table file
    #[new]
    fn new(merge_table_path: &str) -> PyResult<Self> {
        let table = BpeMergeTable::from_file(merge_table_path).map_err(|e| {
            pyo3::exceptions::PyIOError::new_err(format!("Failed to load merge table: {}", e))
        })?;
        let vocab_size = table.bpe_start() + table.merges.len() as u32;
        let encoder = SequentialBpeEncoder::new(&table, vocab_size);
        Ok(Self {
            encoder,
            base_vocab_size: table.base_vocab_size,
        })
    }

    /// Encode a UTF-8 string to BPE tokens
    fn encode(&self, py: Python<'_>, text: &str) -> Vec<u32> {
        let bytes: Vec<u32> = text.bytes().map(|b| b as u32).collect();
        py.detach(move || self.encoder.encode(&bytes))
    }

    /// Encode raw bytes to BPE tokens
    fn encode_bytes(&self, py: Python<'_>, data: &[u8]) -> Vec<u32> {
        let tokens: Vec<u32> = data.iter().map(|&b| b as u32).collect();
        py.detach(move || self.encoder.encode(&tokens))
    }

    /// Get the base vocabulary size (256 for UTF-8 byte encoding)
    fn base_vocab_size(&self) -> u32 {
        self.base_vocab_size
    }
}

/// BPE decoder for UTF-8 byte sequences with streaming support
#[pyclass]
pub struct Utf8BpeDecoder {
    decoder: BpeDecoder,
    /// Buffer for partial UTF-8 sequences during streaming decode
    partial_bytes: Vec<u8>,
}

#[pymethods]
impl Utf8BpeDecoder {
    /// Create a new UTF-8 BPE decoder from a merge table file
    #[new]
    fn new(merge_table_path: &str) -> PyResult<Self> {
        let table = BpeMergeTable::from_file(merge_table_path).map_err(|e| {
            pyo3::exceptions::PyIOError::new_err(format!("Failed to load merge table: {}", e))
        })?;
        let vocab_size = table.bpe_start() + table.merges.len() as u32;
        let decoder = BpeDecoder::new(&table, vocab_size);
        Ok(Self {
            decoder,
            partial_bytes: Vec::new(),
        })
    }

    /// Decode BPE tokens to a UTF-8 string
    ///
    /// Returns the decoded string. Invalid UTF-8 sequences are replaced with
    /// the Unicode replacement character (U+FFFD).
    fn decode(&self, py: Python<'_>, tokens: Vec<u32>) -> String {
        let base_tokens = py.detach(move || self.decoder.decode(&tokens));
        let bytes: Vec<u8> = base_tokens.iter().map(|&t| (t & 0xFF) as u8).collect();
        String::from_utf8_lossy(&bytes).into_owned()
    }

    /// Decode BPE tokens to raw bytes
    fn decode_bytes<'py>(&self, py: Python<'py>, tokens: Vec<u32>) -> Bound<'py, PyBytes> {
        let base_tokens = py.detach(move || self.decoder.decode(&tokens));
        let bytes: Vec<u8> = base_tokens.iter().map(|&t| (t & 0xFF) as u8).collect();
        PyBytes::new(py, &bytes)
    }

    /// Streaming decode: decode tokens and return complete UTF-8 characters
    ///
    /// This method buffers partial UTF-8 sequences across calls, returning only
    /// complete characters. Use `flush()` to get any remaining buffered bytes.
    ///
    /// Returns a tuple of (decoded_text, bytes_buffered)
    fn decode_streaming(&mut self, py: Python<'_>, tokens: Vec<u32>) -> (String, usize) {
        let decoder = &self.decoder;
        let base_tokens = py.detach(move || decoder.decode(&tokens));
        let new_bytes: Vec<u8> = base_tokens.iter().map(|&t| (t & 0xFF) as u8).collect();

        // Combine partial bytes from previous call with new bytes
        self.partial_bytes.extend(new_bytes);

        // Find the last valid UTF-8 boundary
        let mut valid_end = 0;
        let mut i = 0;
        while i < self.partial_bytes.len() {
            let byte = self.partial_bytes[i];
            let char_len = if byte < 0x80 {
                1 // ASCII
            } else if byte < 0xE0 {
                2 // 2-byte sequence
            } else if byte < 0xF0 {
                3 // 3-byte sequence
            } else {
                4 // 4-byte sequence
            };

            if i + char_len <= self.partial_bytes.len() {
                // Check if we have all bytes for this character
                let mut is_valid = true;
                for j in 1..char_len {
                    if i + j >= self.partial_bytes.len()
                        || (self.partial_bytes[i + j] & 0xC0) != 0x80
                    {
                        is_valid = false;
                        break;
                    }
                }
                if is_valid {
                    valid_end = i + char_len;
                    i += char_len;
                } else {
                    // Invalid sequence, skip first byte
                    valid_end = i + 1;
                    i += 1;
                }
            } else {
                // Incomplete sequence at end
                break;
            }
        }

        // Extract complete characters
        let complete: Vec<u8> = self.partial_bytes.drain(..valid_end).collect();
        let result = String::from_utf8_lossy(&complete).into_owned();
        let buffered = self.partial_bytes.len();

        (result, buffered)
    }

    /// Flush any remaining buffered bytes
    ///
    /// Call this when the token stream is complete to get any remaining
    /// partial UTF-8 sequences (which will be lossy-decoded).
    fn flush(&mut self) -> String {
        let bytes = std::mem::take(&mut self.partial_bytes);
        String::from_utf8_lossy(&bytes).into_owned()
    }

    /// Reset the streaming decoder state
    fn reset(&mut self) {
        self.partial_bytes.clear();
    }
}

/// BPE tokenizer combining encoder and decoder with streaming support
#[pyclass]
pub struct Utf8BpeTokenizer {
    encoder: Utf8BpeEncoder,
    decoder: Utf8BpeDecoder,
    vocab_size: u32,
}

#[pymethods]
impl Utf8BpeTokenizer {
    /// Create a new UTF-8 BPE tokenizer from a merge table file
    #[new]
    fn new(merge_table_path: &str) -> PyResult<Self> {
        let encoder = Utf8BpeEncoder::new(merge_table_path)?;
        let decoder = Utf8BpeDecoder::new(merge_table_path)?;

        // Get vocab size from merge table
        let table = BpeMergeTable::from_file(merge_table_path).map_err(|e| {
            pyo3::exceptions::PyIOError::new_err(format!("Failed to load merge table: {}", e))
        })?;
        let vocab_size = table.bpe_start() + table.merges.len() as u32;

        Ok(Self {
            encoder,
            decoder,
            vocab_size,
        })
    }

    /// Encode a UTF-8 string to BPE tokens
    fn encode(&self, py: Python<'_>, text: &str) -> Vec<u32> {
        self.encoder.encode(py, text)
    }

    /// Decode BPE tokens to a UTF-8 string
    fn decode(&self, py: Python<'_>, tokens: Vec<u32>) -> String {
        self.decoder.decode(py, tokens)
    }

    /// Streaming decode: decode tokens and return complete UTF-8 characters
    fn decode_streaming(&mut self, py: Python<'_>, tokens: Vec<u32>) -> (String, usize) {
        self.decoder.decode_streaming(py, tokens)
    }

    /// Flush any remaining buffered bytes from streaming decode
    fn flush(&mut self) -> String {
        self.decoder.flush()
    }

    /// Reset the streaming decoder state
    fn reset(&mut self) {
        self.decoder.reset();
    }

    /// Get the vocabulary size (base + BPE tokens)
    fn vocab_size(&self) -> u32 {
        self.vocab_size
    }

    /// Get the base vocabulary size (256 for UTF-8 byte encoding)
    fn base_vocab_size(&self) -> u32 {
        self.encoder.base_vocab_size()
    }
}

mod compile_api;
#[cfg(feature = "experimental")]
mod sql_runtime;

/// Python module definition
#[pymodule]
fn tct(m: &Bound<'_, PyModule>) -> PyResult<()> {
    compile_api::register(m)?;
    m.add_class::<Utf8BpeEncoder>()?;
    m.add_class::<Utf8BpeDecoder>()?;
    m.add_class::<Utf8BpeTokenizer>()?;
    #[cfg(feature = "experimental")]
    sql_runtime::register(m)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bpe::{BpeMerge, BpeMetadata};
    use std::sync::Once;

    pub(super) fn with_python<T>(f: impl for<'py> FnOnce(Python<'py>) -> T) -> T {
        static INIT: Once = Once::new();
        INIT.call_once(Python::initialize);
        Python::attach(f)
    }

    fn table(schema: &str, merges: Vec<BpeMerge>) -> BpeMergeTable {
        BpeMergeTable {
            schema: schema.to_string(),
            base_vocab_size: 256,
            metadata: BpeMetadata {
                trained_on_instances: 1,
                training_date: "2026-04-30".to_string(),
                max_merges: merges.len(),
                dict_size: None,
            },
            merges,
        }
    }

    #[test]
    fn utf8_bpe_encoder_uses_sequential_no_revisit_semantics() {
        // This deliberately bypasses file validation so the PyO3 wrapper path
        // exercises the same no-revisit merge semantics as the Rust encoder.
        let table = table(
            "utf8-no-revisit",
            vec![
                BpeMerge {
                    pair: [256, 5],
                    new_token: 257,
                    frequency: 2,
                },
                BpeMerge {
                    pair: [1, 2],
                    new_token: 256,
                    frequency: 1,
                },
            ],
        );
        let encoder = Utf8BpeEncoder {
            encoder: SequentialBpeEncoder::new(&table, 258),
            base_vocab_size: table.base_vocab_size,
        };

        with_python(|py| {
            assert_eq!(encoder.encode_bytes(py, &[1, 2, 5]), vec![256, 5]);
        });
    }

    #[test]
    fn utf8_bpe_encoder_constructor_matches_sequential_encoder_for_valid_table() {
        let table = table(
            "utf8-valid",
            vec![
                BpeMerge {
                    pair: [97, 97],
                    new_token: 256,
                    frequency: 3,
                },
                BpeMerge {
                    pair: [256, 98],
                    new_token: 257,
                    frequency: 2,
                },
            ],
        );
        let expected = SequentialBpeEncoder::new(&table, 258).encode(&[97, 97, 98, 97]);
        let file = tempfile::NamedTempFile::new().unwrap();
        std::fs::write(file.path(), serde_json::to_vec(&table).unwrap()).unwrap();
        let encoder = Utf8BpeEncoder::new(file.path().to_str().unwrap()).unwrap();
        let tokenizer = Utf8BpeTokenizer::new(file.path().to_str().unwrap()).unwrap();

        with_python(|py| {
            assert_eq!(encoder.encode(py, "aaba"), expected);
            assert_eq!(encoder.base_vocab_size(), 256);
            assert_eq!(tokenizer.encode(py, "aaba"), expected);
            assert_eq!(tokenizer.decode(py, expected), "aaba");
        });
    }
}
