use crate::cache;
use anyhow::{Context, Result};
use std::path::{Path, PathBuf};
use std::process::Command;

/// Ensure a tool is compiled, returning path to WASM
pub fn ensure_compiled(project_root: &Path, source: &Path) -> Result<PathBuf> {
    // Check cache first
    if let Some(cached) = cache::is_cached(project_root, source)? {
        return Ok(cached);
    }

    // Compile to WASI
    let hash = cache::hash_source(source)?;
    let output = cache::cache_path(project_root, &hash);

    compile_to_wasi(source, &output)?;

    Ok(output)
}

/// Compile a cargo script to WASI
fn compile_to_wasi(source: &Path, output: &Path) -> Result<()> {
    // Ensure cache directory exists
    if let Some(parent) = output.parent() {
        std::fs::create_dir_all(parent)?;
    }

    // Create a temporary directory for the build
    let temp_dir = tempfile::tempdir()?;
    let temp_target = temp_dir.path().join("target");

    // Get the source file name for the binary name
    let bin_name = source
        .file_stem()
        .and_then(|s| s.to_str())
        .context("Invalid source filename")?;

    // Run cargo with script mode targeting WASI
    // Note: cargo script with -Zscript compiles and runs, so we need to use
    // a different approach - extract the manifest and build normally
    let status = Command::new("cargo")
        .args([
            "+nightly",
            "-Zscript",
            source.to_str().unwrap(),
            "--",
            "--help", // Just to make it compile without actually running
        ])
        .env("CARGO_TARGET_DIR", &temp_target)
        .env("CARGO_BUILD_TARGET", "wasm32-wasip1")
        .output()
        .context("Failed to run cargo")?;

    // If the above doesn't work for WASI target, we need an alternative approach
    // For now, let's try direct rustc compilation as a fallback
    if !status.status.success() {
        // Alternative: use rustc directly for simple scripts
        // This is a simplified approach for the initial implementation
        let rustc_output = Command::new("rustc")
            .args([
                "+nightly",
                "--target",
                "wasm32-wasip1",
                "-O",
                "-o",
                output.to_str().unwrap(),
                source.to_str().unwrap(),
            ])
            .output()
            .context("Failed to run rustc")?;

        if !rustc_output.status.success() {
            let stderr = String::from_utf8_lossy(&rustc_output.stderr);
            anyhow::bail!("Compilation failed: {}", stderr);
        }
    }

    // Find the compiled WASM file if using cargo
    let wasm_source = temp_target
        .join("wasm32-wasip1")
        .join("release")
        .join(format!("{}.wasm", bin_name));

    if wasm_source.exists() {
        std::fs::copy(&wasm_source, output)?;
    } else if !output.exists() {
        anyhow::bail!(
            "Compiled WASM not found. Tried: {} and direct rustc output at {}",
            wasm_source.display(),
            output.display()
        );
    }

    Ok(())
}
