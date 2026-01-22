use std::path::{Path, PathBuf};

use anyhow::Result;
use sha1::{Digest, Sha1};

/// Compute git blob hash: SHA1("blob <size>\0<content>").
#[must_use]
pub fn git_blob_hash(content: &[u8]) -> String {
    let header = format!("blob {}\0", content.len());
    let mut hasher = Sha1::new();
    hasher.update(header.as_bytes());
    hasher.update(content);
    hex::encode(hasher.finalize())
}

/// Get cache directory (project-local).
#[must_use]
pub fn cache_dir(project_root: &Path) -> PathBuf {
    project_root.join(".opencode/cache/wasi")
}

/// Get cached WASM path for a given hash.
#[must_use]
pub fn cache_path(project_root: &Path, hash: &str) -> PathBuf {
    cache_dir(project_root).join(format!("{hash}.wasm"))
}

/// Check if cache is valid for the given source file.
///
/// # Errors
/// Returns an error if the source file cannot be read.
pub fn is_cached(project_root: &Path, source: &Path) -> Result<Option<PathBuf>> {
    let content = std::fs::read(source)?;
    let hash = git_blob_hash(&content);
    let cached = cache_path(project_root, &hash);

    if cached.exists() {
        Ok(Some(cached))
    } else {
        Ok(None)
    }
}

/// Get the hash for a source file.
///
/// # Errors
/// Returns an error if the source file cannot be read.
pub fn hash_source(source: &Path) -> Result<String> {
    let content = std::fs::read(source)?;
    Ok(git_blob_hash(&content))
}
