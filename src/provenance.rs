//! Reproducibility metadata helpers for generated data artifacts.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fs::File;
use std::io::{self, BufReader, Read};
use std::path::Path;

/// Stable identity metadata for an input or output file.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct FileProvenance {
    /// Path as provided by the calling workflow.
    pub path: String,
    /// File size in bytes.
    pub bytes: u64,
    /// SHA-256 digest of the file contents, lowercase hex.
    pub sha256: String,
}

/// Compute file size and SHA-256 content hash for provenance reports.
pub fn file_provenance(path: impl AsRef<Path>) -> io::Result<FileProvenance> {
    let path = path.as_ref();
    let bytes = path.metadata()?.len();
    let mut reader = BufReader::new(File::open(path)?);
    let mut hasher = Sha256::new();
    let mut buffer = [0_u8; 8192];

    loop {
        let read = reader.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
    }

    let digest = hasher.finalize();

    Ok(FileProvenance {
        path: path.display().to_string(),
        bytes,
        sha256: hex_lower(digest.as_ref()),
    })
}

/// Compute a lowercase SHA-256 digest for an in-memory byte slice.
pub fn sha256_hex(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    let digest = hasher.finalize();
    hex_lower(digest.as_ref())
}

/// Compute a SHA-256 digest over serde_json's stable serialization for a value.
///
/// This is intended for structs and ordered metadata payloads. Avoid using it
/// as a canonical hash for unordered maps unless the caller controls key order.
pub fn json_sha256(value: &impl Serialize) -> Result<String, serde_json::Error> {
    let bytes = serde_json::to_vec(value)?;
    Ok(sha256_hex(&bytes))
}

pub(crate) fn hex_lower(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut output = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        output.push(HEX[(byte >> 4) as usize] as char);
        output.push(HEX[(byte & 0x0f) as usize] as char);
    }
    output
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn file_provenance_reports_stable_sha256() {
        let dir = tempfile::TempDir::new().unwrap();
        let path = dir.path().join("payload.txt");
        std::fs::write(&path, b"abc").unwrap();

        let provenance = file_provenance(&path).unwrap();

        assert_eq!(provenance.path, path.display().to_string());
        assert_eq!(provenance.bytes, 3);
        assert_eq!(
            provenance.sha256,
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
    }

    #[test]
    fn json_sha256_reports_stable_serialization_hash() {
        #[derive(Serialize)]
        struct Sample<'a> {
            version: u32,
            name: &'a str,
        }

        let sample = Sample {
            version: 1,
            name: "tct",
        };

        assert_eq!(
            json_sha256(&sample).unwrap(),
            sha256_hex(br#"{"version":1,"name":"tct"}"#)
        );
    }
}
