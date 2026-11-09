//! Content-Addressable Storage (CAS) for candidate code artifacts.
//!
//! Uses BLAKE3 for cryptographic hashing and Zstd for high-ratio source code
//! compression inside SQLite BLOB columns.

use std::io::{Read, Write};

/// Maximum decompressed source artifact size (4 MiB) to guard against hostile inputs.
pub const MAX_CODE_BYTES: usize = 4 * 1024 * 1024;

/// Compute the 64-character hex BLAKE3 digest of a source string.
#[must_use]
pub fn hash_code(code: &str) -> String {
    blake3::hash(code.as_bytes()).to_hex().to_string()
}

/// Compress source code with Zstd (level 3).
pub fn compress_code(code: &str) -> Result<Vec<u8>, String> {
    if code.len() > MAX_CODE_BYTES {
        return Err(format!(
            "source code exceeds maximum size ({} > {MAX_CODE_BYTES} bytes)",
            code.len()
        ));
    }
    let mut encoder =
        zstd::stream::write::Encoder::new(Vec::new(), 3).map_err(|e| format!("zstd init: {e}"))?;
    encoder
        .write_all(code.as_bytes())
        .map_err(|e| format!("zstd write: {e}"))?;
    encoder.finish().map_err(|e| format!("zstd finish: {e}"))
}

/// Decompress a Zstd-encoded source code blob with an upper-bound fence.
pub fn decompress_code(blob: &[u8]) -> Result<String, String> {
    let decoder = zstd::stream::read::Decoder::new(blob).map_err(|e| format!("zstd init: {e}"))?;
    let mut bounded = decoder.take((MAX_CODE_BYTES as u64).saturating_add(1));
    let mut out = Vec::new();
    bounded
        .read_to_end(&mut out)
        .map_err(|e| format!("zstd read: {e}"))?;
    if out.len() > MAX_CODE_BYTES {
        return Err(format!(
            "decompressed code exceeds {MAX_CODE_BYTES} byte limit"
        ));
    }
    String::from_utf8(out).map_err(|e| format!("invalid utf-8 in compressed code: {e}"))
}
