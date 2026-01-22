use crate::cache;
use anyhow::{Context, Result};
use std::path::{Path, PathBuf};
use std::process::Command;

/// Parsed cargo frontmatter from a script
struct Frontmatter {
    manifest: String,
    code: String,
}

/// Parse cargo frontmatter from source file
/// Format:
/// ```
/// #!/usr/bin/env -S cargo +nightly -Zscript
/// ---cargo
/// [package]
/// name = "example"
/// ...
/// ---
/// <rust code>
/// ```
fn parse_frontmatter(source: &str) -> Result<Frontmatter> {
    let lines: Vec<&str> = source.lines().collect();

    // Find the start marker (---cargo)
    let start = lines
        .iter()
        .position(|line| line.trim() == "---cargo")
        .context("No ---cargo frontmatter found")?;

    // Find the end marker (---)
    let end = lines
        .iter()
        .skip(start + 1)
        .position(|line| line.trim() == "---")
        .context("No closing --- for frontmatter")?
        + start
        + 1;

    // Extract manifest (between markers)
    let manifest = lines[start + 1..end].join("\n");

    // Extract code (after closing ---)
    let code = lines[end + 1..].join("\n");

    Ok(Frontmatter { manifest, code })
}

/// Ensure a tool is compiled, returning path to WASM
pub fn ensure_compiled(project_root: &Path, source: &Path) -> Result<PathBuf> {
    // Check cache first
    if let Some(cached) = cache::is_cached(project_root, source)? {
        return Ok(cached);
    }

    // Compile to WASI
    let hash = cache::hash_source(source)?;
    let output = cache::cache_path(project_root, &hash);

    compile_to_wasi(project_root, source, &output)?;

    Ok(output)
}

/// Compile a cargo script to WASI
fn compile_to_wasi(project_root: &Path, source: &Path, output: &Path) -> Result<()> {
    // Ensure cache directory exists
    if let Some(parent) = output.parent() {
        std::fs::create_dir_all(parent)?;
    }

    // Read and parse the source file
    let content = std::fs::read_to_string(source).context("Failed to read source file")?;
    let frontmatter = parse_frontmatter(&content)?;

    // Create a temporary directory for the build
    let temp_dir = tempfile::tempdir()?;
    let build_dir = temp_dir.path();

    // Create the tool as a workspace member
    // Read workspace Cargo.toml and modify members to point to tool
    let workspace_toml = std::fs::read_to_string(project_root.join("Cargo.toml"))
        .context("Failed to read workspace Cargo.toml")?;

    // Replace the members line to only include "tool"
    let modified_workspace = workspace_toml
        .lines()
        .map(|line| {
            if line.starts_with("members") {
                "members = [\"tool\"]"
            } else {
                line
            }
        })
        .collect::<Vec<_>>()
        .join("\n");

    // Write workspace Cargo.toml
    std::fs::write(build_dir.join("Cargo.toml"), &modified_workspace)?;

    // Create tool directory
    let tool_dir = build_dir.join("tool");
    std::fs::create_dir_all(&tool_dir)?;

    // Write tool's Cargo.toml
    std::fs::write(tool_dir.join("Cargo.toml"), &frontmatter.manifest)?;

    // Create src directory and write main.rs
    let src_dir = tool_dir.join("src");
    std::fs::create_dir_all(&src_dir)?;
    std::fs::write(src_dir.join("main.rs"), &frontmatter.code)?;

    // Symlink the workspace crates so workspace dependencies resolve
    let crates_src = project_root.join("crates");
    let crates_dst = build_dir.join("crates");
    std::os::unix::fs::symlink(&crates_src, &crates_dst)
        .context("Failed to symlink crates directory")?;

    // Get the binary name from manifest
    let bin_name = frontmatter
        .manifest
        .lines()
        .find(|line| line.trim().starts_with("name = "))
        .and_then(|line| {
            let start = line.find('"')? + 1;
            let end = line.rfind('"')?;
            Some(&line[start..end])
        })
        .unwrap_or("tool");

    // Build with cargo targeting WASI
    let cargo_output = Command::new("cargo")
        .args([
            "+nightly",
            "build",
            "--release",
            "--target",
            "wasm32-wasip1",
            "-p",
            bin_name,
        ])
        .current_dir(build_dir)
        .output()
        .context("Failed to run cargo build")?;

    if !cargo_output.status.success() {
        let stderr = String::from_utf8_lossy(&cargo_output.stderr);
        anyhow::bail!("Compilation failed: {}", stderr);
    }

    // Copy the compiled WASM to cache
    let wasm_source = build_dir
        .join("target")
        .join("wasm32-wasip1")
        .join("release")
        .join(format!("{}.wasm", bin_name));

    if !wasm_source.exists() {
        anyhow::bail!("Compiled WASM not found at {}", wasm_source.display());
    }

    std::fs::copy(&wasm_source, output)?;

    Ok(())
}
