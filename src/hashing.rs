//! Hashing utilities: BLAKE3-based quick and full hashes.

use anyhow::{Context, Result};
use blake3::Hasher;
use std::fs::File;
use std::io::Read;
use std::path::Path;

/// Hash the first `limit` bytes of a file with BLAKE3.
/// If file is smaller than `limit`, hashes the whole file.
pub fn quick_hash(path: &Path, limit: u64) -> Result<String> {
    let mut f = File::open(path).with_context(|| format!("open {}", path.display()))?;
    let mut hasher = Hasher::new();
    let mut buf = vec![0u8; 64 * 1024]; // 64KiB buffer
    let mut left = limit;

    while left > 0 {
        let to_read = buf.len().min(left as usize);
        let got = f.read(&mut buf[..to_read])?;
        if got == 0 {
            break;
        }
        hasher.update(&buf[..got]);
        left -= got as u64;
    }

    Ok(hasher.finalize().to_hex().to_string())
}

/// Hash the entire file with BLAKE3 (streaming, fixed buffer).
pub fn full_hash(path: &Path) -> Result<String> {
    let mut f = File::open(path).with_context(|| format!("open {}", path.display()))?;
    let mut hasher = Hasher::new();
    let mut buf = vec![0u8; 1024 * 1024]; // 1 MiB buffer
    loop {
        let n = f.read(&mut buf)?;
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
    }
    Ok(hasher.finalize().to_hex().to_string())
}
